# Project Specification
<!-- TEMPLATE VERSION: 2.0 -->

---

## 1. Project Identity

| Field | Value |
|-------|-------|
| **Project Name** | qari-cli |
| **One-line Description** | A beautiful Quran TUI for Muslim developers — read, search, and reflect in your terminal. |
| **Project Type** | `cli` |
| **Hackathon / Context** | Personal project — spiritual counterpart to `christ-cli` (github.com/whoisyurii/christ-cli) |
| **Deadline** | N/A — personal project, no hard deadline |
| **Team Size** | 1 (solo) |
| **Primary Language(s)** | Rust (edition 2021) |

---

## 2. Problem Statement

Every existing Quran CLI tool (`vanillaiice/quran-cli`, `codewithnuh/quran-cli`, `omeiirr/quran-cli`) is a minimal lookup utility — a flag-based command that prints text and exits. None offer a full-screen interactive TUI comparable to what `christ-cli` gives Bible readers: a browsable 3-panel interface, themes, live search, session persistence, and a polished experience that makes spending time in the terminal feel intentional.

Muslim developers who live in the terminal have no tool that matches the quality of `christ-cli`. `qari-cli` fills that gap with the same TUI depth, adding Islamic-specific features (Juz navigation, transliteration, offline prayer times, Bengali translation) drawn from `Ayatika` — a prior C/RayLib Quran desktop app by the same author.

---

## 3. Users

| User Type | Goal | Interaction Mode | Success Signal |
|-----------|------|-----------------|----------------|
| Muslim developer | Read Quran during work breaks without leaving the terminal | Full-screen TUI + subcommands | Navigates surahs/ayahs, reads Arabic + translation side by side |
| Bangladeshi developer | Read in Bengali translation | TUI language toggle | Bengali text renders correctly in the scripture panel |
| Any user (quick lookup) | Fetch a specific ayah fast | CLI subcommand (`qari read 2:255`) | Correct ayah printed to stdout and exits |
| Any user (prayer times) | Check today's prayer times without opening a browser | CLI subcommand (`qari pray`) | All 5 prayer times printed for configured location |

---

## 4. Core Features

### Must-Have (MVP — project fails without these)

- [ ] **Full-screen 3-panel TUI** — Surahs panel | Ayahs panel | Scripture panel (Arabic + translation), navigable with arrow keys and Vim keys (j/k/h/l), matching christ-cli's panel layout.
- [ ] **Offline Quran data** — Arabic (Uthmani), English (Sahih International), and Bengali (Muhiuddin Khan) downloaded on first use and cached locally; a bundled core selection covers first-run network failure.
- [ ] **114 Surah metadata** — Name (Arabic + transliterated), meaning, Makki/Madani label, revelation order, ayah count — ported from `Ayatika`'s `surah_meta.c`; embedded as a Rust const array, never fetched.
- [ ] **`qari read <surah>:<ayah>`** — Read a specific ayah by reference (e.g., `qari read 2:255`, `qari read Al-Baqarah 2:255`). Prints to stdout (pipe-friendly plain text when piped, rich TUI when interactive).
- [ ] **`qari search <query>`** — Fuzzy full-text search over English translation + surah name (surah name gets +500 score boost), top 15 results. Algorithm ported from `Ayatika`'s `search.c` (`fts_fuzzy_match` logic). Minimum 2 characters.
- [ ] **`qari random`** — Print a random ayah (Arabic + translation).
- [ ] **`qari today`** — Ayah of the day, seeded by `day_of_year % 6236` (same formula as Ayatika).
- [ ] **Session persistence** — Remembers last surah/ayah position, selected language, and theme across sessions. Stored in `~/.config/qari-cli/config.toml`.
- [ ] **Themes** — At least 3 themes: Dark (default), Light, Sepia. Toggled in TUI with `t`.

### Nice-to-Have (only if MVP is done and time remains)

- [ ] **`qari pray`** — Today's 5 prayer times (Fajr, Dhuhr, Asr, Maghrib, Isha) computed offline using PrayTime algorithm (Karachi method, Hanafi Asr factor 2), from configured lat/lng. Ported from `Ayatika`'s `prayer.c`.
- [ ] **`qari hadith`** — Hadith of the day, seeded by `day_of_year % totalHadiths`, fetched from `fawazahmed0/hadith-api` jsDelivr CDN and cached to `~/.local/share/qari-cli/hadith.json`.
- [ ] **Bookmarks** — Save/delete ayah bookmarks with optional tag and note, persisted to SQLite (`~/.local/share/qari-cli/bookmarks.db`). Schema ported directly from `Ayatika`'s `db.c`. Toggle with `b` in TUI.
- [ ] **Juz navigation** — Browse by Juz (1–30) in addition to Surah. `J` key in TUI switches to Juz mode.
- [ ] **Transliteration toggle** — Show/hide romanized Arabic transliteration below each ayah. `i` key in TUI.
- [ ] **Online translation fallback** — If internet available, fetch any of 440+ editions from `alquran.cloud` REST API (unauthenticated). `v` key opens edition picker in TUI.
- [ ] **`qari pray --city <name>`** — Override location for prayer times using AlAdhan API (`api.aladhan.com/v1/timingsByCity`).
- [ ] **npm distribution** — Wrap binary in an npm package for `npm install -g qari-cli`, mirroring christ-cli's distribution.

