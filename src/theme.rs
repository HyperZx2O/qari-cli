use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
    Sepia,
}

pub struct ThemeColors {
    pub background: Color,
    pub surface: Color,
    pub foreground: Color,
    pub muted: Color,
    pub accent: Color,
    pub border: Color,
    pub highlight: Color,
}

impl Theme {
    pub fn colors(self) -> ThemeColors {
        match self {
            Self::Dark => ThemeColors {
                background: Color::Rgb(15, 15, 20),
                surface: Color::Rgb(25, 25, 35),
                foreground: Color::Rgb(220, 215, 200),
                muted: Color::Rgb(120, 115, 100),
                accent: Color::Rgb(200, 160, 80),
                border: Color::Rgb(60, 55, 45),
                highlight: Color::Rgb(40, 35, 55),
            },
            Self::Light => ThemeColors {
                background: Color::Rgb(252, 250, 245),
                surface: Color::Rgb(240, 238, 230),
                foreground: Color::Rgb(30, 25, 20),
                muted: Color::Rgb(140, 130, 110),
                accent: Color::Rgb(140, 90, 30),
                border: Color::Rgb(200, 190, 170),
                highlight: Color::Rgb(225, 220, 200),
            },
            Self::Sepia => ThemeColors {
                background: Color::Rgb(30, 22, 12),
                surface: Color::Rgb(45, 33, 18),
                foreground: Color::Rgb(210, 190, 155),
                muted: Color::Rgb(130, 110, 75),
                accent: Color::Rgb(195, 145, 60),
                border: Color::Rgb(80, 60, 30),
                highlight: Color::Rgb(60, 45, 22),
            },
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Sepia,
            Self::Sepia => Self::Dark,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::Sepia => "Sepia",
        }
    }

    pub fn from_config(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "light" => Self::Light,
            "sepia" => Self::Sepia,
            _ => Self::Dark,
        }
    }
}
