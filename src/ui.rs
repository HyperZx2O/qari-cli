use crate::app::{AppState, Language, Panel};
use crate::theme::ThemeColors;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Wrap,
};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

pub fn draw(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();
    let colors = state.theme.colors();
    frame.render_widget(
        Block::default().style(Style::default().bg(colors.background)),
        area,
    );

    let [header_area, panels_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);
    render_header(frame, header_area, state, &colors);
    let scripture_area = if panels_area.width < 72 {
        match state.active_panel {
            Panel::Surahs => render_surahs(frame, panels_area, state, &colors),
            Panel::Ayahs => render_ayahs(frame, panels_area, state, &colors),
            Panel::Scripture => render_scripture(frame, panels_area, state, &colors),
        }
        panels_area
    } else {
        let surah_width = if panels_area.width < 100 { 24 } else { 30 };
        let panels = Layout::horizontal([
            Constraint::Length(surah_width),
            Constraint::Length(12),
            Constraint::Min(24),
        ])
        .split(panels_area);
        render_surahs(frame, panels[0], state, &colors);
        render_ayahs(frame, panels[1], state, &colors);
        render_scripture(frame, panels[2], state, &colors);
        panels[2]
    };
    render_footer(frame, footer_area, state, &colors);

    if state.search_mode {
        render_search_overlay(frame, scripture_area, state, &colors);
    }
    if state.show_help {
        render_help(frame, area, state, &colors);
    }
}

fn render_header(frame: &mut Frame, area: Rect, state: &AppState, colors: &ThemeColors) {
    let Some(surah) = state.surahs.get(state.current_surah) else {
        return;
    };
    let Some(ayah) = surah.ayahs.get(state.current_ayah) else {
        return;
    };
    let location = if area.width >= 72 {
        format!(
            "◆ {} {}:{} ◆ Juz {} ◆ [{}] ◆ [{}]",
            surah.name_transliterated,
            surah.number,
            ayah.number,
            ayah.juz,
            state.language.label(),
            state.theme.label()
        )
    } else {
        format!(
            "{} {}:{} [{}]",
            surah.name_transliterated,
            surah.number,
            ayah.number,
            state.language.label()
        )
    };
    let mut spans = vec![
        Span::styled(
            " qari-cli ",
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(location, Style::default().fg(colors.foreground)),
    ];
    if state.offline_mode {
        spans.push(Span::styled(
            " ◆ [OFFLINE]",
            Style::default()
                .fg(colors.danger)
                .add_modifier(Modifier::BOLD),
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(colors.surface)),
        area,
    );
}

fn render_surahs(frame: &mut Frame, area: Rect, state: &mut AppState, colors: &ThemeColors) {
    let active = state.active_panel == Panel::Surahs && !state.search_mode;
    let items: Vec<ListItem> = state
        .surahs
        .iter()
        .map(|surah| {
            let badge = if surah.is_meccan { "M" } else { "D" };
            ListItem::new(format!(
                "{:>3}. {} [{}]",
                surah.number, surah.name_transliterated, badge
            ))
        })
        .collect();
    let highlight_style = if active {
        Style::default()
            .bg(colors.highlight)
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.foreground)
    };
    let list = List::new(items)
        .block(panel_block(" Surahs ", active, colors))
        .style(Style::default().fg(colors.foreground))
        .highlight_style(highlight_style)
        .highlight_symbol(if active { "▶ " } else { "  " });
    frame.render_stateful_widget(list, area, &mut state.surah_list);
}

fn render_ayahs(frame: &mut Frame, area: Rect, state: &mut AppState, colors: &ThemeColors) {
    let active = state.active_panel == Panel::Ayahs && !state.search_mode;
    let items: Vec<ListItem> = state
        .surahs
        .get(state.current_surah)
        .map(|surah| {
            surah
                .ayahs
                .iter()
                .map(|ayah| ListItem::new(format!("Ayah {}", ayah.number)))
                .collect()
        })
        .unwrap_or_default();
    let highlight_style = if active {
        Style::default()
            .bg(colors.highlight)
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.foreground)
    };
    let list = List::new(items)
        .block(panel_block(" Ayahs ", active, colors))
        .style(Style::default().fg(colors.foreground))
        .highlight_style(highlight_style)
        .highlight_symbol(if active { "▶ " } else { "  " });
    frame.render_stateful_widget(list, area, &mut state.ayah_list);
}

