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
    pub danger: Color,
}

impl Theme {
    pub fn colors(self) -> ThemeColors {
        match self {
            Self::Dark => ThemeColors {
                background: Color::Rgb(15, 15, 20),
                surface: Color::Rgb(25, 25, 35),
                foreground: Color::Rgb(220, 215, 200),
                muted: Color::Rgb(170, 162, 142),
                accent: Color::Rgb(200, 160, 80),
                border: Color::Rgb(115, 105, 85),
                highlight: Color::Rgb(40, 35, 55),
                danger: Color::Rgb(240, 125, 120),
            },
            Self::Light => ThemeColors {
                background: Color::Rgb(252, 250, 245),
                surface: Color::Rgb(240, 238, 230),
                foreground: Color::Rgb(30, 25, 20),
                muted: Color::Rgb(90, 80, 65),
                accent: Color::Rgb(120, 70, 15),
                border: Color::Rgb(145, 130, 105),
                highlight: Color::Rgb(225, 220, 200),
                danger: Color::Rgb(150, 30, 35),
            },
            Self::Sepia => ThemeColors {
                background: Color::Rgb(30, 22, 12),
                surface: Color::Rgb(45, 33, 18),
                foreground: Color::Rgb(210, 190, 155),
                muted: Color::Rgb(175, 150, 110),
                accent: Color::Rgb(195, 145, 60),
                border: Color::Rgb(135, 110, 70),
                highlight: Color::Rgb(60, 45, 22),
                danger: Color::Rgb(240, 125, 100),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_and_borders_meet_contrast_floor() {
        for theme in [Theme::Dark, Theme::Light, Theme::Sepia] {
            let colors = theme.colors();
            assert!(contrast(colors.muted, colors.surface) >= 4.5);
            assert!(contrast(colors.accent, colors.highlight) >= 4.5);
            assert!(contrast(colors.border, colors.surface) >= 3.0);
        }
    }

    fn contrast(foreground: Color, background: Color) -> f64 {
        let luminance = |color| match color {
            Color::Rgb(red, green, blue) => [red, green, blue]
                .into_iter()
                .zip([0.2126, 0.7152, 0.0722])
                .map(|(channel, weight)| {
                    let value = f64::from(channel) / 255.0;
                    let linear = if value <= 0.04045 {
                        value / 12.92
                    } else {
                        ((value + 0.055) / 1.055).powf(2.4)
                    };
                    linear * weight
                })
                .sum::<f64>(),
            _ => panic!("theme colors must use RGB values"),
        };
        let first = luminance(foreground);
        let second = luminance(background);
        (first.max(second) + 0.05) / (first.min(second) + 0.05)
    }
}
