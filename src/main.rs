#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod auth;
mod icons;
mod style;
mod types;
mod ytmusic;

use iced::{Size, window};

use app::Hum;

fn main() -> iced::Result {
    iced::application(Hum::boot, Hum::update, Hum::view)
        .title(Hum::title)
        .subscription(Hum::subscription)
        .theme(|_: &Hum| style::app_theme())
        .window(window::Settings {
            size: Size::new(1160.0, 740.0),
            min_size: Some(Size::new(860.0, 560.0)),
            ..Default::default()
        })
        .centered()
        .run()
}
