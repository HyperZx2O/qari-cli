//! Grapheme-cluster-safe text wrapping and truncation for the Scripture
//! panel.
//!
//! Arabic, Hebrew and polytonic Greek render a syllable from a base letter
//! plus combining marks (tashkil, niqqud, breathings, accents). Splitting
//! one of those sequences across two terminal rows breaks the shaper and
//! renders dotted-circle placeholders, so every break here happens on
//! whitespace or — only for words longer than the panel — on extended
//! grapheme-cluster boundaries, never inside a cluster.
//!
//! Widths come from `unicode-width`, the same crate ratatui uses for cell
//! placement, so wrapped rows line up with the rendered buffer.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

fn word_width(word: &str) -> usize {
    word.graphemes(true).map(UnicodeWidthStr::width).sum()
}

/// Wrap `text` to rows of at most `max_width` display columns.
/// Breaks only on whitespace; a single over-long word is broken on
/// grapheme boundaries. Never splits a grapheme cluster.
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let max_width = max_width.max(1);
    let mut rows = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    let push_word = |rows: &mut Vec<String>,
                     current: &mut String,
                     current_width: &mut usize,
                     word: &str,
                     word_width: usize| {
        if current.is_empty() {
            current.push_str(word);
            *current_width = word_width;
        } else if *current_width + 1 + word_width <= max_width {
            current.push(' ');
            current.push_str(word);
            *current_width += 1 + word_width;
        } else {
            rows.push(std::mem::take(current));
            current.push_str(word);
            *current_width = word_width;
        }
    };

    for word in text.split_whitespace() {
        let width = word_width(word);
        if width <= max_width {
            push_word(&mut rows, &mut current, &mut current_width, word, width);
            continue;
        }
        // Over-long word: emit the pending row, then break the word itself
        // on grapheme boundaries.
        if !current.is_empty() {
            rows.push(std::mem::take(&mut current));
            current_width = 0;
        }
        let mut fragment = String::new();
        let mut fragment_width = 0usize;
        for grapheme in word.graphemes(true) {
            let cluster_width = UnicodeWidthStr::width(grapheme);
            if !fragment.is_empty() && fragment_width + cluster_width > max_width {
                rows.push(std::mem::take(&mut fragment));
                fragment_width = 0;
            }
            fragment.push_str(grapheme);
            fragment_width += cluster_width;
        }
        if !fragment.is_empty() {
            current = fragment;
            current_width = fragment_width;
        }
    }

    if !current.is_empty() {
        rows.push(current);
    }
    rows
}

/// Collapse whitespace and shorten to `max_chars` graphemes, appending `…`
/// when truncated. Never splits a grapheme cluster, so a combining mark
/// never lands alone on the next row.
pub fn truncate_clusters(text: &str, max_chars: usize) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let total: usize = one_line.graphemes(true).count();
    if total <= max_chars {
        return one_line;
    }
    let mut result: String = one_line
        .graphemes(true)
        .take(max_chars.saturating_sub(1))
        .collect();
    result.push('…');
    result
}

