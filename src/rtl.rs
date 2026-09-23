use arabic_reshaper::ArabicReshaper;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtlMode {
    Logical,
    Visual,
}

impl RtlMode {
    /// `logical`/`visual` from config; otherwise logical on VTE terminals,
    /// which do their own BiDi shaping, and visual everywhere else.
    pub fn detect(configured: &str) -> Self {
        match configured.to_ascii_lowercase().as_str() {
            "logical" => Self::Logical,
            "visual" => Self::Visual,
            _ if std::env::var_os("VTE_VERSION").is_some() => Self::Logical,
            _ => Self::Visual,
        }
    }
}

pub fn terminal_lines(text: &str, max_width: usize, mode: RtlMode) -> Vec<String> {
    // Greek (and any other left-to-right script) needs wrapping only:
    // reversing it would print it backwards.
    if !is_rtl(text) {
        return crate::wrap::wrap_text(text, max_width);
    }
    let logical_lines = logical_lines(text, max_width);
    match mode {
        RtlMode::Logical => logical_lines,
        RtlMode::Visual => logical_lines
            .iter()
            .map(|line| terminal_line(line))
            .collect(),
    }
}

fn is_rtl(text: &str) -> bool {
    text.chars().any(|character| {
        matches!(
            character as u32,
            0x0590..=0x05FF | 0x0600..=0x06FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF
        )
    })
}

fn logical_lines(text: &str, max_width: usize) -> Vec<String> {
    let max_width = max_width.max(1);
    let mut logical_lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };

        if !current.is_empty() && logical_width(&candidate) > max_width {
            logical_lines.push(std::mem::take(&mut current));
            current.push_str(word);
        } else {
            current = candidate;
        }
    }

    if !current.is_empty() {
        logical_lines.push(current);
    }

    logical_lines
}

fn terminal_line(text: &str) -> String {
    if is_hebrew(text) {
        // Hebrew letters don't join, so no reshaping: only bidi reorder.
        // Niqqud and cantillation are stripped like Arabic tashkil — terminals
        // shape pointed Hebrew unreliably, and dotted circles read worse
        // than clean consonantal text.
        return text
            .chars()
            .filter(|character| !is_hebrew_combining_mark(*character))
            .rev()
            .collect();
    }
    reshaper()
        .reshape(text)
        .chars()
        .filter(|character| !is_arabic_combining_mark(*character))
        .rev()
        .collect()
}

fn is_hebrew(text: &str) -> bool {
    text.chars()
        .any(|character| matches!(character as u32, 0x0590..=0x05FF))
}

fn logical_width(text: &str) -> usize {
    text.chars()
        .filter(|character| {
            !is_arabic_combining_mark(*character) && !is_hebrew_combining_mark(*character)
        })
        .count()
}

fn reshaper() -> &'static ArabicReshaper<'static> {
    static RESHAPER: OnceLock<ArabicReshaper<'static>> = OnceLock::new();
    RESHAPER.get_or_init(ArabicReshaper::new)
}

fn is_arabic_combining_mark(character: char) -> bool {
    matches!(
        character as u32,
        0x0610..=0x061A
            | 0x064B..=0x065F
            | 0x0670
            | 0x06D6..=0x06DC
            | 0x06DF..=0x06E4
            | 0x06E7..=0x06E8
            | 0x06EA..=0x06ED
            | 0x08D3..=0x08FF
    )
}

fn is_hebrew_combining_mark(character: char) -> bool {
    matches!(
        character as u32,
        0x0591..=0x05BD | 0x05BF | 0x05C1..=0x05C2 | 0x05C4..=0x05C5 | 0x05C7
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_and_reorders_arabic_for_ltr_terminal_cells() {
        let lines = terminal_lines("السَّلَامُ عَلَيْكُمْ", 40, RtlMode::Visual);
        assert_eq!(lines.len(), 1);
        assert!(!lines[0].contains('َ'));
        assert!(lines[0]
            .chars()
            .any(|character| { matches!(character as u32, 0xFB50..=0xFDFF | 0xFE70..=0xFEFF) }));
    }

    #[test]
    fn wraps_before_visual_reordering() {
        let lines = terminal_lines("بسم الله الرحمن الرحيم", 8, RtlMode::Visual);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.chars().count() <= 8));
        assert_eq!(lines[0], terminal_line("بسم الله"));
    }

    #[test]
    fn logical_mode_preserves_uthmani_marks() {
        let lines = terminal_lines("السَّلَامُ عَلَيْكُمْ", 40, RtlMode::Logical);
        assert!(lines[0].contains('َ'));
    }

    #[test]
    fn hebrew_reorders_without_marks() {
        // Genesis 1:1 pointed: niqqud + cantillation strip, order reverses.
        let lines = terminal_lines("בְּרֵאשִׁ֖ית", 40, RtlMode::Visual);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "תישארב");
        assert!(!lines[0]
            .chars()
            .any(|character| { matches!(character as u32, 0x0591..=0x05C7) }));
    }

    #[test]
    fn greek_never_reverses() {
        let lines = terminal_lines("Ἐν ἀρχῇ", 40, RtlMode::Visual);
        assert_eq!(lines, ["Ἐν ἀρχῇ"]);
    }
}
