use iced::gradient::Linear;
use iced::widget::{button, container, slider, text_input};
use iced::{Background, Border, Color, Gradient, Radians, Shadow, Theme, Vector, color};

pub const BG_CANVAS: Color = color!(0x090A0F);
pub const BG_SIDEBAR: Color = Color::from_rgba(0.035, 0.04, 0.062, 0.62);
pub const BG_HEADER: Color = Color::TRANSPARENT;
pub const BG_PLAYER: Color = Color::from_rgba(0.038, 0.042, 0.065, 0.86);
pub const BG_CARD: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.045);
pub const BG_HOVER: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.095);
pub const BG_PILL: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.075);
pub const BORDER: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.095);
pub const TEXT_PRIMARY: Color = color!(0xF5F6FA);
pub const TEXT_SECONDARY: Color = color!(0xA7ACBA);
pub const TEXT_MUTED: Color = color!(0x6E7382);
pub const YT_RED: Color = color!(0xFF3355);

#[derive(Debug, Clone, Copy)]
pub struct CoverPalette {
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub accent: Color,
}

impl Default for CoverPalette {
    fn default() -> Self {
        Self {
            primary: Color::from_rgb(0.16, 0.08, 0.16),
            secondary: Color::from_rgb(0.07, 0.11, 0.20),
            tertiary: color!(0x6E56CF),
            accent: YT_RED,
        }
    }
}

pub fn extract_cover_palette(bytes: &[u8]) -> CoverPalette {
    let Ok(img) = image::load_from_memory(bytes) else {
        return CoverPalette::default();
    };
    let thumb = img
        .resize_exact(16, 16, image::imageops::FilterType::Nearest)
        .to_rgb8();

    let mut buckets: [(f32, f32, f32, f32); 2] = [(0.0, 0.0, 0.0, 0.0); 2];
    for (idx, p) in thumb.pixels().enumerate() {
        let r = p[0] as f32 / 255.0;
        let g = p[1] as f32 / 255.0;
        let b = p[2] as f32 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let sat = if max > 0.001 { (max - min) / max } else { 0.0 };
        let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        // Prefer rich mid-tone colors over pure black/white borders.
        let luma_weight = if (0.08..=0.88).contains(&luma) { 1.0 } else { 0.2 };
        let weight = (0.15 + sat * 2.5) * luma_weight;
        let slot = usize::from(idx >= 128);
        buckets[slot].0 += r * weight;
        buckets[slot].1 += g * weight;
        buckets[slot].2 += b * weight;
        buckets[slot].3 += weight;
    }

    let avg = |(sr, sg, sb, sw): (f32, f32, f32, f32)| -> (f32, f32, f32) {
        if sw > 0.001 {
            (sr / sw, sg / sw, sb / sw)
        } else {
            (0.14, 0.10, 0.18)
        }
    };

    let (r1, g1, b1) = avg(buckets[0]);
    let (r2, g2, b2) = avg(buckets[1]);

    // Scale mood colors into deep, readable dark surfaces so white typography stays crisp.
    let mood = |r: f32, g: f32, b: f32, gain: f32| {
        let lum = (0.2126 * r + 0.7152 * g + 0.0722 * b).max(0.05);
        let scale = (0.26 / lum).min(1.25) * gain;
        Color::from_rgb(
            (r * scale + 0.03).clamp(0.03, 0.38),
            (g * scale + 0.03).clamp(0.03, 0.34),
            (b * scale + 0.05).clamp(0.05, 0.42),
        )
    };

    let accent = Color::from_rgb(
        (r1 * 0.8 + 0.28).clamp(0.45, 1.0),
        (g1 * 0.8 + 0.20).clamp(0.32, 0.92),
        (b1 * 0.8 + 0.24).clamp(0.38, 0.98),
    );
    let tertiary = Color::from_rgb(
        (r2 * 0.85 + 0.24).clamp(0.38, 0.96),
        (g2 * 0.85 + 0.26).clamp(0.38, 0.96),
        (b2 * 0.85 + 0.30).clamp(0.44, 1.0),
    );

    CoverPalette {
        primary: mood(r1, g1, b1, 0.96),
        secondary: mood(r2, g2, b2, 0.84),
        tertiary,
        accent,
    }
}

pub fn app_theme() -> Theme {
    Theme::custom(
        "hum".to_string(),
        iced::theme::Palette {
            background: BG_CANVAS,
            text: TEXT_PRIMARY,
            primary: YT_RED,
            success: color!(0x2BA640),
            warning: color!(0xF5A623),
            danger: YT_RED,
        },
    )
}

pub fn sidebar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_SIDEBAR)),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn header_bar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_HEADER)),
        ..Default::default()
    }
}

pub fn player_bar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_PLAYER)),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn divider(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BORDER)),
        ..Default::default()
    }
}

pub fn thumb_box(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_CARD)),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

