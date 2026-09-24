# qari-cli — Quran CLI and Terminal Scripture Reader

[![CI](https://github.com/HyperZx2O/qari-cli/actions/workflows/release.yml/badge.svg)](https://github.com/HyperZx2O/qari-cli/actions/workflows/release.yml)
[![Release](https://img.shields.io/github/v/release/HyperZx2O/qari-cli?label=release)](https://github.com/HyperZx2O/qari-cli/releases/latest)
[![npm](https://img.shields.io/npm/v/qari-cli?label=npm)](https://www.npmjs.com/package/qari-cli)
[![License](https://img.shields.io/github/license/HyperZx2O/qari-cli)](https://github.com/HyperZx2O/qari-cli/blob/main/LICENSE)
[![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-2f6f5f)](https://github.com/HyperZx2O/qari-cli/releases/latest)

A Quran CLI and terminal scripture reader: fourteen shelves — six revelations plus eight hadith books — in one static Rust binary. A three-column TUI for reading, and a pipe-friendly CLI for everything else.

Read, search, and browse the Quran, Hadith, and other scriptures in your terminal with English translations where available, offline caching, prayer times, bookmarks, and keyboard navigation.

No commentary, no audio, no web or mobile UI. Primary texts with English where a translation exists, and an honest marker where one does not.

![qari-cli demo: library tour, verse focus, search, themes, and CLI](assets/demo.gif)

## Install

```sh
npm install -g qari-cli
```

Works with Bun too:

```sh
bun add -g qari-cli
bunx qari-cli today
```

Or on Linux/macOS, download and review a versioned installer first:

```sh
curl --proto '=https' --tlsv1.2 -fL -o install.sh \
  https://raw.githubusercontent.com/HyperZx2O/qari-cli/v0.1.0/install.sh
less install.sh
bash install.sh
```

The installer verifies the release checksum before extracting the binary and installs to
`~/.local/bin` by default. Set `INSTALL_DIR` to choose another user-writable directory.

Prebuilt binaries for Windows, macOS, and Linux (including Windows ARM64) are attached to every GitHub release.

On first use, the app downloads what it needs and caches it on disk: both Quran editions plus any Alkotob chapter or hadith book you open. A small bundled Quran selection keeps the reader usable if a download fails.

## The library

The TUI is one continuous three-column shelf — no mode key:

1. **Books** — the fourteen shelves: Quran, Tawrat, Zabur, Injil, Tanakh, Greek NT, then Sahih Bukhari, Sahih Muslim, Abu Dawud, Tirmidhi, Nasa'i, Ibn Majah, Muwatta Malik, Nawawi 40.
2. **Units** — the open book's readings, flattened one level deep: surahs, `Genesis 1 … Deuteronomy 34`, `Psalm 1 … 150`, `Hadith 1 … 7563`.
3. **Text** — the whole unit as one scroll, with a verse cursor that `j`/`k` moves verse to verse. The cursor verse is marked with `▶`; the pane follows it.

Selecting a row in columns 1 and 2 opens it immediately — `Enter` only moves focus.

## Collections and languages

| Shelf | Source | Text | English |
|---|---|---|---|
| Quran | alquran.cloud (Uthmani) | Arabic | Sahih International, always |
| Tawrat | Alkotob `tawrat` (5 books) | Arabic | — |
| Zabur | Alkotob `zabur` (150 psalms) | Arabic | — |
| Injil | Alkotob `injil` (27 books) | Arabic | `injilen` for the Gospels + Acts; `(AR)` elsewhere |
| Tanakh | Alkotob `wlc` (39 books) | Hebrew | — |
| Greek NT | Alkotob `gnt` (27 books) | Greek | — |
| 8 hadith books | fawazahmed0 bulk editions | Arabic | English per book where translated |

Press `v` to cycle the open shelf's languages (Tawrat and Zabur stay Arabic; Tanakh stays Hebrew; Greek stays Greek). Verses with no translation show `(AR)` instead of a gap — in the text, in search, and in the focus view. A chapter first opened in Arabic picks up its English automatically once you toggle. Quran transliteration rides along on `i`, and Juz navigation on `J`.

## Usage

```sh
qari                         # interactive TUI (Quran by default)
qari intro                   # replay the startup animation
qari --no-intro              # skip the animation
qari read 2:255              # one Quran ayah
qari read "Al-Baqarah"       # full surah
qari read "tawrat gen 1:3"   # Tawrat verse (or "Genesis 1")
qari read zabur 23           # whole psalm (or "zabur 23:5")
qari read "injil joh 3:16"   # Injil verse
qari read "bukhari 402"      # one hadith by number
qari search mercy            # top fuzzy matches (Quran)
qari search --collection tawrat "النور"  # cached chapters only
qari search --collection bukhari "صلاة"
qari random                  # random ayah (or: random bukhari)
qari today                   # deterministic ayah of the day
qari pray                    # configured coordinates
qari pray --city Dhaka --country BD
qari hadith                  # Bukhari hadith of the day
qari hadith bukhari          # list one hadith book (muslim, abudawud,
qari hadith bukhari 5        #   tirmidhi, nasai, ibnmajah, malik, nawawi)
qari books                   # shelves and Alkotob editions
qari books tawrat            # books of one revelation
qari books --edition tawrat --book gen --chapter 1
```

Search covers the open shelf's downloaded chapters only — it never touches the network.CLI output contains no ANSI escapes when piped. Each hadith book downloads its Arabic and English bulk once (several MB on first open) and reads offline after that.

## Interactive keys

| Key | Action |
|---|---|
| `h`/`l`, `←`/`→` | Move between Books, Units, Text columns |
| `j`/`k`, `↓`/`↑` | Navigate the active column (verse cursor in Text) |
| `/` | Live search |
| `Enter` | Move into the next column |
| `y` or `c` | Copy verse |
| `b` | Toggle bookmark |
| `t` | Cycle theme |
| `v` | Cycle language (per collection) |
| `f` | Verse focus: transliteration plus highlighted translation |
| `i` | Transliteration (Quran) |
| `J` | Juz navigation (Quran) |
| `?` | Help |
| `qq` | Quit |
| `Ctrl+C` | Quit immediately |
| `Ctrl+L` | Redraw screen |

## Features

- Three-column library: Books, Units, and full-unit text with a marked verse cursor
- Verse focus modal (`f`) with transliteration and highlighted translation
- Six revelations plus eight hadith books, one shelf each
- Live fuzzy search over downloaded text, three themes, string-key bookmarks with legacy migration, session persistence
- Animated first-launch intro while Quran data loads in the background
- Offline-first disk cache with bundled fallback; honest `(AR)` markers, never silent gaps
- Prayer times with Karachi/Hanafi defaults and optional city lookup
- Pipe-friendly `read`, `search`, `random`, `today`, `pray`, `hadith`, and `books` commands

## Data and offline behavior

Runtime data lives outside the repo in the platform data directory (`%LOCALAPPDATA%\qari-cli\` on Windows): Alkotob chapters cached per file as you open them, hadith bulks per book and language, bookmarks, and the Quran editions. Deleting that directory resets to freshly-downloaded state; the bundled fallback covers the gap if the network is down.

## Configuration

Settings live in `~/.config/qari-cli/config.toml` (or the platform-equivalent config directory): language, theme, transliteration, last position, bookmarks, and prayer coordinates. Notable flags:

- `reduced_motion = true` disables the animated intro.
- `rtl_mode = "auto"` shapes Arabic for the terminal; use `"logical"` on terminals with native BiDi support or `"visual"` for traditional LTR terminal grids.

## Terminals and fonts

Any Unicode-capable terminal works — Windows Terminal, iTerm2, Kitty, Alacritty, GNOME Terminal. Arabic glyph quality depends on installed fonts and terminal shaping support. The fullscreen TUI shapes Arabic into terminal-safe visual order so cursor positions stay stable; plain CLI output and copied verses keep the original Uthmani text with all marks. The interface is fully keyboard-operable.

## Built with

Rust, Ratatui, Crossterm, alquran.cloud, the Alkotob API, AlAdhan, and the fawazahmed0 hadith dataset.

## License

MIT