fn render_scripture(frame: &mut Frame, area: Rect, state: &mut AppState, colors: &ThemeColors) {
    let active = state.active_panel == Panel::Scripture && !state.search_mode;
    let block = panel_block(" Scripture ", active, colors);
    let inner = block.inner(area);
    let content_width = inner.width.saturating_sub(1).max(1) as usize;

    let Some((arabic, english, bengali, metadata_one, metadata_two)) =
        state.surahs.get(state.current_surah).and_then(|surah| {
            surah.ayahs.get(state.current_ayah).map(|ayah| {
                (
                    ayah.arabic.clone(),
                    ayah.english.clone(),
                    ayah.bengali.clone(),
                    format!(
                        "Surah: {} ({})",
                        surah.name_transliterated,
                        if surah.is_meccan { "Meccan" } else { "Medinan" }
                    ),
                    format!(
                        "Juz {} · Page {} · Ayah {}/{}",
                        ayah.juz, ayah.page, ayah.number, surah.ayah_count
                    ),
                )
            })
        })
    else {
        frame.render_widget(block, area);
        return;
    };

    let arabic_lines = crate::rtl::terminal_lines(&arabic, content_width, state.rtl_mode);
    let mut lines: Vec<Line> = arabic_lines
        .iter()
        .map(|line| {
            Line::styled(
                line.clone(),
                Style::default()
                    .fg(colors.foreground)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Right)
        })
        .collect();
    lines.push(Line::from(""));

    let mut content_height = arabic_lines.len() + 1;
    match state.language {
        Language::Arabic => {}
        Language::English => {
            content_height += wrapped_height(&english, content_width);
            lines.push(Line::styled(
                english,
                Style::default().fg(colors.foreground),
            ));
        }
        Language::Bengali => {
            content_height += wrapped_height(&english, content_width) + 1;
            lines.push(Line::styled(
                english,
                Style::default().fg(colors.foreground),
            ));
            lines.push(Line::from(""));
            content_height += wrapped_height(&bengali, content_width);
            lines.push(Line::styled(
                bengali,
                Style::default().fg(colors.foreground),
            ));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        metadata_one.clone(),
        Style::default().fg(colors.muted),
    ));
    lines.push(Line::styled(
        metadata_two.clone(),
        Style::default().fg(colors.muted),
    ));
    content_height += 1
        + wrapped_height(&metadata_one, content_width)
        + wrapped_height(&metadata_two, content_width);
    if state.bookmarked {
        lines.push(Line::styled(
            "★ Bookmarked",
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        ));
        content_height += 1;
    }

    let max_scroll = content_height.saturating_sub(inner.height as usize) as u16;
    state.scripture_scroll = state.scripture_scroll.min(max_scroll);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((state.scripture_scroll, 0)),
        area,
    );

    if max_scroll > 0 {
        let mut scrollbar_state =
            ScrollbarState::new(max_scroll as usize).position(state.scripture_scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .style(Style::default().fg(colors.accent)),
            inner,
            &mut scrollbar_state,
        );
    }
}

