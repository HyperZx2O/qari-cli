use crate::theme::ThemeColors;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthStr;

// Hallmark pre-flight (terminal adaptation):
// - Tokens: ThemeColors only (theme.rs) — no hardcoded colors below.
// - Type: terminal monospace only. Art is pure ASCII + single-line box
//   drawing; never BOLD (faux-bold overstrikes block glyphs in VS Code /
//   Windows Terminal and ghosts the logo).
// - Motion: one authored moment (mark -> wordmark -> typed tagline),
//   first-launch only, any-key skip, reduced_motion bypass.
// Impeccable mode: Onboard (first-run loading flow).

/// Open Quran resting on a rehal stand, pure ASCII (no box drawing, so it
/// survives every terminal font and codepage). Rows use relative indents
/// only (widest rows start at column 0); `render_art` centers the whole
/// block, so editors stripping trailing whitespace can never break the
/// symmetry. Verse lines vary in length like real ayahs.
const MARK_ART: &[&str] = &[
    "   ______       ______   ",
    "  | ~~~~    |    ~~~~ |  ",
    "  | ~~~~~   |   ~~~~~ |  ",
    "  | ~~~     |     ~~~ |  ",
    "  _____________________  ",
    "        \\       /        ",
    "          \\   /          ",
    "            X            ",
    "          /   \\          ",
];

/// "QARI" wordmark in FIGlet Standard (pure ASCII, generated with
/// `figlet QARI` — never hand-drawn). Standard is the most portable splash
/// font: no block or double-line glyphs, so it survives faux-bold,
/// narrow fonts, and legacy codepages. Rows are padded to equal width
/// at render time.
const TITLE_ART: &[&str] = &[
    " ___      _    ____  ___ ",
    " / _ \\    / \\  |  _ \\|_ _|",
    " | | | |  / _ \\ | |_) || | ",
    " | |_| | / ___ \\|  _ < | | ",
    "  \\__\\_\\/_/   \\_\\_| \\_\\___|",
];

const INTRO_END_TICK: u32 = 150;

const TAGLINE: &str = "Ilm at your fingertips";

pub struct IntroState {
    tick: u32,
    skipped: bool,
    animated: bool,
    started: Instant,
}

impl IntroState {
    pub fn new(animated: bool) -> Self {
        Self {
            tick: if animated { 0 } else { INTRO_END_TICK + 1 },
            skipped: false,
            animated,
            started: Instant::now(),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.saturating_add(1);
    }

    pub fn skip(&mut self) {
        if self.animated {
            self.skipped = true;
        }
    }

    pub fn can_enter_reader(&self, data_ready: bool) -> bool {
        data_ready && (self.skipped || self.tick > INTRO_END_TICK)
    }

    fn phase(&self) -> u8 {
        match self.tick {
            0..=30 => 0,
            31..=65 => 1,
            66..=115 => 2,
            _ => 3,
        }
    }

    fn loading_visible(&self) -> bool {
        self.animated || self.started.elapsed() >= Duration::from_millis(150)
    }
}

pub fn render(frame: &mut Frame, state: &IntroState, colors: &ThemeColors, data_ready: bool) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(colors.background)),
        area,
    );

    if !state.animated || area.width < 58 || area.height < 21 {
        render_compact(frame, area, state, colors, data_ready);
        return;
    }

    let [vertical] = Layout::vertical([Constraint::Length(19)])
        .flex(Flex::Center)
        .areas(area);
    let [content] = Layout::horizontal([Constraint::Length(40)])
        .flex(Flex::Center)
        .areas(vertical);
    let [mark_area, title_area, tagline_area, loading_area, hint_area] = Layout::vertical([
        Constraint::Length(9),
        Constraint::Length(5),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .areas(content);
    let phase = state.phase();

    let mark_progress = if phase == 0 {
        state.tick as f32 / 30.0
    } else {
        1.0
    };
    render_art(
        frame,
        mark_area,
        MARK_ART,
        interpolate(colors.background, colors.accent, mark_progress.min(1.0)),
    );

    if phase >= 1 {
        let title_progress = if phase == 1 {
            (state.tick.saturating_sub(31)) as f32 / 34.0
        } else {
            1.0
        };
        render_art(
            frame,
            title_area,
            TITLE_ART,
            interpolate(
                colors.background,
                colors.foreground,
                title_progress.min(1.0),
            ),
        );
    }

    if phase >= 2 {
        let total = TAGLINE.chars().count();
        let visible = if phase == 2 {
            (state.tick.saturating_sub(66) as usize * total / 50).min(total)
        } else {
            total
        };
        frame.render_widget(
            Paragraph::new(TAGLINE.chars().take(visible).collect::<String>())
                .alignment(Alignment::Center)
                .style(Style::default().fg(colors.muted)),
            tagline_area,
        );
    }

    render_loading(frame, loading_area, state, colors, data_ready);
    frame.render_widget(
        Paragraph::new("Press any key to skip")
            .alignment(Alignment::Center)
            .style(Style::default().fg(colors.muted)),
        hint_area,
    );
}

fn render_compact(
    frame: &mut Frame,
    area: Rect,
    state: &IntroState,
    colors: &ThemeColors,
    data_ready: bool,
) {
    let [content] = Layout::vertical([Constraint::Length(3)])
        .flex(Flex::Center)
        .areas(area);
    let lines = vec![
        Line::styled(
            "qari-cli",
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        ),
        if state.loading_visible() {
            loading_line(state.tick, colors, data_ready)
        } else {
            Line::from("")
        },
    ];
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), content);
}