### Explicitly Out of Scope

- Audio recitation playback (requires system audio dependencies; out of scope for a terminal tool)
- Tafsir commentary (data too large to bundle; no clean free API)
- Tajweed colouring (requires font-level markup; not feasible in terminal)
- Windows title bar theming (Ayatika-specific GUI feature)
- RayLib, GUI, or any graphical rendering
- Web or mobile UI of any kind
- User accounts or cloud sync

---

## 5. Tech Stack

| Layer | Technology | Version / Config | Reason for Choice |
|-------|-----------|-----------------|-------------------|
| Language | Rust | edition 2021, stable toolchain | Single binary output, ratatui ecosystem, same stack as christ-cli |
| TUI framework | ratatui | 0.29 | The standard Rust TUI library; christ-cli uses it |
| Terminal backend | crossterm | 0.28 | Cross-platform (macOS, Linux, Windows); ratatui default |
| CLI parsing | clap | 4.x, derive feature | Industry standard; derive macros reduce boilerplate |
| HTTP client | reqwest | 0.12, blocking feature | Simple blocking HTTP for data fetch on first run |
| JSON parsing | serde + serde_json | 1.x | Standard Rust JSON; needed for alquran.cloud responses |
| Config | toml + serde | 0.8 | Human-readable config file, same pattern as Cargo |
| Bookmarks DB | rusqlite | 0.31, bundled feature | SQLite; schema ported from Ayatika; `bundled` compiles SQLite in |
| Fuzzy search | fuzzy-matcher | 0.3 | Rust port of `fts_fuzzy_match`; replaces Ayatika's C header |
| Clipboard | arboard | 3.x | Cross-platform clipboard; mirrors christ-cli |
| Dev tooling | cargo, cargo-dist | latest | Build + cross-platform binary distribution |
| Distribution | npm wrapper + install.sh | — | Mirrors christ-cli's install UX exactly |
| Hosting | GitHub Releases (cargo-dist) | — | Free, standard for Rust CLIs |

---

## 6. Architecture & Data Flow

### First Run
User runs `qari` → binary checks `~/.local/share/qari-cli/quran_ar.json`. If missing → fetches `quran-uthmani`, `en.sahih`, `bn.bengali` editions from `api.alquran.cloud/v1/quran/{edition}` → writes 3 JSON files to `~/.local/share/qari-cli/`. If fetch fails → falls back to minimal bundled dataset (Al-Fatiha + Ayat al-Kursi) with status message `"Offline mode — connect to fetch full data"`.

### Subsequent Runs
Binary reads cached JSON from disk → deserialises into `Vec<Surah>` → launches TUI (or executes subcommand and exits).

### TUI Flow
`App` struct holds state: `selected_surah`, `selected_ayah`, `active_panel`, `language`, `theme`, `search_query`. `crossterm` event loop reads key events → `handle_input()` mutates state → `ratatui::Terminal::draw()` renders three `Block` widgets per frame. On quit (`qq`) → `save_config()` writes `lastSurah`, `lastAyah`, `language`, `theme` to config.

### Subcommand Flow
`qari read 2:255` → clap parses → load data from cache (or fetch) → find ayah → print to stdout (plain text if piped, styled if TTY) → exit 0.

### Key Entities / Data Structures

- `Surah` — `number: u8`, `name_arabic: String`, `name_transliterated: String`, `name_meaning: String`, `is_meccan: bool`, `revelation_order: u8`, `ayah_count: u16`, `ayahs: Vec<Ayah>`
- `Ayah` — `number: u16`, `arabic: String`, `english: String`, `bengali: String`, `juz: u8`, `page: u16`
- `SurahMeta` — const array of 114 entries embedded at compile time (ported from `surah_meta.c`); fields: number, name, meaning, is_meccan, revelation_order, ayah_count
- `AppState` — `current_surah: usize`, `current_ayah: usize`, `active_panel: Panel`, `language: Language`, `theme: Theme`, `search_mode: bool`, `search_query: String`, `bookmarks: Vec<Bookmark>`
- `Bookmark` — `surah_id: u8`, `ayah_id: u16`, `tag: String`, `note: String`, `timestamp: i64` (SQLite schema from Ayatika)
- `Config` — `last_surah: u8`, `last_ayah: u16`, `language: String`, `theme: String`, `latitude: f64`, `longitude: f64`, `calc_method: u8`

