use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::platform::run_return::EventLoopExtRunReturn;
use tao::window::WindowBuilder;
use wry::{PageLoadEvent, WebContext, WebView, WebViewBuilder};

use crate::auth::app_data_dir;

pub const SIGNIN_FLAG: &str = "--ytm-signin";
const YTM_LOGIN_URL: &str = "https://accounts.google.com/ServiceLogin?ltmpl=music&service=youtube&uilel=3&passive=true&continue=https%3A%2F%2Fwww.youtube.com%2Fsignin%3Faction_handle_signin%3Dtrue%26app%3Ddesktop%26hl%3Den%26next%3Dhttps%253A%252F%252Fmusic.youtube.com%252F&hl=en";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebViewCookiePayload {
    pub cookie_header: String,
    pub sapisid: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum WebViewOutcome {
    Ok(WebViewCookiePayload),
    Err { message: String },
}

#[derive(Debug, Clone, Copy)]
enum UserEvent {
    CheckCookies,
}

/// Returns the output file path when launched in the `--ytm-signin <path>` popup mode.
pub fn signin_arg() -> Option<PathBuf> {
    let mut args = std::env::args_os();
    while let Some(arg) = args.next() {
        if arg == SIGNIN_FLAG {
            return args.next().map(PathBuf::from);
        }
    }
    None
}

/// Runs the embedded WebView2 Google/YouTube Music login window until cookies are captured or closed.
pub fn run_signin_window(out_path: &Path) -> i32 {
    match open_and_wait(out_path) {
        Ok(Some(payload)) => {
            write_outcome(out_path, &WebViewOutcome::Ok(payload));
            0
        }
        Ok(None) => {
            write_outcome(
                out_path,
                &WebViewOutcome::Err {
                    message: String::from("Sign-in cancelled."),
                },
            );
            1
        }
        Err(message) => {
            write_outcome(out_path, &WebViewOutcome::Err { message });
            1
        }
    }
}

fn write_outcome(path: &Path, outcome: &WebViewOutcome) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(outcome) {
        let _ = std::fs::write(path, json);
    }
}

fn open_and_wait(out_path: &Path) -> Result<Option<WebViewCookiePayload>, String> {
    let _ = out_path;
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("Sign in to YouTube Music")
        .with_inner_size(LogicalSize::new(500.0, 680.0))
        .with_min_inner_size(LogicalSize::new(400.0, 520.0))
        .build(&event_loop)
        .map_err(|e| format!("Failed creating sign-in window: {e}"))?;

    let profile_dir = app_data_dir().map(|d| d.join("webview_login"));
    if let Some(dir) = &profile_dir {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut web_context = WebContext::new(profile_dir);

    let webview = WebViewBuilder::new_with_web_context(&mut web_context)
        .with_incognito(true)
        .with_background_color((18, 18, 18, 255))
        .with_on_page_load_handler(move |event, url| {
            if matches!(event, PageLoadEvent::Finished)
                && url.starts_with("https://music.youtube.com")
            {
                let _ = proxy.send_event(UserEvent::CheckCookies);
            }
        })
        .with_url(YTM_LOGIN_URL)
        .build(&window)
        .map_err(|e| format!("Failed starting WebView2 sign-in: {e}"))?;

    let mut captured: Option<WebViewCookiePayload> = None;

    event_loop.run_return(|event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(UserEvent::CheckCookies) => {
                if captured.is_none()
                    && let Some(payload) = extract_ytm_cookies(&webview)
                {
                    captured = Some(payload);
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
    });

    Ok(captured)
}

fn extract_ytm_cookies(webview: &WebView) -> Option<WebViewCookiePayload> {
    let mut map = BTreeMap::new();
    for url in ["https://www.youtube.com", "https://music.youtube.com"] {
        if let Ok(cookies) = webview.cookies_for_url(url) {
            for cookie in cookies {
                let value = cookie.value().trim();
                if !value.is_empty() {
                    map.insert(cookie.name().to_string(), value.to_string());
                }
            }
        }
    }

    let sapisid = map
        .get("SAPISID")
        .or_else(|| map.get("__Secure-3PAPISID"))
        .or_else(|| map.get("__Secure-1PAPISID"))
        .cloned()?;

    let has_session_cookie = ["SID", "__Secure-3PSID", "__Secure-1PSID", "LOGIN_INFO"]
        .iter()
        .any(|k| map.contains_key(*k));
    if !has_session_cookie {
        return None;
    }

    let cookie_header = map
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ");

    Some(WebViewCookiePayload {
        cookie_header,
        sapisid,
    })
}