/// Center the art BLOCK in the area (not each line on its own), keeping
/// rows mutually aligned by construction: source rows carry relative
/// indents, trailing whitespace is trimmed, rows are right-padded to the
/// block width, and the whole block is offset to the center. Plain
/// (non-bold) style: faux-bold would overstrike the glyphs.
fn render_art(frame: &mut Frame, area: Rect, art: &[&str], color: Color) {
    let style = Style::default().fg(color);
    let block_width = art
        .iter()
        .map(|line| line.trim_end().width())
        .max()
        .unwrap_or(0);
    let block_pad = (area.width as usize).saturating_sub(block_width) / 2;
    let lines: Vec<Line> = art
        .iter()
        .map(|line| {
            let content = line.trim_end();
            let right = block_width.saturating_sub(content.width());
            Line::styled(
                format!("{}{content}{}", " ".repeat(block_pad), " ".repeat(right)),
                style,
            )
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Left), area);
}

fn render_loading(
    frame: &mut Frame,
    area: Rect,
    state: &IntroState,
    colors: &ThemeColors,
    data_ready: bool,
) {
    frame.render_widget(
        Paragraph::new(loading_line(state.tick, colors, data_ready)).alignment(Alignment::Center),
        area,
    );
}

fn loading_line(tick: u32, colors: &ThemeColors, data_ready: bool) -> Line<'static> {
    let frames = ["|", "/", "—", "\\"];
    let icon = if data_ready {
        "◆"
    } else {
        frames[(tick as usize / 8) % frames.len()]
    };
    let message = if data_ready {
        " Ilm ready"
    } else {
        " Preparing Ilm…"
    };
    Line::from(vec![
        Span::styled(icon.to_string(), Style::default().fg(colors.accent)),
        Span::styled(message, Style::default().fg(colors.muted)),
    ])
}

fn interpolate(from: Color, to: Color, amount: f32) -> Color {
    match (from, to) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => Color::Rgb(
            (r1 as f32 + (r2 as f32 - r1 as f32) * amount) as u8,
            (g1 as f32 + (g2 as f32 - g1 as f32) * amount) as u8,
            (b1 as f32 + (b2 as f32 - b1 as f32) * amount) as u8,
        ),
        _ => to,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn waits_for_animation_and_data() {
        let mut state = IntroState::new(true);
        for _ in 0..=INTRO_END_TICK {
            state.tick();
        }
        assert!(!state.can_enter_reader(false));
        assert!(state.can_enter_reader(true));
    }

    #[test]
    fn skip_still_waits_for_data() {
        let mut state = IntroState::new(true);
        state.skip();
        assert!(!state.can_enter_reader(false));
        assert!(state.can_enter_reader(true));
    }

    #[test]
    fn compact_loading_indicator_is_delayed() {
        let state = IntroState::new(false);
        assert!(!state.loading_visible());
    }

    #[test]
    fn mark_art_rows_share_one_center() {
        // Every row's content must be centered on the same column:
        // 2 * indent + content_width is constant across rows. This holds
        // even when rows have different widths or trailing spaces are
        // stripped, which is exactly what broke the old logo.
        let centers: Vec<usize> = MARK_ART
            .iter()
            .map(|line| {
                let trimmed = line.trim_end();
                let indent = trimmed.chars().take_while(|c| *c == ' ').count();
                indent * 2 + trimmed.trim_start().width()
            })
            .collect();
        for (index, center) in centers.iter().enumerate() {
            assert_eq!(
                *center, centers[0],
                "mark row {index} off-center: {:?}",
                MARK_ART[index]
            );
        }
    }

    #[test]
    fn title_art_fits_content_column() {
        let max = TITLE_ART
            .iter()
            .map(|line| line.trim_end().width())
            .max()
            .unwrap();
        assert!(max <= 38, "title too wide for the 40-col content: {max}");
        assert!(max >= 20, "title looks truncated: {max}");
    }

    #[test]
    fn compact_intro_renders_on_small_terminals() {
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
        let state = IntroState::new(true);
        terminal
            .draw(|frame| render(frame, &state, &Theme::Dark.colors(), false))
            .unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("qari-cli"));
        assert!(output.contains("Preparing Ilm"));
    }

    #[test]
    fn tagline_types_out_before_completion() {
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut state = IntroState::new(true);
        state.tick = 80;
        terminal
            .draw(|frame| render(frame, &state, &Theme::Dark.colors(), true))
            .unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Ilm at"));
        assert!(!output.contains("fingertips"));
    }

    #[test]
    fn full_intro_renders_title_and_tagline() {
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut state = IntroState::new(true);
        state.tick = INTRO_END_TICK;
        terminal
            .draw(|frame| render(frame, &state, &Theme::Dark.colors(), true))
            .unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("| |_| |"));
        assert!(output.contains("Ilm at your fingertips"));
        assert!(output.contains("Ilm ready"));
    }
}
