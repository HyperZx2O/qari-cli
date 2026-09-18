use arabic_reshaper::ArabicReshaper;
use std::sync::OnceLock;

pub fn terminal_lines(text: &str, max_width: usize) -> Vec<String> {
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
        .iter()
        .map(|line| terminal_line(line))
        .collect()
}

fn terminal_line(text: &str) -> String {
    reshaper()
        .reshape(text)
        .chars()
        .filter(|character| !is_arabic_combining_mark(*character))
        .rev()
        .collect()
}

fn logical_width(text: &str) -> usize {
    text.chars()
        .filter(|character| !is_arabic_combining_mark(*character))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_and_reorders_arabic_for_ltr_terminal_cells() {
        let lines = terminal_lines("السَّلَامُ عَلَيْكُمْ", 40);
        assert_eq!(lines.len(), 1);
        assert!(!lines[0].contains('َ'));
        assert!(lines[0]
            .chars()
            .any(|character| { matches!(character as u32, 0xFB50..=0xFDFF | 0xFE70..=0xFEFF) }));
    }

    #[test]
    fn wraps_before_visual_reordering() {
        let lines = terminal_lines("بسم الله الرحمن الرحيم", 8);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.chars().count() <= 8));
        assert_eq!(lines[0], terminal_line("بسم الله"));
    }
}