fn render_footer(frame: &mut Frame, area: Rect, state: &AppState, colors: &ThemeColors) {
    let line = if state.search_mode {
        Line::from(vec![
            Span::styled(" / ", Style::default().fg(colors.accent)),
            Span::styled(
                state.search_query.as_str(),
                Style::default().fg(colors.foreground),
            ),
            Span::styled("█", Style::default().fg(colors.accent)),
            Span::styled(
                "  ↑/↓ choose · Enter open · Esc close",
                Style::default().fg(colors.muted),
            ),
        ])
    } else if let Some(message) = &state.status_msg {
        Line::styled(
            format!(" {}{message}", if state.status_error { "! " } else { "" }),
            Style::default()
                .fg(if state.status_error {
                    colors.danger
                } else {
                    colors.accent
                })
                .add_modifier(Modifier::BOLD),
        )
    } else {
        let hint = if area.width >= 108 {
            if state.active_panel == Panel::Scripture {
                " j/k scroll · h/l panels · / search · y copy · b bookmark · t theme · v language · ? help · qq quit"
            } else {
                " j/k navigate · h/l panels · / search · y copy · b bookmark · t theme · v language · ? help · qq quit"
            }
        } else if area.width >= 64 {
            " j/k move · h/l panels · / search · ? help · qq quit"
        } else {
            " j/k · h/l · / · ? · qq"
        };
        Line::styled(hint, Style::default().fg(colors.muted))
    };
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(colors.surface)),
        area,
    );
}

fn render_search_overlay(
    frame: &mut Frame,
    scripture_area: Rect,
    state: &mut AppState,
    colors: &ThemeColors,
) {
    let area = Rect {
        x: scripture_area.x.saturating_add(1),
        y: scripture_area.y.saturating_add(1),
        width: scripture_area.width.saturating_sub(2),
        height: scripture_area.height.saturating_sub(2),
    };
    frame.render_widget(Clear, area);

    let result_width = area.width.saturating_sub(18).clamp(12, 72) as usize;
    let items: Vec<ListItem> = state
        .search_results
        .iter()
        .map(|result| {
            ListItem::new(format!(
                "{}:{}  {} — {}",
                result.surah_number,
                result.ayah_number,
                result.surah_name,
                truncate(&result.english_text, result_width)
            ))
        })
        .collect();
    let title = if state.search_query.chars().count() < 2 {
        " Search — type at least 2 characters ".to_string()
    } else {
        format!(" Search — {} results ", state.search_results.len())
    };
    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(colors.accent)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.accent))
                .style(Style::default().bg(colors.surface)),
        )
        .style(Style::default().fg(colors.foreground))
        .highlight_style(
            Style::default()
                .bg(colors.highlight)
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(list, area, &mut state.search_list);
}