pub fn brand_circle(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(YT_RED)),
        border: Border {
            radius: 10.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn modal_card(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color!(0x13151D))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.12),
            width: 1.0,
            radius: 14.0.into(),
        },
        ..Default::default()
    }
}

pub fn glass_card(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.055))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.09),
            width: 1.0,
            radius: 14.0.into(),
        },
        ..Default::default()
    }
}

pub fn search_input(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let (bg, border) = match status {
        text_input::Status::Focused { .. } => (
            Color::from_rgba(1.0, 1.0, 1.0, 0.11),
            Color::from_rgba(1.0, 1.0, 1.0, 0.25),
        ),
        text_input::Status::Hovered => (
            Color::from_rgba(1.0, 1.0, 1.0, 0.09),
            Color::from_rgba(1.0, 1.0, 1.0, 0.16),
        ),
        _ => (BG_PILL, BORDER),
    };

    text_input::Style {
        background: Background::Color(bg),
        border: Border {
            color: border,
            width: 1.0,
            radius: 20.0.into(),
        },
        icon: TEXT_SECONDARY,
        placeholder: TEXT_MUTED,
        value: TEXT_PRIMARY,
        selection: Color::from_rgba(1.0, 1.0, 1.0, 0.22),
    }
}

pub fn plain_input(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let border = match status {
        text_input::Status::Focused { .. } => color!(0x666666),
        text_input::Status::Hovered => color!(0x404040),
        _ => BORDER,
    };

    text_input::Style {
        background: Background::Color(color!(0x0F0F0F)),
        border: Border {
            color: border,
            width: 1.0,
            radius: 8.0.into(),
        },
        icon: TEXT_MUTED,
        placeholder: TEXT_MUTED,
        value: TEXT_PRIMARY,
        selection: color!(0x3D3D3D),
    }
}

pub fn nav_item(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = if active {
            Color::from_rgba(1.0, 1.0, 1.0, 0.10)
        } else if matches!(status, button::Status::Hovered) {
            Color::from_rgba(1.0, 1.0, 1.0, 0.055)
        } else {
            Color::TRANSPARENT
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if active { TEXT_PRIMARY } else { TEXT_SECONDARY },
            border: Border {
                radius: 10.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub fn track_row(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = match (active, status) {
            (true, button::Status::Hovered) => Color::from_rgba(1.0, 1.0, 1.0, 0.13),
            (true, _) => Color::from_rgba(1.0, 1.0, 1.0, 0.09),
            (false, button::Status::Hovered) => Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            (false, button::Status::Pressed) => BG_HOVER,
            _ => Color::TRANSPARENT,
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: TEXT_PRIMARY,
            border: Border {
                radius: 10.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub fn lyric_row(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.055),
        button::Status::Pressed => Color::from_rgba(1.0, 1.0, 1.0, 0.09),
        _ => Color::TRANSPARENT,
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT_PRIMARY,
        border: Border {
            radius: 12.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn card_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.06),
        button::Status::Pressed => BG_HOVER,
        _ => Color::TRANSPARENT,
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT_PRIMARY,
        border: Border {
            radius: 12.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn mood_pill(accent: u32) -> impl Fn(&Theme, button::Status) -> button::Style {
    let r = ((accent >> 16) & 0xFF) as f32 / 255.0;
    let g = ((accent >> 8) & 0xFF) as f32 / 255.0;
    let b = (accent & 0xFF) as f32 / 255.0;
    let stripe = Color::from_rgba(r, g, b, 0.65);

    move |_theme, status| {
        let bg = match status {
            button::Status::Hovered => BG_HOVER,
            button::Status::Pressed => Color::from_rgba(1.0, 1.0, 1.0, 0.14),
            _ => BG_CARD,
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: TEXT_PRIMARY,
            border: Border {
                color: stripe,
                width: 1.0,
                radius: 18.0.into(),
            },
            ..Default::default()
        }
    }
}

pub fn icon_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.10),
        button::Status::Pressed => Color::from_rgba(1.0, 1.0, 1.0, 0.16),
        _ => Color::TRANSPARENT,
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT_PRIMARY,
        border: Border {
            radius: 18.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn pill_button(primary: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = if primary {
            match status {
                button::Status::Hovered => color!(0xE6E9F0),
                button::Status::Pressed => color!(0xCFD4DF),
                _ => TEXT_PRIMARY,
            }
        } else {
            match status {
                button::Status::Hovered => BG_HOVER,
                button::Status::Pressed => Color::from_rgba(1.0, 1.0, 1.0, 0.14),
                _ => BG_PILL,
            }
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if primary { BG_CANVAS } else { TEXT_PRIMARY },
            border: Border {
                color: if primary { Color::TRANSPARENT } else { BORDER },
                width: 1.0,
                radius: 18.0.into(),
            },
            ..Default::default()
        }
    }
}

pub fn play_circle(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => color!(0xE6E9F0),
        button::Status::Pressed => color!(0xCFD4DF),
        _ => TEXT_PRIMARY,
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: BG_CANVAS,
        border: Border {
            radius: 20.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn yt_progress_slider(accent: Color) -> impl Fn(&Theme, slider::Status) -> slider::Style {
    move |_theme, status| {
        let handle_radius = match status {
            slider::Status::Hovered | slider::Status::Dragged => 5.5,
            slider::Status::Active => 0.0,
        };

        slider::Style {
            rail: slider::Rail {
                backgrounds: (
                    Background::Color(accent),
                    Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.16)),
                ),
                width: 3.0,
                border: Border::default(),
            },
            handle: slider::Handle {
                shape: slider::HandleShape::Circle {
                    radius: handle_radius,
                },
                background: Background::Color(TEXT_PRIMARY),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
        }
    }
}

pub fn volume_slider(_theme: &Theme, status: slider::Status) -> slider::Style {
    let handle_radius = match status {
        slider::Status::Hovered | slider::Status::Dragged => 5.0,
        slider::Status::Active => 0.0,
    };

    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(TEXT_PRIMARY),
                Background::Color(color!(0x454545)),
            ),
            width: 3.0,
            border: Border {
                radius: 2.0.into(),
                ..Default::default()
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle {
                radius: handle_radius,
            },
            background: Background::Color(TEXT_PRIMARY),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        },
    }
}

pub fn floating_card(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.055))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.11),
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.40),
            offset: Vector::new(0.0, 10.0),
            blur_radius: 26.0,
        },
        ..Default::default()
    }
}

