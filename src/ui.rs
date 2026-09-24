use crate::app::{AppState, Language, Panel};
use crate::collection::CollectionId;
use crate::quran::Surah;
use crate::theme::ThemeColors;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Wrap,
};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();
    let colors = state.theme.colors();
    frame.render_widget(
        Block::default().style(Style::default().bg(colors.background)),
        area,
    );

    // Overdrive (quiet devotion progress): a hairline under the header
    // filling with position inside the current Juz. Static, accent on a
    // border track, wide terminals only — the row collapses to zero height
    // on narrow screens so compact mode stays minimal.
    let wide = area.width >= 72;
    let [header_area, progress_area, panels_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(u16::from(wide)),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);
    render_header(frame, header_area, state, &colors);
    if wide {
        render_juz_progress(frame, progress_area, state, &colors);
    }
    let scripture_area = if panels_area.width < 72 {
        match state.active_panel {
            Panel::Surahs => render_books(frame, panels_area, state, &colors),
            Panel::Ayahs => render_units(frame, panels_area, state, &colors),
            Panel::Scripture => render_scripture(frame, panels_area, state, &colors),
        }
        panels_area
    } else {
        // Books holds 14 shelf rows (six revelations + eight hadith books);
        // 22 gives its longest label ("Sahih Bukhari", "Muwatta Malik")
        // exactly its 13 columns after borders, the `▶ ` highlight symbol
        // and the `{:>3}. ` prefix. Units keeps 24 — it already fits.
        let panels = Layout::horizontal([
            Constraint::Length(22),
            Constraint::Length(24),
            Constraint::Min(24),
        ])
        .split(panels_area);
        render_books(frame, panels[0], state, &colors);
        render_units(frame, panels[1], state, &colors);
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
    if state.show_verse {
        render_verse(frame, area, state, &colors);
    }
}

