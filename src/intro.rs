use crate::theme::ThemeColors;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;
use std::time::{Duration, Instant};

const MARK_ART: &[&str] = &[
    "              ╭──────╮              ",
    "          ╭───╯      ╰───╮          ",
    "       ╭──╯   ╭────────╮  ╰──╮       ",
    "       │     ╱          ╲    │       ",
    "       │    ╱   ╲    ╱   ╲   │       ",
    "       │   ╱     ╲  ╱     ╲  │       ",
    "       ╰──╯       ╲╱       ╰──╯       ",
    "                   ◆                 ",
];

const TITLE_ART: &[&str] = &[
    " ██████╗  █████╗ ██████╗ ██╗",
    "██╔═══██╗██╔══██╗██╔══██╗██║",
    "██║   ██║███████║██████╔╝██║",
    "██║▄▄ ██║██╔══██║██╔══██╗██║",
    "╚██████╔╝██║  ██║██║  ██║██║",
    " ╚══▀▀═╝ ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝",
];

const INTRO_END_TICK: u32 = 175;

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
            0..=50 => 0,
            51..=95 => 1,
            96..=140 => 2,
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

    if !state.animated || area.width < 58 || area.height < 20 {
        render_compact(frame, area, state, colors, data_ready);
        return;
    }

    let [vertical] = Layout::vertical([Constraint::Length(19)])
        .flex(Flex::Center)
        .areas(area);
    let [content] = Layout::horizontal([Constraint::Length(48)])
        .flex(Flex::Center)
        .areas(vertical);
    let [mark_area, title_area, tagline_area, loading_area, hint_area] = Layout::vertical([
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .areas(content);
    let phase = state.phase();

    let mark_progress = if phase == 0 {
        state.tick as f32 / 50.0
    } else {
        1.0
    };
    render_art(
        frame,
        mark_area,
        MARK_ART,
        interpolate(colors.background, colors.accent, mark_progress.min(1.0)),
        false,
    );

    if phase >= 1 {
        let title_progress = if phase == 1 {
            (state.tick.saturating_sub(51)) as f32 / 44.0
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
            true,
        );
    }

    if phase >= 2 {
        let tagline = "The Quran at your fingertips";
        let visible = if phase == 2 {
            state.tick.saturating_sub(96) as usize * tagline.chars().count() / 44
        } else {
            tagline.chars().count()
        };
        frame.render_widget(
            Paragraph::new(tagline.chars().take(visible).collect::<String>())
                .alignment(Alignment::Left)
                .style(Style::default().fg(colors.muted)),
            tagline_area,
        );
    }

    render_loading(frame, loading_area, state, colors, data_ready);
    frame.render_widget(
        Paragraph::new("Press any key to skip")
            .alignment(Alignment::Right)
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

fn render_art(frame: &mut Frame, area: Rect, art: &[&str], color: Color, bold: bool) {
    let mut style = Style::default().fg(color);
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    let lines: Vec<Line> = art.iter().map(|line| Line::styled(*line, style)).collect();
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

fn render_loading(
    frame: &mut Frame,
    area: Rect,
    state: &IntroState,
    colors: &ThemeColors,
    data_ready: bool,
) {
    frame.render_widget(
        Paragraph::new(loading_line(state.tick, colors, data_ready)).alignment(Alignment::Left),
        area,
    );
}

fn loading_line(tick: u32, colors: &ThemeColors, data_ready: bool) -> Line<'static> {
    let frames = ["◐", "◓", "◑", "◒"];
    let icon = if data_ready {
        "◆"
    } else {
        frames[(tick as usize / 8) % frames.len()]
    };
    let message = if data_ready {
        " Quran ready"
    } else {
        " Preparing Quran…"
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
        assert!(output.contains("Preparing Quran"));
    }

    #[test]
    fn full_intro_renders_title_and_tagline() {
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut state = IntroState::new(true);
        state.tick = 150;
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
        assert!(output.contains("██████"));
        assert!(output.contains("The Quran at your fingertips"));
        assert!(output.contains("Quran ready"));
    }
}
