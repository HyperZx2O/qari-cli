use crate::app::{AppState, Language, Panel};
use crate::theme::ThemeColors;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

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
    let panels = Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Percentage(15),
        Constraint::Percentage(65),
    ])
    .split(panels_area);

    render_header(frame, header_area, state, &colors);
    render_surahs(frame, panels[0], state, &colors);
    render_ayahs(frame, panels[1], state, &colors);
    render_scripture(frame, panels[2], state, &colors);
    render_footer(frame, footer_area, state, &colors);

    if state.search_mode {
        render_search_overlay(frame, panels[2], state, &colors);
    }
    if state.show_help {
        render_help(frame, area, &colors);
    }
}

fn render_header(frame: &mut Frame, area: Rect, state: &AppState, colors: &ThemeColors) {
    let Some(surah) = state.surahs.get(state.current_surah) else {
        return;
    };
    let Some(ayah) = surah.ayahs.get(state.current_ayah) else {
        return;
    };
    let mut spans = vec![
        Span::styled(
            " islam-cli ",
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "◆ {} {}:{} ◆ Juz {} ◆ [{}] ◆ [{}]",
                surah.name_transliterated,
                surah.number,
                ayah.number,
                ayah.juz,
                state.language.label(),
                state.theme.label()
            ),
            Style::default().fg(colors.foreground),
        ),
    ];
    if state.offline_mode {
        spans.push(Span::styled(
            " ◆ [OFFLINE]",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
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
    let list = List::new(items)
        .block(panel_block(" Surahs ", active, colors))
        .style(Style::default().fg(colors.foreground))
        .highlight_style(
            Style::default()
                .bg(colors.highlight)
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
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
    let list = List::new(items)
        .block(panel_block(" Ayahs ", active, colors))
        .style(Style::default().fg(colors.foreground))
        .highlight_style(
            Style::default()
                .bg(colors.highlight)
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(list, area, &mut state.ayah_list);
}

fn render_scripture(frame: &mut Frame, area: Rect, state: &AppState, colors: &ThemeColors) {
    let active = state.active_panel == Panel::Scripture && !state.search_mode;
    let block = panel_block(" Scripture ", active, colors);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(surah) = state.surahs.get(state.current_surah) else {
        return;
    };
    let Some(ayah) = surah.ayahs.get(state.current_ayah) else {
        return;
    };

    let [arabic_area, translation_area, metadata_area] = Layout::vertical([
        Constraint::Percentage(45),
        Constraint::Min(2),
        Constraint::Length(2),
    ])
    .areas(inner);

    let arabic = Paragraph::new(Line::styled(
        ayah.arabic.as_str(),
        Style::default()
            .fg(colors.foreground)
            .add_modifier(Modifier::BOLD),
    ))
    .alignment(Alignment::Right)
    .wrap(Wrap { trim: false });
    frame.render_widget(arabic, arabic_area);

    let translation = match state.language {
        Language::Arabic => vec![Line::styled(
            "Arabic-only view",
            Style::default().fg(colors.muted),
        )],
        Language::English => vec![Line::from(ayah.english.as_str())],
        Language::Bengali => vec![
            Line::from(ayah.english.as_str()),
            Line::from(""),
            Line::from(ayah.bengali.as_str()),
        ],
    };
    frame.render_widget(
        Paragraph::new(translation)
            .style(Style::default().fg(colors.foreground))
            .wrap(Wrap { trim: false }),
        translation_area,
    );

    let metadata = vec![
        Line::styled(
            format!(
                "Surah: {} ({})",
                surah.name_transliterated,
                if surah.is_meccan { "Meccan" } else { "Medinan" }
            ),
            Style::default().fg(colors.muted),
        ),
        Line::styled(
            format!(
                "Juz {} · Page {} · Ayah {}/{}",
                ayah.juz, ayah.page, ayah.number, surah.ayah_count
            ),
            Style::default().fg(colors.muted),
        ),
    ];
    frame.render_widget(Paragraph::new(metadata), metadata_area);
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
            format!(" {message}"),
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Line::styled(
            " j/k navigate · h/l panels · / search · y copy · b bookmark · t theme · v language · ? help · qq quit",
            Style::default().fg(colors.muted),
        )
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

    let items: Vec<ListItem> = state
        .search_results
        .iter()
        .map(|result| {
            ListItem::new(format!(
                "{}:{}  {} — {}",
                result.surah_number,
                result.ayah_number,
                result.surah_name,
                truncate(&result.english_text, 60)
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
                .title(title)
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

fn render_help(frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let popup = centered_rect(66, 70, area);
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
    frame.render_widget(
        Paragraph::new(help)
            .block(
                Block::default()
                    .title(" Keybindings (? to close) ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors.accent))
                    .style(Style::default().bg(colors.surface)),
            )
            .style(Style::default().fg(colors.foreground)),
        popup,
    );
}

fn panel_block(title: &'static str, active: bool, colors: &ThemeColors) -> Block<'static> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if active { colors.accent } else { colors.border }))
        .style(Style::default().bg(colors.surface))
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