fn render_header(frame: &mut Frame, area: Rect, state: &AppState, colors: &ThemeColors) {
    let Some(location) = header_location(state, area.width) else {
        return;
    };
    let mut spans = vec![
        Span::styled(
            " qari-cli ",
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            crate::output::sanitize_terminal_text(&location),
            Style::default().fg(colors.foreground),
        ),
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

/// Header location line, or None when the current selection names nothing
/// (empty corpus, books still loading).
fn header_location(state: &AppState, width: u16) -> Option<String> {
    if state.collection == CollectionId::Quran {
        let surah = state.surahs.get(state.current_surah)?;
        let ayah = surah.ayahs.get(state.current_ayah)?;
        return Some(if width >= 72 {
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
        });
    }
    let reference = state
        .units
        .get(state.current_unit)
        .map(|unit| unit.label.clone())
        .unwrap_or_else(|| format!("Chapter {}", state.current_chapter.max(1)));
    Some(if width >= 72 {
        format!(
            "◆ {} · {} ◆ [{}] ◆ [{}]",
            state.collection.label(),
            reference,
            state.language.label(),
            state.theme.label()
        )
    } else {
        format!(
            "{} {} [{}]",
            state.collection.label(),
            reference,
            state.language.label()
        )
    })
}

/// Position of the cursor inside the shelf: (1-based index, shelf
/// total) over the flattened units. Juz progress for the Quran.
/// `(0, 1)` when nothing is selected.
fn reading_progress(state: &AppState) -> (usize, usize) {
    if state.collection == CollectionId::Quran {
        return juz_position(&state.surahs, state.current_surah, state.current_ayah);
    }
    if state.units.is_empty() {
        return (0, 1);
    }
    (state.current_unit + 1, state.units.len())
}

/// Position of one ayah inside its Juz: (1-based index, Juz ayah total),
/// counted over loaded surahs in reading order. `(0, 1)` when out of range.
/// Shares `quran::juz_entries`, the single Juz scan in the app.
fn juz_position(surahs: &[Surah], current_surah: usize, current_ayah: usize) -> (usize, usize) {
    let Some(juz) = surahs
        .get(current_surah)
        .and_then(|surah| surah.ayahs.get(current_ayah))
        .map(|ayah| ayah.juz)
    else {
        return (0, 1);
    };
    let entries = crate::quran::juz_entries(surahs, juz);
    let done = entries
        .iter()
        .position(|entry| *entry > (current_surah, current_ayah))
        .unwrap_or(entries.len());
    (done, entries.len().max(1))
}

fn render_juz_progress(frame: &mut Frame, area: Rect, state: &AppState, colors: &ThemeColors) {
    let (done, total) = reading_progress(state);
    let width = area.width as usize;
    let filled = ((done as u64 * width as u64) / total.max(1) as u64).min(width as u64) as usize;
    let mut spans = Vec::with_capacity(2);
    if filled > 0 {
        spans.push(Span::styled(
            "─".repeat(filled),
            Style::default().fg(colors.accent),
        ));
    }
    if filled < width {
        spans.push(Span::styled(
            "─".repeat(width - filled),
            Style::default().fg(colors.border),
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(colors.background)),
        area,
    );
}

fn render_books(frame: &mut Frame, area: Rect, state: &mut AppState, colors: &ThemeColors) {
    use crate::collection::CollectionId;
    frame.render_widget(Clear, area);
    let active = state.active_panel == Panel::Surahs && !state.search_mode;
    let items: Vec<ListItem> = if state.juz_mode && state.collection == CollectionId::Quran {
        (1..=30u8)
            .map(|juz| ListItem::new(format!("Juz {juz}")))
            .collect()
    } else {
        CollectionId::library()
            .iter()
            .enumerate()
            .map(|(index, id)| ListItem::new(format!("{:>3}. {}", index + 1, id.label())))
            .collect()
    };
    let highlight_style = if active {
        Style::default()
            .bg(colors.highlight)
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.foreground)
    };
    let shelf = if state.juz_mode && state.collection == CollectionId::Quran {
        " Juz "
    } else {
        " Books "
    };
    let list = List::new(items)
        .block(panel_block(shelf, active, colors))
        .style(Style::default().fg(colors.foreground))
        .highlight_style(highlight_style)
        .highlight_symbol(if active { "▶ " } else { "  " });
    if state.juz_mode && state.collection == CollectionId::Quran {
        frame.render_stateful_widget(list, area, &mut state.surah_list);
    } else {
        frame.render_stateful_widget(list, area, &mut state.library_list);
    }
}

fn render_units(frame: &mut Frame, area: Rect, state: &mut AppState, colors: &ThemeColors) {
    frame.render_widget(Clear, area);
    let active = state.active_panel == Panel::Ayahs && !state.search_mode;
    let (items, use_unit_list): (Vec<ListItem>, bool) = if state.juz_mode
        && state.collection == CollectionId::Quran
    {
        (
            crate::quran::juz_entries(&state.surahs, state.current_juz)
                .into_iter()
                .map(|(surah_index, ayah_index)| {
                    let surah = &state.surahs[surah_index];
                    let ayah = &surah.ayahs[ayah_index];
                    ListItem::new(format!("{}:{}", surah.number, ayah.number))
                })
                .collect(),
            false,
        )
    } else {
        match state.collection {
            CollectionId::Quran => (
                state
                    .surahs
                    .iter()
                    .map(|surah| {
                        let badge = if surah.is_meccan { "M" } else { "D" };
                        ListItem::new(crate::output::sanitize_terminal_text(&format!(
                            "{:>3}. {} [{}]",
                            surah.number, surah.name_transliterated, badge
                        )))
                    })
                    .collect(),
                false,
            ),
            _ => (
                state
                    .units
                    .iter()
                    .map(|unit| ListItem::new(crate::output::sanitize_terminal_text(&unit.label)))
                    .collect(),
                true,
            ),
        }
    };
    let highlight_style = if active {
        Style::default()
            .bg(colors.highlight)
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.foreground)
    };
    let list = List::new(items)
        .block(panel_block(
            state.collection.section_title(),
            active,
            colors,
        ))
        .style(Style::default().fg(colors.foreground))
        .highlight_style(highlight_style)
        .highlight_symbol(if active { "▶ " } else { "  " });
    if use_unit_list {
        frame.render_stateful_widget(list, area, &mut state.unit_list);
    } else if state.juz_mode {
        frame.render_stateful_widget(list, area, &mut state.ayah_list);
    } else {
        frame.render_stateful_widget(list, area, &mut state.surah_list);
    }
}

/// Wrapped rows for one text, with a blank fallback for empty input.
fn wrap_or_blank(text: &str, content_width: usize) -> Vec<String> {
    let mut wrapped =
        crate::wrap::wrap_text(&crate::output::sanitize_terminal_text(text), content_width);
    if wrapped.is_empty() {
        wrapped.push(String::new());
    }
    wrapped
}

/// Wrap one text into styled rows. The shared body behind the three
/// once-local `push_wrapped` closures.
fn push_wrapped(lines: &mut Vec<Line>, text: &str, style: Style, content_width: usize) {
    for row in wrap_or_blank(text, content_width) {
        lines.push(Line::styled(row, style));
    }
}

/// Shelf-list selection treatment, reused for the verse cursor.
fn highlight_style(colors: &ThemeColors) -> Style {
    Style::default()
        .bg(colors.highlight)
        .fg(colors.accent)
        .add_modifier(Modifier::BOLD)
}

/// Cursor-verse label shared by both scripture panes: `▶ ` + highlight
/// while focused, the plain muted label otherwise.
fn cursor_line(text: String, cursor: bool, muted: Style, highlight: Style) -> (String, Style) {
    if cursor {
        (format!("▶ {text}"), highlight)
    } else {
        (text, muted)
    }
}

/// Centered popup frame shared by the help and verse modals.
fn modal_frame(area: Rect) -> Rect {
    if area.width < 72 || area.height < 22 {
        Rect {
            x: area.x.saturating_add(1),
            y: area.y.saturating_add(1),
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        }
    } else {
        centered_rect(66, 70, area)
    }
}

fn render_scripture(frame: &mut Frame, area: Rect, state: &mut AppState, colors: &ThemeColors) {
    frame.render_widget(Clear, area);
    let active = state.active_panel == Panel::Scripture && !state.search_mode;
    let block = panel_block(" Scripture ", active, colors);
    let inner = block.inner(area);
    // Reserve the last inner column for the scrollbar (and as a hard
    // guarantee that no scripture row ever reaches the terminal's final
    // column, where an unexpected wide-advance would wrap and corrupt the
    // rows below).
    let content_width = inner.width.saturating_sub(1).max(1) as usize;

    if state.collection != CollectionId::Quran {
        render_reader_panel(frame, area, block, inner, content_width, state, colors);
        return;
    }

    let Some(surah) = state.surahs.get(state.current_surah).cloned() else {
        frame.render_widget(block, area);
        return;
    };

    let muted = Style::default().fg(colors.muted);
    let body = Style::default().fg(colors.foreground);
    let sacred = Style::default()
        .fg(colors.foreground)
        .add_modifier(Modifier::BOLD);
    let highlight = highlight_style(colors);

    let mut lines: Vec<Line> = Vec::new();
    let mut verse_starts: Vec<u16> = Vec::new();
    // Headline: surah-level context once, then every ayah scrolls under it.
    push_wrapped(
        &mut lines,
        &format!(
            "Surah {} ({}) · {} ayahs",
            surah.name_transliterated,
            if surah.is_meccan { "Meccan" } else { "Medinan" },
            surah.ayahs.len().max(1)
        ),
        muted,
        content_width,
    );
    lines.push(Line::from(""));

    for (ayah_index, ayah) in surah.ayahs.iter().enumerate() {
        verse_starts.push(lines.len() as u16);
        let cursor = ayah_index == state.current_ayah && active;
        let (label, style) = cursor_line(format!("Ayah {}", ayah.number), cursor, muted, highlight);
        push_wrapped(&mut lines, &label, style, content_width);
        for row in crate::rtl::terminal_lines(&ayah.arabic, content_width, state.rtl_mode) {
            lines.push(Line::styled(row, sacred).alignment(Alignment::Right));
        }
        if state.show_transliteration {
            if let Some((_, transliterated)) = state
                .translit_cache
                .get(&surah.number)
                .and_then(|verses| verses.iter().find(|(id, _)| *id == ayah.number.to_string()))
            {
                push_wrapped(&mut lines, transliterated, muted, content_width);
            }
        }
        lines.push(Line::styled(
            "─".repeat(content_width),
            Style::default().fg(colors.accent),
        ));
        lines.push(Line::from(""));
        if state.language == Language::English {
            push_wrapped(&mut lines, &ayah.english, body, content_width);
        }
        lines.push(Line::from(""));
        if state.bookmarked && ayah_index == state.current_ayah {
            lines.push(Line::styled(
                "★ Bookmarked",
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::from(""));
        }
    }

    scroll_to_verse(state, &verse_starts);
    render_scroll_lines(frame, area, block, inner, lines, state, colors);
}

/// Scripture panel for every non-Quran shelf: the whole unit (chapter,
/// psalm, or hadith) scrolls under a verse cursor, with English where the
/// edition has it and an honest `(AR)` marker where it does not.
fn render_reader_panel(
    frame: &mut Frame,
    area: Rect,
    block: Block<'static>,
    inner: Rect,
    content_width: usize,
    state: &mut AppState,
    colors: &ThemeColors,
) {
    let muted = Style::default().fg(colors.muted);
    let body = Style::default().fg(colors.foreground);
    let sacred = Style::default()
        .fg(colors.foreground)
        .add_modifier(Modifier::BOLD);
    let highlight = highlight_style(colors);
    let active = state.active_panel == Panel::Scripture && !state.search_mode;
    let mut lines: Vec<Line> = Vec::new();
    let mut verse_starts: Vec<u16> = Vec::new();

    let Some(texts) = state.chapter_texts() else {
        let message = state
            .chapter_error
            .clone()
            .unwrap_or_else(|| "Select a unit to read".to_string());
        lines.push(Line::styled(
            crate::output::sanitize_terminal_text(&message),
            muted,
        ));
        render_scroll_lines(frame, area, block, inner, lines, state, colors);
        return;
    };
    let heading = format!(
        "{} {} · {}",
        state.current_book_label(),
        state.current_chapter,
        state.collection.label()
    );
    push_wrapped(&mut lines, &heading, muted, content_width);
    lines.push(Line::from(""));
    for (verse_index, (verse_id, arabic)) in texts.arabic.iter().enumerate() {
        verse_starts.push(lines.len() as u16);
        let cursor = verse_index == state.current_ayah && active;
        let (label, style) = cursor_line(format!("Verse {verse_id}"), cursor, muted, highlight);
        push_wrapped(&mut lines, &label, style, content_width);
        for row in crate::rtl::terminal_lines(
            &crate::output::sanitize_terminal_text(arabic),
            content_width,
            state.rtl_mode,
        ) {
            lines.push(Line::styled(row, sacred).alignment(Alignment::Right));
        }
        if state.language == Language::English {
            match texts.english.iter().find(|(id, _)| id == verse_id) {
                Some((_, english)) => push_wrapped(&mut lines, english, body, content_width),
                // Translated editions cover only some books: show the
                // primary-script verse with an honest marker instead of a gap.
                None => lines.push(Line::styled("(AR)", muted)),
            }
        }
        lines.push(Line::from(""));
        if state.bookmarked && verse_index == state.current_ayah {
            lines.push(Line::styled(
                "★ Bookmarked",
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::from(""));
        }
    }

    scroll_to_verse(state, &verse_starts);
    render_scroll_lines(frame, area, block, inner, lines, state, colors);
}

/// Advance the reading pane to the cursor verse when a movement (or a
/// fresh unit selection) asked for it. The terminal-height clamp is
/// render_scroll_lines' job; here we only pick the row to land on.
/// Landing near the head of a unit keeps the surah/chapter headline
/// visible instead of shoving it off the top.
fn scroll_to_verse(state: &mut AppState, verse_starts: &[u16]) {
    if !state.scroll_to_verse {
        return;
    }
    if let Some(&start) = verse_starts.get(state.current_ayah) {
        // The first verse starts right under the headline, which can wrap
        // to several rows on narrow panels; scrolling to `start` there
        // would hide the headline the comment above promises to keep.
        state.scripture_scroll = if state.current_ayah == 0 { 0 } else { start };
    }
    state.scroll_to_verse = false;
}

fn render_scroll_lines(
    frame: &mut Frame,
    area: Rect,
    block: Block<'static>,
    inner: Rect,
    lines: Vec<Line<'static>>,
    state: &mut AppState,
    colors: &ThemeColors,
) {
    let max_scroll = (lines.len() as u16).saturating_sub(inner.height);
    state.scripture_scroll = state.scripture_scroll.min(max_scroll);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
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
        Line::styled(
            footer_hint(state.active_panel == Panel::Scripture, area.width),
            Style::default().fg(colors.muted),
        )
    };
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(colors.surface)),
        area,
    );
}

