use iced::widget::{button, container, slider, text_input};
use iced::{Background, Border, Color, Theme, color};

pub const BG_CANVAS: Color = color!(0x030303);
pub const BG_SIDEBAR: Color = color!(0x030303);
pub const BG_HEADER: Color = color!(0x0A0A0A);
pub const BG_PLAYER: Color = color!(0x212121);
pub const BG_CARD: Color = color!(0x121212);
pub const BG_HOVER: Color = color!(0x282828);
pub const BG_PILL: Color = color!(0x1E1E1E);
pub const BORDER: Color = color!(0x272727);
pub const TEXT_PRIMARY: Color = color!(0xFFFFFF);
pub const TEXT_SECONDARY: Color = color!(0xAAAAAA);
pub const TEXT_MUTED: Color = color!(0x717171);
pub const YT_RED: Color = color!(0xFF0000);

pub fn app_theme() -> Theme {
    Theme::custom(
        "YouTube Music".to_string(),
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
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn player_bar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_PLAYER)),
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
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

pub fn brand_circle(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(YT_RED)),
        border: Border {
            radius: 14.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn modal_card(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color!(0x181818))),
        border: Border {
            color: color!(0x353535),
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    }
}

pub fn search_input(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let (bg, border) = match status {
        text_input::Status::Focused { .. } => (color!(0x181818), color!(0x555555)),
        text_input::Status::Hovered => (color!(0x222222), color!(0x383838)),
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
        selection: color!(0x3D3D3D),
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
            BG_HOVER
        } else if matches!(status, button::Status::Hovered) {
            color!(0x181818)
        } else {
            Color::TRANSPARENT
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if active { TEXT_PRIMARY } else { TEXT_SECONDARY },
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub fn track_row(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = match (active, status) {
            (true, button::Status::Hovered) => color!(0x262626),
            (true, _) => color!(0x1D1D1D),
            (false, button::Status::Hovered) => color!(0x181818),
            (false, button::Status::Pressed) => BG_HOVER,
            _ => Color::TRANSPARENT,
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: TEXT_PRIMARY,
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub fn card_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => color!(0x181818),
        button::Status::Pressed => BG_HOVER,
        _ => Color::TRANSPARENT,
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT_PRIMARY,
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn mood_pill(accent: u32) -> impl Fn(&Theme, button::Status) -> button::Style {
    let r = ((accent >> 16) & 0xFF) as f32 / 255.0;
    let g = ((accent >> 8) & 0xFF) as f32 / 255.0;
    let b = (accent & 0xFF) as f32 / 255.0;
    let stripe = Color::from_rgb(r, g, b);

    move |_theme, status| {
        let bg = match status {
            button::Status::Hovered => BG_HOVER,
            button::Status::Pressed => color!(0x303030),
            _ => BG_CARD,
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: TEXT_PRIMARY,
            border: Border {
                color: stripe,
                width: 1.5,
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    }
}

pub fn icon_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => color!(0x323232),
        button::Status::Pressed => color!(0x3E3E3E),
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
                button::Status::Hovered => color!(0xE5E5E5),
                button::Status::Pressed => color!(0xCCCCCC),
                _ => TEXT_PRIMARY,
            }
        } else {
            match status {
                button::Status::Hovered => BG_HOVER,
                button::Status::Pressed => color!(0x333333),
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
        button::Status::Hovered => color!(0xE6E6E6),
        button::Status::Pressed => color!(0xCCCCCC),
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

pub fn yt_progress_slider(_theme: &Theme, status: slider::Status) -> slider::Style {
    let handle_radius = match status {
        slider::Status::Hovered | slider::Status::Dragged => 6.0,
        slider::Status::Active => 0.0,
    };

    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(YT_RED),
                Background::Color(color!(0x404040)),
            ),
            width: 3.0,
            border: Border::default(),
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle {
                radius: handle_radius,
            },
            background: Background::Color(YT_RED),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        },
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