/// True for combining marks that must never start a rendered row on their
/// own: the Greek/Latin, Hebrew and Arabic mark ranges this app renders.
/// Test-only assertion helper: production code keeps clusters intact by
/// construction (breaks happen on whitespace or grapheme boundaries).
#[cfg(test)]
pub fn is_combining_mark(character: char) -> bool {
    matches!(
        character as u32,
        0x0300..=0x036F
            | 0x0591..=0x05BD
            | 0x05BF
            | 0x05C1..=0x05C2
            | 0x05C4..=0x05C5
            | 0x05C7
            | 0x0610..=0x061A
            | 0x064B..=0x065F
            | 0x0670
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Al-Fatihah 1:1 (Uthmani): tashkil and the superscript alef ride
    /// their base letters.
    const AR_1_1: &str = "بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ";

    fn measure(row: &str) -> usize {
        row.graphemes(true).map(UnicodeWidthStr::width).sum()
    }

    #[test]
    fn wrap_never_orphans_combining_marks() {
        for width in 1..=60 {
            for row in wrap_text(AR_1_1, width) {
                let first = row.chars().next().expect("row must not be empty");
                assert!(
                    !is_combining_mark(first),
                    "width {width}: row starts with orphaned mark {first:?}: {row:?}"
                );
            }
        }
    }

    #[test]
    fn wrap_preserves_every_grapheme() {
        for width in [1, 5, 8, 13, 27, 40, 200] {
            let rows = wrap_text(AR_1_1, width);
            let rebuilt: String = rows
                .iter()
                .flat_map(|row| row.graphemes(true))
                .filter(|grapheme| *grapheme != " ")
                .collect::<Vec<_>>()
                .concat();
            let expected: String = AR_1_1
                .split_whitespace()
                .flat_map(|word| word.graphemes(true))
                .collect::<Vec<_>>()
                .concat();
            assert_eq!(rebuilt, expected, "graphemes lost or split at width {width}");
        }
    }

    #[test]
    fn wrap_respects_width() {
        for width in [8, 20, 40] {
            for row in wrap_text(AR_1_1, width) {
                let measured = measure(&row);
                assert!(measured <= width, "row exceeds width {width}: {row:?}");
            }
        }
    }

    #[test]
    fn truncate_keeps_clusters_intact() {
        let shortened = truncate_clusters("Ἐν ἀρχῇ ἦν ὁ Λόγος", 6);
        assert!(shortened.ends_with('…'));
        assert_eq!(shortened.graphemes(true).count(), 6);
        for grapheme in shortened.graphemes(true) {
            let first = grapheme.chars().next().unwrap();
            assert!(!is_combining_mark(first), "split cluster: {grapheme:?}");
        }
    }

    #[test]
    fn truncate_short_text_unchanged() {
        assert_eq!(truncate_clusters("mercy", 10), "mercy");
    }

    #[test]
    fn polytonic_greek_keeps_combining_marks_attached() {
        // "Ἐν ἀρχῇ": breathing marks and accents must never start a row.
        let text = "Ἐν ἀρχῇ ἦν ὁ Λόγος";
        for width in [4, 9, 40] {
            for row in wrap_text(text, width) {
                let first = row.chars().next().expect("row must not be empty");
                assert!(
                    !is_combining_mark(first),
                    "width {width}: row starts with orphaned mark {first:?}: {row:?}"
                );
            }
        }
        let rebuilt: String = wrap_text(text, 9)
            .concat()
            .graphemes(true)
            .filter(|grapheme| *grapheme != " ")
            .collect();
        let expected: String = text
            .split_whitespace()
            .flat_map(|word| word.graphemes(true))
            .collect::<Vec<_>>()
            .concat();
        assert_eq!(rebuilt, expected);
    }

    #[test]
    fn overlong_word_breaks_only_on_cluster_edges() {
        // One long unspaced Arabic word: must still fit narrow panels
        // without ever splitting a letter from its marks.
        let word = "ٱلرَّحْمَـٰنِ".repeat(40);
        for width in [10, 27, 40] {
            let rows = wrap_text(&word, width);
            assert!(rows.len() > 1, "expected wrapping at width {width}");
            for row in &rows {
                assert!(measure(row) <= width, "row exceeds {width}: {row:?}");
                let first = row.chars().next().expect("row must not be empty");
                assert!(!is_combining_mark(first), "orphaned mark: {row:?}");
            }
            assert_eq!(rows.concat(), word, "clusters lost at width {width}");
        }
    }

    #[test]
    fn empty_and_blank_inputs_yield_no_rows() {
        assert!(wrap_text("", 40).is_empty());
        assert!(wrap_text("   ", 40).is_empty());
        assert_eq!(truncate_clusters("", 5), "");
    }
}