---

## 7. System Boundaries

| Concern | Decision |
|---------|----------|
| **What this system does NOT do** | No audio, no tafsir, no user accounts, no web UI, no push notifications |
| **External dependencies** | `api.alquran.cloud` (Quran data, first run only) — fallback: bundled minimal dataset. `fawazahmed0/hadith-api` jsDelivr CDN (hadith, nice-to-have) — fallback: skip feature with message. `api.aladhan.com` (city prayer times, nice-to-have) — fallback: offline PrayTime calculation |
| **Data stored** | `~/.local/share/qari-cli/quran_ar.json`, `quran_en.json`, `quran_bn.json`, `hadith.json`, `bookmarks.db`. `~/.config/qari-cli/config.toml` |
| **Data never stored** | No PII, no API keys, no secrets. All data is public Islamic scripture |
| **Authentication** | None — all APIs used are fully unauthenticated and free |
| **Rate limits / quotas** | alquran.cloud: no stated rate limit; data fetched once and cached. jsDelivr CDN: no rate limit |

---

## 8. Constraints

| Constraint | Value | Impact |
|-----------|-------|--------|
| Time budget | Personal project — no deadline | Build MVP first, nice-to-haves after |
| Infra budget | $0 — GitHub Releases only | Binary distribution only; no server |
| Binary size | Target ≤ 15MB (christ-cli is ~5MB) | Bundle only 3 editions offline; fetch others on demand |
| Arabic rendering | Terminal must support Unicode; no font embedding | Works in Windows Terminal, iTerm2, Kitty, Alacritty. Warn if terminal reports no Unicode support |
| Bengali rendering | Same Unicode requirement | Same caveat; test with Windows Terminal + Bangla font |
| Rust skill | Author has ML/backend experience, Rust is new | Use derive macros; keep modules small; refer to christ-cli source as implementation reference |
| No `.cpp` files | N/A for Rust — not a constraint here | — |

---

## 9. Acceptance Criteria

| Feature | Passing Condition (EARS) |
|---------|--------------------------|
| 3-panel TUI | WHEN user runs `qari` with no args THE system SHALL open a full-screen TUI with Surahs, Ayahs, and Scripture panels visible and navigable within 1 second |
| Offline Quran data | WHEN user runs any command with no internet connection THE system SHALL still display Arabic + English ayahs from cached or bundled data |
| 114 Surah metadata | WHEN user browses the Surahs panel THE system SHALL display all 114 surahs with name, transliteration, and Makki/Madani badge |
| `qari read` | WHEN user runs `qari read 2:255` THE system SHALL print Ayat al-Kursi in Arabic and English to stdout and exit 0 |
| `qari read` pipe-friendly | WHEN user runs `qari read 2:255 \| wc -l` THE system SHALL output plain text with no ANSI escape codes |
| `qari search` | WHEN user runs `qari search "mercy"` THE system SHALL return up to 15 ayah results ranked by fuzzy score, with surah name results boosted |
| `qari random` | WHEN user runs `qari random` THE system SHALL print a random ayah (Arabic + English) that is different from the previous call with high probability |
| `qari today` | WHEN user runs `qari today` on the same calendar day twice THE system SHALL return the same ayah both times |
| Session persistence | WHEN user closes TUI and reopens it THE system SHALL restore the last surah/ayah position, language, and theme |
| Themes | WHEN user presses `t` in TUI THE system SHALL cycle to the next theme and re-render immediately |
| Bengali translation | WHEN user selects Bengali (`bn`) language THE system SHALL display Bengali translation text in the Scripture panel for every ayah |

**Definition of Done (whole project):**
- [ ] All Must-Have acceptance criteria pass
- [ ] No panic on happy path with realistic input (all 114 surahs, 6236 ayahs)
- [ ] Binary runs on macOS, Linux, and Windows without extra dependencies
- [ ] README covers install + first run in ≤ 5 steps
- [ ] `qari read 2:255` works fully offline after first data fetch
- [ ] TUI exits cleanly on `qq` and `Ctrl+C` with terminal restored

---

