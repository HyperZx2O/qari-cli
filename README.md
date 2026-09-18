# qari-cli

A full-screen Quran reader for the terminal, with Arabic, Sahih International English, and Muhiuddin Khan Bengali text.

## Install

```sh
npm install -g qari-cli
```

Or on Linux/macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/HyperZx2O/qari-cli/main/install.sh | bash
```

On first use, the application downloads and caches all three Quran editions. A small bundled selection remains available if that download fails.

## Usage

```sh
qari                         # interactive TUI
qari intro                   # replay the startup animation
qari --intro                 # force the animation before reading
qari --no-intro              # skip the animation
qari read 2:255              # specific ayah
qari read "Al-Baqarah"       # full surah
qari search mercy            # top fuzzy matches
qari random                  # random ayah
qari today                   # deterministic ayah of the day
qari pray                    # configured coordinates
qari pray --city Dhaka --country BD
qari hadith                  # Sahih al-Bukhari hadith of the day
```

Subcommand output contains no ANSI escapes when piped.

## Interactive keys

| Key | Action |
|---|---|
| `h`/`l`, `←`/`→` | Switch panel |
| `j`/`k`, `↓`/`↑` | Navigate lists or scroll Scripture/help |
| `/` | Live search |
| `Enter` | Select result/panel |
| `y` or `c` | Copy ayah |
| `b` | Toggle bookmark |
| `t` | Cycle theme |
| `v` | Cycle EN/BN/AR |
| `?` | Help |
| `qq` | Quit |
| `Ctrl+C` | Quit immediately |

## Features

- Three-panel Surah, Ayah, and Scripture browser
- Animated first-launch intro while Quran data loads
- Arabic, English, and Bengali text cached for offline use
- Live fuzzy search, three themes, bookmarks, and session persistence
- Prayer times with Karachi/Hanafi defaults and optional city lookup
- Pipe-friendly read, search, random, today, prayer, and hadith commands

Unicode-capable terminals such as Windows Terminal, iTerm2, Kitty, Alacritty, and GNOME Terminal are supported. Arabic/Bengali glyph quality depends on installed fonts and terminal shaping support.

The full-screen TUI shapes Arabic into terminal-safe visual order and omits combining recitation marks so terminal cursor positions remain stable. Plain CLI output and copied ayahs retain the original Uthmani text with all marks.

Configuration is stored in `~/.config/qari-cli/config.toml` (or the platform-equivalent config directory). Set `reduced_motion = true` to disable the animated intro. Arabic rendering defaults to `rtl_mode = "auto"`; use `"logical"` for terminals with native BiDi support or `"visual"` for traditional LTR terminal grids. `QARI_RTL_MODE` can override this per launch.

Built with Rust, Ratatui, Crossterm, alquran.cloud, AlAdhan, and the fawazahmed0 hadith dataset.

## License

MIT
