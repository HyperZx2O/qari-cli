//! Helpers for handling untrusted text before it reaches a terminal.

/// Remove terminal control characters while preserving ordinary line breaks
/// and tabs. In particular, this prevents ESC, OSC, and other C0/C1 controls
/// from being interpreted by a user's terminal.
pub fn sanitize_terminal_text(text: &str) -> String {
    text.chars()
        .filter(|&character| {
            matches!(character, '\n' | '\t')
                || (!character.is_control() && !matches!(character as u32, 0x80..=0x9f))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::sanitize_terminal_text;

    #[test]
    fn removes_ansi_and_terminal_control_sequences() {
        let sanitized = sanitize_terminal_text("before\x1b[31mred\x1b]52;c;secret\x07after");
        assert_eq!(sanitized, "before[31mred]52;c;secretafter");
    }

    #[test]
    fn preserves_newlines_and_tabs() {
        assert_eq!(sanitize_terminal_text("a\n\tb"), "a\n\tb");
    }
}