fn render_help(frame: &mut Frame, area: Rect, state: &mut AppState, colors: &ThemeColors) {
    let popup = if area.width < 72 || area.height < 22 {
        Rect {
            x: area.x.saturating_add(1),
            y: area.y.saturating_add(1),
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        }
    } else {
        centered_rect(66, 70, area)
    };
    frame.render_widget(Clear, popup);
    let help = vec![
        Line::from(""),
        Line::from("  h / ←     Move to left panel"),
        Line::from("  l / →     Move to right panel"),
        Line::from("  j / ↓     Move down"),
        Line::from("  k / ↑     Move up"),
        Line::from("  Enter     Select"),
        Line::from("  /         Search"),
        Line::from("  y / c     Copy ayah"),
        Line::from("  b         Toggle bookmark"),
        Line::from("  t         Cycle theme"),
        Line::from("  v         Cycle language"),
        Line::from("  ?         Close help"),
        Line::from("  qq        Quit"),
        Line::from("  Ctrl+C    Quit immediately"),
    ];
    let visible_rows = popup.height.saturating_sub(2) as usize;
    let max_scroll = help.len().saturating_sub(visible_rows) as u16;
    state.help_scroll = state.help_scroll.min(max_scroll);
    frame.render_widget(
        Paragraph::new(help)
            .block(
                Block::default()
                    .title(" Keybindings (? to close) ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors.accent))
                    .style(Style::default().bg(colors.surface)),
            )
            .style(Style::default().fg(colors.foreground))
            .wrap(Wrap { trim: false })
            .scroll((state.help_scroll, 0)),
        popup,
    );
}

fn panel_block(title: &'static str, active: bool, colors: &ThemeColors) -> Block<'static> {
    let title = if active {
        format!(" > {} ", title.trim())
    } else {
        title.to_string()
    };
    Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(if active { colors.accent } else { colors.muted })
                .add_modifier(if active {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if active { colors.accent } else { colors.border }))
        .style(Style::default().bg(colors.surface))
}

fn wrapped_height(text: &str, width: usize) -> usize {
    let width = width.max(1);
    let mut rows = 1usize;
    let mut column = 0usize;
    for word in text.split_whitespace() {
        let word_width = UnicodeWidthStr::width(word);
        let separator = usize::from(column > 0);
        if column > 0 && column + separator + word_width > width {
            rows += 1;
            column = word_width;
        } else {
            column += separator + word_width;
        }
    }
    rows
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [vertical] = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .areas(area);
    let [centered] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(vertical);
    centered
}

fn truncate(text: &str, max_chars: usize) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= max_chars {
        return one_line;
    }
    let mut result: String = one_line.chars().take(max_chars.saturating_sub(1)).collect();
    result.push('…');
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Language;
    use crate::theme::Theme;
    use ratatui::backend::TestBackend;
    use ratatui::widgets::ListState;
    use ratatui::Terminal;

    #[test]
    fn arabic_stays_inside_scripture_panel() {
        let mut state = test_state();
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let buffer = terminal.backend().buffer();

        for y in 1..31 {
            for x in 0..42 {
                let symbol = buffer[(x, y)].symbol();
                assert!(
                    !symbol.chars().any(is_arabic_presentation_form),
                    "Arabic escaped into cell ({x}, {y})"
                );
            }
        }

        assert_eq!(buffer[(29, 5)].symbol(), "│");
        assert_eq!(buffer[(30, 5)].symbol(), "│");
        assert_eq!(buffer[(41, 5)].symbol(), "│");
        assert_eq!(buffer[(42, 5)].symbol(), "│");
    }

    #[test]
    fn redraw_removes_previous_ayah_text() {
        let mut state = test_state();
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();

        state.current_ayah = 1;
        state.ayah_list.select(Some(1));
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();

        let rendered = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(!rendered.contains("In the name of Allah"));
        assert_eq!(
            rendered.matches("[All] praise is [due] to Allah").count(),
            1
        );
        assert_eq!(rendered.matches("Surah: Al-Fatihah").count(), 1);
        assert_eq!(rendered.matches("Page 1").count(), 1);
    }

    #[test]
    fn long_scripture_scrolls_to_metadata() {
        let mut state = test_state();
        state.active_panel = Panel::Scripture;
        state.surahs[0].ayahs[0].english = "A long translated passage ".repeat(120);
        state.scripture_scroll = u16::MAX;
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let rendered = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(state.scripture_scroll > 0);
        assert!(rendered.contains("Surah: Al-Fatihah"));
        assert!(rendered.contains("Ayah 1/7"));
    }

    #[test]
    fn narrow_terminal_shows_only_active_panel() {
        let mut state = test_state();
        state.active_panel = Panel::Scripture;
        let mut terminal = Terminal::new(TestBackend::new(60, 18)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let rendered = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("Scripture"));
        assert!(!rendered.contains("Surahs"));
        assert!(!rendered.contains("Ayahs"));
    }

    fn test_state() -> AppState {
        let surahs = crate::data::load_fallback();
        let mut surah_list = ListState::default();
        surah_list.select(Some(0));
        let mut ayah_list = ListState::default();
        ayah_list.select(Some(0));

        AppState {
            surahs,
            current_surah: 0,
            current_ayah: 0,
            active_panel: Panel::Surahs,
            language: Language::English,
            theme: Theme::Dark,
            search_mode: false,
            search_query: String::new(),
            search_results: Vec::new(),
            show_help: false,
            offline_mode: false,
            status_msg: None,
            status_error: false,
            quit_count: 0,
            quit_started: None,
            surah_list,
            ayah_list,
            search_list: ListState::default(),
            bookmark_conn: None,
            bookmarked: false,
            scripture_scroll: 0,
            help_scroll: 0,
            rtl_mode: crate::rtl::RtlMode::Visual,
            status_started: None,
        }
    }

    fn is_arabic_presentation_form(character: char) -> bool {
        matches!(character as u32, 0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
    }
}