pub fn accent_card(accent: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Gradient(Gradient::Linear(
            Linear::new(Radians(1.2))
                .add_stop(0.0, Color::from_rgba(accent.r, accent.g, accent.b, 0.18))
                .add_stop(1.0, Color::from_rgba(1.0, 1.0, 1.0, 0.045)),
        ))),
        border: Border {
            color: Color::from_rgba(accent.r, accent.g, accent.b, 0.36),
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(accent.r, accent.g, accent.b, 0.18),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 30.0,
        },
        ..Default::default()
    }
}

pub fn cover_glow(accent: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.14),
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(accent.r, accent.g, accent.b, 0.35),
            offset: Vector::new(0.0, 18.0),
            blur_radius: 44.0,
        },
        ..Default::default()
    }
}

pub fn toast_banner(accent: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.08, 0.09, 0.14, 0.94))),
        border: Border {
            color: Color::from_rgba(accent.r, accent.g, accent.b, 0.55),
            width: 1.0,
            radius: 22.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.50),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        ..Default::default()
    }
}

pub fn command_modal(accent: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.065, 0.072, 0.105, 0.97))),
        border: Border {
            color: Color::from_rgba(accent.r, accent.g, accent.b, 0.35),
            width: 1.0,
            radius: 18.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.68),
            offset: Vector::new(0.0, 22.0),
            blur_radius: 50.0,
        },
        ..Default::default()
    }
}

pub fn fx_badge(accent: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(Color::from_rgba(accent.r, accent.g, accent.b, 0.18))),
        border: Border {
            color: Color::from_rgba(accent.r, accent.g, accent.b, 0.45),
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    }
}

pub fn chip_button(active: bool, accent: Color) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = if active {
            match status {
                button::Status::Hovered => Color::from_rgba(accent.r, accent.g, accent.b, 0.32),
                _ => Color::from_rgba(accent.r, accent.g, accent.b, 0.22),
            }
        } else {
            match status {
                button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.11),
                button::Status::Pressed => Color::from_rgba(1.0, 1.0, 1.0, 0.16),
                _ => BG_PILL,
            }
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: TEXT_PRIMARY,
            border: Border {
                color: if active {
                    Color::from_rgba(accent.r, accent.g, accent.b, 0.72)
                } else {
                    BORDER
                },
                width: 1.0,
                radius: 18.0.into(),
            },
            ..Default::default()
        }
    }
}

pub fn eq_slider(accent: Color) -> impl Fn(&Theme, slider::Status) -> slider::Style {
    move |_theme, status| {
        let handle_radius = match status {
            slider::Status::Hovered | slider::Status::Dragged => 7.0,
            slider::Status::Active => 5.5,
        };

        slider::Style {
            rail: slider::Rail {
                backgrounds: (
                    Background::Color(accent),
                    Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.16)),
                ),
                width: 4.0,
                border: Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
            },
            handle: slider::Handle {
                shape: slider::HandleShape::Circle {
                    radius: handle_radius,
                },
                background: Background::Color(TEXT_PRIMARY),
                border_width: 1.5,
                border_color: accent,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_cover_palette_handles_invalid_bytes() {
        let palette = extract_cover_palette(b"not-an-image");
        let fallback = CoverPalette::default();
        assert_eq!(palette.primary, fallback.primary);
        assert_eq!(palette.secondary, fallback.secondary);
        assert_eq!(palette.tertiary, fallback.tertiary);
        assert_eq!(palette.accent, fallback.accent);
    }
}