/// Footer key hint for the idle (no search, no status) footer: every
/// variant names the verse focus modal and must fit its width gate —
/// pinned by `footer_hints_fit_their_gates`.
fn footer_hint(active_scripture: bool, width: u16) -> &'static str {
    if width >= 108 {
        if active_scripture {
            " j/k verse · h/l panels · / search · y copy · b bookmark · t theme · v language · f verse · ? help · qq quit"
        } else {
            " j/k move · h/l panels · / search · y copy · b bookmark · t theme · v language · f verse · ? help · qq quit"
        }
    } else if width >= 64 {
        " j/k move · h/l panels · / search · f verse · ? help · qq quit"
    } else {
        " j/k · h/l · / · f · ? · qq"
    }
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
            ListItem::new(crate::wrap::truncate_clusters(
                &crate::output::sanitize_terminal_text(&result.display),
                result_width,
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
    let popup = modal_frame(area);
    frame.render_widget(Clear, popup);
    let help = vec![
        Line::from(""),
        Line::from("  h / ←     Book column ←"),
        Line::from("  l / →     Verse / text column →"),
        Line::from("  j / ↓     Move down (in the verse/unit row)"),
        Line::from("  k / ↑     Move up"),
        Line::from("  Enter     Move into the next column"),
        Line::from("  /         Search"),
        Line::from("  y / c     Copy verse"),
        Line::from("  b         Toggle bookmark"),
        Line::from("  t         Cycle theme"),
        Line::from("  v         Cycle language"),
        Line::from("  f         Verse focus (transliteration + translation)"),
        Line::from("  i         Transliteration (Quran)"),
        Line::from("  J         Juz navigation (Quran)"),
        Line::from("  ?         Close help"),
        Line::from("  qq        Quit"),
        Line::from("  Ctrl+C    Quit immediately"),
        Line::from("  Ctrl+L    Redraw screen"),
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

/// Verse focus modal (`f`): the verse under the cursor with its
/// transliteration where cached (Quran-only) and its translation
/// highlighted where translated — `(AR)` where not. Same popup shape
/// as help; silent None when the verse has no loaded rows.
fn render_verse(frame: &mut Frame, area: Rect, state: &mut AppState, colors: &ThemeColors) {
    let popup = modal_frame(area);
    let content_width = popup.width.saturating_sub(2).max(1) as usize;
    let Some((title, lines)) = verse_modal_lines(state, colors, content_width) else {
        return;
    };
    frame.render_widget(Clear, popup);
    let visible_rows = popup.height.saturating_sub(2) as usize;
    let max_scroll = lines.len().saturating_sub(visible_rows) as u16;
    state.verse_scroll = state.verse_scroll.min(max_scroll);
    frame.render_widget(
        Paragraph::new(lines)
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
            .wrap(Wrap { trim: false })
            .scroll((state.verse_scroll, 0)),
        popup,
    );
}

fn verse_modal_lines(
    state: &AppState,
    colors: &ThemeColors,
    content_width: usize,
) -> Option<(String, Vec<Line<'static>>)> {
    let muted = Style::default().fg(colors.muted);
    let sacred = Style::default()
        .fg(colors.foreground)
        .add_modifier(Modifier::BOLD);
    let translation_title = Style::default()
        .fg(colors.accent)
        .add_modifier(Modifier::BOLD);
    let translation_body = Style::default().fg(colors.accent);

    let (title, primary, translit, english): (String, String, Option<String>, Option<String>) =
        if state.collection == CollectionId::Quran {
            let surah = state.surahs.get(state.current_surah)?;
            let ayah = surah.ayahs.get(state.current_ayah)?;
            let translit = state
                .translit_cache
                .get(&surah.number)
                .and_then(|verses| verses.iter().find(|(id, _)| *id == ayah.number.to_string()))
                .map(|(_, text)| text.clone());
            (
                format!(
                    " {} {}:{} ",
                    surah.name_transliterated, surah.number, ayah.number
                ),
                ayah.arabic.clone(),
                translit,
                Some(ayah.english.clone()),
            )
        } else {
            let (verse_id, arabic) = state.active_verse()?;
            // Same verse identity as copy: Psalm shelves name the unit,
            // every other shelf names the book.
            let heading = match state.collection {
                CollectionId::Zabur => state.unit_label().to_string(),
                _ => state.current_book_label(),
            };
            (
                format!(" {heading} {}:{verse_id} ", state.current_chapter),
                arabic,
                None,
                state.active_verse_english(&verse_id),
            )
        };

    let mut lines = Vec::new();
    for row in wrap_or_blank(&primary, content_width) {
        lines.push(Line::styled(row, sacred).alignment(Alignment::Right));
    }
    if let Some(translit) = translit {
        lines.push(Line::from(""));
        lines.push(Line::styled("Transliteration", muted));
        push_wrapped(&mut lines, &translit, muted, content_width);
    }
    lines.push(Line::from(""));
    match english {
        Some(english) => {
            lines.push(Line::styled("Translation", translation_title));
            push_wrapped(&mut lines, &english, translation_body, content_width);
        }
        None => lines.push(Line::styled("(AR)", muted)),
    }
    Some((crate::output::sanitize_terminal_text(&title), lines))
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

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [vertical] = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .areas(area);
    let [centered] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(vertical);
    centered
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
        // At any width: Books is 22 and Units 24, so scripture starts at x=46.
        let scripture_x = 22 + 24;
        for y in 1..31 {
            for x in 0..scripture_x {
                let symbol = buffer[(x, y)].symbol();
                assert!(
                    !symbol.chars().any(is_arabic_presentation_form),
                    "Arabic escaped into cell ({x}, {y})"
                );
            }
        }

        assert_eq!(buffer[(21, 5)].symbol(), "│");
        assert_eq!(buffer[(22, 5)].symbol(), "│");
        assert_eq!(buffer[(45, 5)].symbol(), "│");
        assert_eq!(buffer[(46, 5)].symbol(), "│");
    }

    #[test]
    fn quran_panel_renders_the_whole_surah_with_verse_rows() {
        let mut state = test_state();
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();

        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        // Whole surah context at the top, then every ayah under it.
        assert_eq!(rendered.matches("Surah Al-Fatihah").count(), 1);
        assert!(rendered.contains("Ayah 1"));
        assert_eq!(
            rendered.matches("[All] praise is [due] to Allah").count(),
            1
        );
    }

    #[test]
    fn long_surah_scrolls_to_the_cursor_verse() {
        use crate::quran::Ayah;
        let mut state = test_state();
        // The bundled fallback is tiny (Al-Baqarah = 1 ayah), so synthesize
        // a long surah and jump to its last ayah: the cursor must scroll
        // the pane there.
        state.surahs[1].ayahs = (0..300u16)
            .map(|i| Ayah {
                number: 255 + i,
                arabic: "بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ".to_string(),
                english: "A long translated passage ".repeat(120),
                juz: 3,
            })
            .collect();
        state.current_surah = 1;
        state.current_ayah = state.surahs[1].ayahs.len() - 1;
        state.scroll_to_verse = true;
        state.active_panel = Panel::Scripture;
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(state.scripture_scroll > 0);
        let last = state.surahs[1].ayahs.last().unwrap().number;
        assert!(rendered.contains(&format!("Ayah {last}")));
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

    #[test]
    fn english_rows_land_on_exact_cells() {
        use unicode_segmentation::UnicodeSegmentation;
        use unicode_width::UnicodeWidthStr;

        // (terminal width, height, scripture panel x = 22 + 24 everywhere).
        for (width, height, panel_x) in [(120u16, 32u16, 46u16), (80u16, 24u16, 46u16)] {
            let mut state = test_state();
            state.language = Language::English;
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| draw(frame, &mut state)).unwrap();
            let buffer = terminal.backend().buffer();

            // Walk the English block grapheme by grapheme: every lead cell
            // must hold exactly the next source cluster. content_width
            // mirrors render_scripture: inner(width-panel_x-2) minus the
            // 1-column scrollbar reserve.
            let ayah = &state.surahs[0].ayahs[0];
            let content_width = (width - panel_x - 3) as usize;
            let arabic_rows =
                crate::rtl::terminal_lines(&ayah.arabic, content_width, state.rtl_mode).len();
            let english_rows = crate::wrap::wrap_text(&ayah.english, content_width);
            // Headline + blank + "Ayah 1" label push Arabic down by 3 more
            // rows than the old single-ayah layout; hairline + blank after.
            // Headline rows (it wraps on narrow panels) + blank + "Ayah 1"
            // label push Arabic down; hairline + blank after. Landing on the
            // first verse keeps scroll at 0, so the headline stays on top.
            let surah = &state.surahs[0];
            let headline = format!(
                "Surah {} ({}) · {} ayahs",
                surah.name_transliterated,
                if surah.is_meccan { "Meccan" } else { "Medinan" },
                surah.ayahs.len().max(1)
            );
            let headline_rows = crate::wrap::wrap_text(&headline, content_width).len() as u16;
            let first_y = 3 + headline_rows + 1 + 1 + arabic_rows as u16 + 2;
            let first_x = panel_x + 1;
            for (index, row) in english_rows.iter().enumerate() {
                let mut x = first_x;
                for grapheme in row.graphemes(true) {
                    assert_eq!(
                        buffer[(x, first_y + index as u16)].symbol(),
                        grapheme,
                        "{width}x{height}: cell mismatch in English row {index}"
                    );
                    x += UnicodeWidthStr::width(grapheme) as u16;
                }
            }

            // No rendered row may start with an orphaned combining mark.
            for y in 0..height {
                let mut row = String::new();
                for x in 0..width {
                    row.push_str(buffer[(x, y)].symbol());
                }
                let trimmed = row.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let first = trimmed.chars().next().unwrap();
                assert!(
                    !crate::wrap::is_combining_mark(first),
                    "{width}x{height} row {y} starts with orphaned mark: {trimmed:?}"
                );
            }
        }
    }

    #[test]
    fn juz_progress_counts_reading_order() {
        let state = test_state();
        // Fallback Al-Fatihah sits entirely in one Juz: position must climb
        // ayah by ayah within a stable total.
        let first = super::juz_position(&state.surahs, 0, 0);
        let second = super::juz_position(&state.surahs, 0, 1);
        assert!(first.1 >= 7, "Juz total covers Al-Fatihah: {first:?}");
        assert_eq!(first.1, second.1);
        assert_eq!(second.0, first.0 + 1);
    }

    #[test]
    fn progress_hairline_fills_from_the_left() {
        let mut state = test_state();
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let buffer = terminal.backend().buffer();
        let colors = Theme::Dark.colors();
        let (done, total) = super::reading_progress(&state);
        let expected_fill = (done * 120 / total.max(1)).min(120);
        assert!(expected_fill > 0 && expected_fill < 120);
        let mut accent_cells = 0;
        for x in 0..120 {
            let cell = &buffer[(x, 1)];
            if cell.symbol() == "─" && cell.fg == colors.accent {
                accent_cells += 1;
            }
        }
        assert_eq!(accent_cells, expected_fill);
        // Remainder of the row is the quiet border track, not background.
        let track = &buffer[(119, 1)];
        assert_eq!(track.symbol(), "─");
        assert_eq!(track.fg, colors.border);
    }

    #[test]
    fn tiny_terminal_never_panics() {
        // Extreme sizes with every overlay and language on: draw must hold
        // without panicking, even when panels collapse to a few cells.
        for (width, height) in [(40, 10), (30, 12), (20, 8)] {
            for language in [
                Language::English,
                Language::Arabic,
                Language::Hebrew,
                Language::Greek,
            ] {
                let mut state = test_state();
                state.language = language;
                state.search_mode = true;
                state.search_query = "mercy".to_string();
                state.show_help = true;
                state.scripture_scroll = u16::MAX;
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal.draw(|frame| draw(frame, &mut state)).unwrap();
            }
        }
    }

    #[test]
    fn tawrat_shelf_flattens_books_into_units_without_network() {
        use crate::alkotob::BookMeta;

        let mut state = test_state();
        state.collection = CollectionId::Tawrat;
        state.books = ["gen", "exo", "lev", "num", "deu"]
            .iter()
            .map(|id| BookMeta {
                id: id.to_string(),
                name: "x".to_string(),
                chapter_count: 10,
            })
            .collect();
        state.units = crate::collection::flat_units(CollectionId::Tawrat, &state.books);
        state.current_unit = 0;
        state.unit_list.select(Some(0));
        state.current_book = 0;
        state.current_chapter = 1;
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert_eq!(state.units.len(), 50);
        assert!(rendered.contains("Genesis 1"), "first unit missing");
        assert!(rendered.contains("Exodus 1"), "second book missing");
        assert!(rendered.contains("Tawrat"));
    }

    #[test]
    fn zabur_shelf_flattens_psalms_without_network() {
        use crate::alkotob::BookMeta;

        let mut state = test_state();
        state.collection = CollectionId::Zabur;
        state.books = vec![BookMeta {
            id: "psa".to_string(),
            name: "x".to_string(),
            chapter_count: 150,
        }];
        state.units = crate::collection::flat_units(CollectionId::Zabur, &state.books);
        state.current_unit = 22;
        state.unit_list.select(Some(22));
        state.current_book = 0;
        state.current_chapter = 23;
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert_eq!(state.units.len(), 150);
        assert!(rendered.contains("Psalm 23"));
        assert!(rendered.contains("Zabur"));
    }

    #[test]
    fn tanakh_and_greek_shelves_flatten_without_network() {
        use crate::alkotob::BookMeta;

        for (collection, ids, labels) in [
            (
                CollectionId::Tanakh,
                vec!["gen", "ps", "isa", "mal"],
                vec!["Genesis", "Psalms", "Isaiah", "Malachi"],
            ),
            (
                CollectionId::Greek,
                vec!["mat", "joh", "rev"],
                vec!["Matthew", "John", "Revelation"],
            ),
        ] {
            let mut state = test_state();
            state.collection = collection;
            state.books = ids
                .iter()
                .map(|id| BookMeta {
                    id: id.to_string(),
                    name: "x".to_string(),
                    // Small enough that every flattened unit row is visible.
                    chapter_count: 3,
                })
                .collect();
            state.units = crate::collection::flat_units(collection, &state.books);
            state.current_unit = 0;
            state.unit_list.select(Some(0));
            state.current_book = 0;
            state.current_chapter = 1;
            let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
            terminal.draw(|frame| draw(frame, &mut state)).unwrap();
            let rendered: String = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect();
            for book in labels {
                assert!(rendered.contains(book), "shelf missing {book}");
            }
            assert!(rendered.contains(collection.label()));
        }
    }

    #[test]
    fn entering_quran_resets_reader_state() {
        use crate::alkotob::BookMeta;

        let mut state = test_state();
        state.collection = CollectionId::Tawrat;
        state.books = vec![BookMeta {
            id: "gen".to_string(),
            name: "x".to_string(),
            chapter_count: 50,
        }];
        state.current_book = 2;
        state.current_chapter = 17;
        state.language = Language::Arabic;
        state.enter_collection(CollectionId::Quran, None, 1);
        assert_eq!(state.collection, CollectionId::Quran);
        assert!(state.books.is_empty());
        assert!(state.units.is_empty());
        assert!(state.chapter_cache.is_empty());
        assert!(state.status_msg.is_none());
    }

    #[test]
    fn collection_search_select_lands_on_chapter() {
        use crate::alkotob::BookMeta;
        use crate::collection::ChapterTexts;
        use crate::search::SearchResult;

        let mut state = test_state();
        state.collection = CollectionId::Tawrat;
        state.books = vec![BookMeta {
            id: "gen".to_string(),
            name: "x".to_string(),
            chapter_count: 50,
        }];
        state.units = crate::collection::flat_units(CollectionId::Tawrat, &state.books);
        state.chapter_cache.insert(
            ("gen".to_string(), 3),
            ChapterTexts {
                arabic: vec![("1".to_string(), "نُور".to_string())],
                english: Vec::new(),
            },
        );
        state.search_mode = true;
        state.search_results = vec![SearchResult {
            surah_number: 0,
            ayah_number: 0,
            surah_name: String::new(),
            english_text: String::new(),
            collection: "tawrat".to_string(),
            book_id: "gen".to_string(),
            chapter: 3,
            verse_id: "1".to_string(),
            display: "x".to_string(),
        }];
        state.search_list.select(Some(0));
        state.apply_action(crate::input::AppAction::Select);
        assert!(!state.search_mode);
        assert_eq!(state.current_book, 0);
        assert_eq!(state.current_chapter, 3);
        assert_eq!(state.active_panel, crate::app::Panel::Scripture);
    }

    #[test]
    fn english_chapter_marks_untranslated_verses() {
        use crate::alkotob::BookMeta;
        use crate::collection::ChapterTexts;

        let mut state = test_state();
        state.collection = CollectionId::Injil;
        state.language = Language::English;
        state.books = vec![BookMeta {
            id: "rom".to_string(),
            name: "x".to_string(),
            chapter_count: 16,
        }];
        state.chapter_cache.insert(
            ("rom".to_string(), 1),
            ChapterTexts {
                arabic: vec![("1".to_string(), "نُور".to_string())],
                english: Vec::new(),
            },
        );
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(rendered.contains("(AR)"));
    }

    #[test]
    fn verse_modal_shows_quran_verse_with_translation() {
        let mut state = test_state();
        state.language = Language::English;
        // Transliteration pre-seeded from the open surah: the opener must
        // not touch the network.
        let number = state.surahs[0].number;
        let ayah_id = state.surahs[0].ayahs[0].number.to_string();
        state
            .translit_cache
            .insert(number, vec![(ayah_id, "Bismillaahir".to_string())]);
        state.apply_action(crate::input::AppAction::ToggleVerse);
        assert!(state.show_verse);
        // j/k scroll the modal; the verse cursor stays put.
        state.apply_action(crate::input::AppAction::SelectDown);
        assert_eq!(state.current_ayah, 0);
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(rendered.contains("1:1"));
        assert!(rendered.contains("Transliteration"));
        assert!(rendered.contains("Bismillaahir"));
        assert!(rendered.contains("Translation"));
        state.apply_action(crate::input::AppAction::ToggleVerse);
        assert!(!state.show_verse);
    }

    #[test]
    fn verse_modal_marks_untranslated_verse() {
        use crate::alkotob::BookMeta;
        use crate::collection::ChapterTexts;

        // Tawrat has no English edition: the opener must not fetch either.
        let mut state = test_state();
        state.collection = CollectionId::Tawrat;
        state.language = Language::English;
        state.books = vec![BookMeta {
            id: "gen".to_string(),
            name: "Genesis".to_string(),
            chapter_count: 50,
        }];
        state.current_book = 0;
        state.current_chapter = 1;
        state.chapter_cache.insert(
            ("gen".to_string(), 1),
            ChapterTexts {
                arabic: vec![("1".to_string(), "نُور".to_string())],
                english: Vec::new(),
            },
        );
        state.apply_action(crate::input::AppAction::ToggleVerse);
        assert!(state.show_verse);
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(rendered.contains("1:1"));
        assert!(rendered.contains("(AR)"));
        assert!(!rendered.contains("Translation"));
    }

    #[test]
    fn scripture_marks_the_cursor_ayah() {
        let mut state = test_state();
        state.language = Language::English;
        state.active_panel = crate::app::Panel::Scripture;
        state.current_ayah = 1;
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let buffer = terminal.backend().buffer();
        let colors = Theme::Dark.colors();
        // Cursor label carries the shelf lists' ▶ + highlight; every
        // other label stays muted.
        let mut marked = 0;
        for y in 0..32 {
            let mut row = String::new();
            for x in 46..120 {
                row.push_str(buffer[(x, y)].symbol());
            }
            if row.contains("▶ Ayah 2") {
                marked += 1;
                assert_eq!(buffer[(47, y)].bg, colors.highlight);
                assert_eq!(buffer[(47, y)].fg, colors.accent);
            }
            assert!(!row.contains("▶ Ayah 1"), "only the cursor verse is marked");
        }
        assert_eq!(marked, 1);
    }

    #[test]
    fn reader_marks_the_cursor_verse() {
        use crate::alkotob::BookMeta;
        use crate::collection::ChapterTexts;

        let mut state = test_state();
        state.collection = CollectionId::Tawrat;
        state.language = Language::English;
        state.active_panel = crate::app::Panel::Scripture;
        state.books = vec![BookMeta {
            id: "gen".to_string(),
            name: "Genesis".to_string(),
            chapter_count: 50,
        }];
        state.current_book = 0;
        state.current_chapter = 1;
        state.current_ayah = 1;
        state.chapter_cache.insert(
            ("gen".to_string(), 1),
            ChapterTexts {
                arabic: vec![
                    ("1".to_string(), "نُور".to_string()),
                    ("2".to_string(), "هُدى".to_string()),
                ],
                english: Vec::new(),
            },
        );
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let buffer = terminal.backend().buffer();
        let colors = Theme::Dark.colors();
        let mut marked = 0;
        for y in 0..32 {
            let mut row = String::new();
            for x in 46..120 {
                row.push_str(buffer[(x, y)].symbol());
            }
            if row.contains("▶ Verse 2") {
                marked += 1;
                assert_eq!(buffer[(47, y)].bg, colors.highlight);
                assert_eq!(buffer[(47, y)].fg, colors.accent);
            }
            assert!(
                !row.contains("▶ Verse 1"),
                "only the cursor verse is marked"
            );
        }
        assert_eq!(marked, 1);
    }

    #[test]
    fn english_upgrade_truth_table() {
        use crate::app::AppState;
        use crate::collection::CollectionId;
        // Cached Arabic-only + English wanted + translated shelf → upgrade.
        assert!(AppState::needs_english_upgrade(
            CollectionId::Injil,
            true,
            true
        ));
        assert!(AppState::needs_english_upgrade(
            CollectionId::Bukhari,
            true,
            true
        ));
        // Shelves with no English edition never fetch.
        assert!(!AppState::needs_english_upgrade(
            CollectionId::Tawrat,
            true,
            true
        ));
        assert!(!AppState::needs_english_upgrade(
            CollectionId::Greek,
            true,
            true
        ));
        // English already cached, or Arabic wanted → nothing to do.
        assert!(!AppState::needs_english_upgrade(
            CollectionId::Injil,
            false,
            true
        ));
        assert!(!AppState::needs_english_upgrade(
            CollectionId::Injil,
            true,
            false
        ));
    }

    #[test]
    fn language_toggle_never_fetches_untranslated_shelves() {
        use crate::alkotob::BookMeta;
        use crate::collection::ChapterTexts;

        let mut state = test_state();
        state.collection = CollectionId::Tawrat;
        state.language = Language::English;
        state.books = vec![BookMeta {
            id: "gen".to_string(),
            name: "Genesis".to_string(),
            chapter_count: 50,
        }];
        state.current_book = 0;
        state.current_chapter = 1;
        state.chapter_cache.insert(
            ("gen".to_string(), 1),
            ChapterTexts {
                arabic: vec![("1".to_string(), "نُور".to_string())],
                english: Vec::new(),
            },
        );
        state.apply_action(crate::input::AppAction::CycleLanguage);
        // Tawrat forces Arabic; the Arabic-only rows survive untouched.
        assert!(matches!(state.language, Language::Arabic));
        let cached = state
            .chapter_cache
            .get(&("gen".to_string(), 1))
            .expect("cached chapter survives the toggle");
        assert!(cached.english.is_empty());
        assert!(state.chapter_error.is_none());
    }

    #[test]
    fn footer_hints_fit_their_gates() {
        use unicode_width::UnicodeWidthStr;
        for (width, scripture) in [
            (108u16, true),
            (108, false),
            (107, true),
            (64, true),
            (64, false),
            (40, false),
        ] {
            let hint = super::footer_hint(scripture, width);
            assert!(
                hint.width() <= width as usize,
                "{width} wide: footer hint {hint:?} overflows"
            );
            assert!(hint.contains('f'), "footer must name the verse focus modal");
        }
    }

    #[test]
    fn juz_mode_lists_juz_and_jumps_to_ayah() {
        let mut state = test_state();
        // Fallback Al-Baqarah 255 sits in Juz 3.
        state.current_surah = 1;
        state.current_ayah = 0;
        state.apply_action(crate::input::AppAction::ToggleJuzMode);
        assert!(state.juz_mode);
        assert_eq!(state.current_juz, 3);
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(rendered.contains("Juz 1"));
        assert!(rendered.contains("2:255"));
        // Scroll to the last row: Juz 30 must appear.
        state.current_juz = 30;
        state.surah_list.select(Some(29));
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(rendered.contains("Juz 30"));
    }

    #[test]
    fn transliteration_renders_cached_row() {
        let mut state = test_state();
        state.show_transliteration = true;
        state.translit_cache.insert(
            1,
            vec![("1".to_string(), "Bismillaahir Rahmaanir Raheem".to_string())],
        );
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(rendered.contains("Bismillaahir Rahmaanir Raheem"));
    }

    #[test]
    fn hadith_books_flatten_into_hadith_units_without_network() {
        use crate::alkotob::BookMeta;
        use crate::collection::ChapterTexts;

        let mut state = test_state();
        state.collection = CollectionId::Bukhari;
        state.books = vec![BookMeta {
            id: "bukhari".to_string(),
            name: "Sahih Bukhari".to_string(),
            chapter_count: 42,
        }];
        state.units = crate::collection::flat_units(CollectionId::Bukhari, &state.books);
        state.current_unit = 4;
        state.unit_list.select(Some(4));
        state.current_book = 0;
        state.current_chapter = 5;
        state.chapter_cache.insert(
            ("bukhari".to_string(), 5),
            ChapterTexts {
                arabic: vec![("5".to_string(), "نص".to_string())],
                english: vec![("5".to_string(), "text".to_string())],
            },
        );
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert_eq!(state.units.len(), 42);
        assert!(rendered.contains("Hadith 5"));
        assert!(rendered.contains("Hadith"));
        assert!(rendered.contains("Sahih Bukhari 5"));
    }

    #[test]
    fn longest_shelf_labels_render_untruncated() {
        // Regression: "Sahih Bukhari", "Sahih Muslim" and "Muwatta Malik"
        // (13 chars) were clipped by the 16/20-column Books panel once the
        // eight hadith books joined the shelf.
        for width in [80u16, 120u16] {
            let mut state = test_state();
            let mut terminal = Terminal::new(TestBackend::new(width, 32)).unwrap();
            terminal.draw(|frame| draw(frame, &mut state)).unwrap();
            let rendered: String = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect();
            for label in ["Sahih Bukhari", "Sahih Muslim", "Muwatta Malik"] {
                assert!(
                    rendered.contains(label),
                    "{width} wide: shelf label {label:?} truncated"
                );
            }
        }
    }

    #[test]
    fn units_column_steps_units_and_text_column_steps_verses() {
        use crate::alkotob::BookMeta;
        use crate::collection::ChapterTexts;

        let mut state = test_state();
        state.collection = CollectionId::Tawrat;
        state.books = ["gen", "exo"]
            .iter()
            .map(|id| BookMeta {
                id: id.to_string(),
                name: "x".to_string(),
                chapter_count: 10,
            })
            .collect();
        state.units = crate::collection::flat_units(CollectionId::Tawrat, &state.books);

        // Column 2: two steps down lands on "Genesis 2" and derives the
        // book/chapter behind it.
        state.apply_action(crate::input::AppAction::PanelRight);
        state.apply_action(crate::input::AppAction::SelectDown);
        state.apply_action(crate::input::AppAction::SelectDown);
        assert_eq!(state.current_unit, 2);
        assert_eq!(state.current_chapter, 3);
        assert_eq!(state.units[state.current_unit].label, "Genesis 3");

        // Column 3: the verse cursor moves verse by verse inside the unit.
        state.chapter_cache.insert(
            ("gen".to_string(), 3),
            ChapterTexts {
                arabic: vec![
                    ("1".into(), "أ".into()),
                    ("2".into(), "ب".into()),
                    ("3".into(), "ج".into()),
                ],
                english: Vec::new(),
            },
        );
        state.apply_action(crate::input::AppAction::PanelRight);
        state.apply_action(crate::input::AppAction::SelectDown);
        assert_eq!(state.current_ayah, 1);
        assert!(state.scroll_to_verse);
        state.apply_action(crate::input::AppAction::SelectDown);
        state.apply_action(crate::input::AppAction::SelectDown);
        assert_eq!(state.current_ayah, 3 - 1);
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
            collection: CollectionId::Quran,
            books: Vec::new(),
            current_book: 0,
            current_chapter: 1,
            units: Vec::new(),
            current_unit: 0,
            library_list: {
                let mut list = ListState::default();
                list.select(Some(0));
                list
            },
            unit_list: ListState::default(),
            chapter_cache: std::collections::HashMap::new(),
            chapter_error: None,
            hadith_bulk: std::collections::HashMap::new(),
            scroll_to_verse: true,
            juz_mode: false,
            current_juz: 1,
            show_transliteration: false,
            translit_cache: std::collections::HashMap::new(),
            bookmark_error: None,
            bookmarked: false,
            scripture_scroll: 0,
            help_scroll: 0,
            show_verse: false,
            verse_scroll: 0,
            rtl_mode: crate::rtl::RtlMode::Visual,
            status_started: None,
        }
    }

    fn is_arabic_presentation_form(character: char) -> bool {
        matches!(character as u32, 0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
    }
}
