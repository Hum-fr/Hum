#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod auth;
mod icons;
mod style;
mod types;
mod updater;
mod webview_login;
mod ytmusic;

use iced::{Size, window};

use app::Hum;

fn main() -> iced::Result {
    if let Some(out_path) = webview_login::signin_arg() {
        std::process::exit(webview_login::run_signin_window(&out_path));
    }

    updater::cleanup_old_exe();

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