## 10. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Arabic Unicode not rendering in user's terminal | M | M | Print warning on launch if terminal reports limited Unicode; document supported terminals in README |
| Bengali font missing on user system | M | L | Graceful fallback: show `[Bengali not available — install a Unicode font]` in scripture panel |
| alquran.cloud API down on first run | L | H | Bundle Al-Fatiha + first 10 ayahs of Al-Baqarah as hardcoded fallback; show clear message to retry |
| Rust learning curve slowing development | M | M | Reference christ-cli source directly for ratatui patterns; keep modules ≤ 200 lines each |
| Binary size bloat from bundled JSON | M | L | Compress JSON with `include_bytes!` + decompress at runtime; or fetch-only with good cache |
| ratatui version churn (breaking changes) | L | M | Pin exact versions in `Cargo.lock`; check christ-cli's pinned versions |
| SQLite bundled compile failing on Windows | L | M | `rusqlite` with `bundled` feature handles this; test early |

---

## 11. File & Folder Structure

```
qari-cli/
├── src/
│   ├── main.rs              # Entry point: clap dispatch → subcommand or TUI
│   ├── app.rs               # AppState struct + event loop
│   ├── ui.rs                # ratatui layout: 3-panel render, search overlay, help overlay
│   ├── quran.rs             # Surah/Ayah data model + deserialisation
│   ├── surah_meta.rs        # const array of 114 SurahMeta entries (ported from surah_meta.c)
│   ├── data.rs              # load_from_cache(), fetch_and_cache(), bundled fallback
│   ├── search.rs            # Fuzzy search over ayahs (fts_fuzzy_match logic, surah name boost)
│   ├── prayer.rs            # Offline PrayTime calculation (ported from prayer.c)
│   ├── bookmarks.rs         # SQLite CRUD via rusqlite (schema from Ayatika db.c)
│   ├── config.rs            # Load/save ~/.config/qari-cli/config.toml
│   ├── theme.rs             # Theme enum + color tokens (Dark, Light, Sepia)
│   ├── input.rs             # Key event → AppAction mapping (Vim + arrow modes)
│   └── commands/
│       ├── read.rs          # `qari read <ref>` — parse ref, print ayah
│       ├── search.rs        # `qari search <query>` — CLI search output
│       ├── random.rs        # `qari random`
│       ├── today.rs         # `qari today` (day_of_year % 6236)
│       ├── pray.rs          # `qari pray` (offline) + `--city` (AlAdhan API)
│       └── hadith.rs        # `qari hadith` (fawazahmed0 CDN, day-seeded)
├── data/
│   └── fallback.json        # Bundled minimal dataset (Al-Fatiha + Ayat al-Kursi)
├── assets/
│   └── demo.gif             # (generated) README demo
├── npm/
│   ├── package.json         # npm wrapper package (mirrors christ-cli pattern)
│   └── install.js           # Downloads platform binary from GitHub Releases
├── .github/
│   └── workflows/
│       └── release.yml      # cargo-dist: build + publish binaries on tag push
├── tests/
│   ├── test_read.rs         # `qari read 2:255` output test
│   ├── test_search.rs       # fuzzy search scoring + surah boost
│   ├── test_today.rs        # day-seeded ayah determinism
│   ├── test_surah_meta.rs   # all 114 entries present and valid
│   └── test_prayer.rs       # PrayTime output for Dhaka (known values)
├── docs/
│   └── spec.md              # This file
├── Cargo.toml
├── Cargo.lock               # Committed — pinned deps
├── install.sh               # curl install script (mirrors christ-cli)
├── .gitignore
└── README.md
```

**Naming conventions:**
- Files: `snake_case`
- Structs / Enums: `PascalCase`
- Constants: `UPPER_SNAKE_CASE`
- Functions: `snake_case`

---

## 12. TUI Keybindings Reference

| Key | Action |
|-----|--------|
| `h` / `←` | Move focus to left panel |
| `l` / `→` | Move focus to right panel |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `Enter` | Select surah / ayah |
| `/` | Open live search |
| `y` | Copy selected ayah to clipboard |
| `t` | Cycle theme |
| `v` | Pick translation/language |
| `J` | Toggle Juz navigation mode (nice-to-have) |
| `i` | Toggle transliteration (nice-to-have) |
| `b` | Toggle bookmarks panel (nice-to-have) |
| `?` | Help overlay |
| `qq` | Quit |

---

## 13. Change Log

| Version | Date | What Changed | Why | Affected Plans |
|---------|------|-------------|-----|----------------|
| v1.0 | 2026-09-18 | Initial spec | — | All |
```

Key decisions recorded:
- Rust chosen over Go/Python/C to match christ-cli's stack and single-binary output
- alquran.cloud chosen as data source (same as Ayatika)
- surah_meta.rs, search.rs, prayer.rs, bookmarks.rs logic ported from Ayatika C codebase
- Audio, tafsir, tajweed explicitly out of scope
- npm + install.sh distribution mirrors christ-cli exactly
