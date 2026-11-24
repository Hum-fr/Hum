use iced::widget::svg;
use iced::{Color, Theme};

fn icon<'a>(bytes: &'static [u8], size: f32, color: Color) -> svg::Svg<'a> {
    svg(svg::Handle::from_memory(bytes))
        .width(size)
        .height(size)
        .style(move |_: &Theme, _| svg::Style { color: Some(color) })
}

pub fn home<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor"><path d="M12 3.1 4 9.5V20a1 1 0 0 0 1 1h5v-6h4v6h5a1 1 0 0 0 1-1V9.5l-8-6.4Z"/></svg>"#,
        size,
        color,
    )
}

pub fn settings<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.08a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.08a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z"/></svg>"#,
        size,
        color,
    )
}

pub fn explore<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><polygon points="15.5 8.5 13.5 13.5 8.5 15.5 10.5 10.5 15.5 8.5"/></svg>"#,
        size,
        color,
    )
}

pub fn library<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 6v12a2 2 0 0 0 2 2h12"/><rect x="8" y="4" width="12" height="12" rx="2"/><path d="M13 12V8l3-1"/></svg>"#,
        size,
        color,
    )
}

pub fn queue<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 6h16M4 12h10M4 18h10"/><path d="m17 12 4 3-4 3v-6Z" fill="currentColor" stroke="none"/></svg>"#,
        size,
        color,
    )
}

pub fn search<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>"#,
        size,
        color,
    )
}

pub fn play<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor"><path d="M7 5.2v13.6a.8.8 0 0 0 1.2.7l11-6.8a.8.8 0 0 0 0-1.4l-11-6.8A.8.8 0 0 0 7 5.2Z"/></svg>"#,
        size,
        color,
    )
}

pub fn pause<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor"><rect x="6.5" y="5" width="4" height="14" rx="1"/><rect x="13.5" y="5" width="4" height="14" rx="1"/></svg>"#,
        size,
        color,
    )
}

pub fn prev<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor"><rect x="5" y="6" width="2.2" height="12" rx="0.8"/><path d="M18.5 6.4v11.2a.7.7 0 0 1-1.1.6L8.8 12.6a.7.7 0 0 1 0-1.2l8.6-5.6a.7.7 0 0 1 1.1.6Z"/></svg>"#,
        size,
        color,
    )
}

pub fn next<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor"><rect x="16.8" y="6" width="2.2" height="12" rx="0.8"/><path d="M5.5 6.4v11.2a.7.7 0 0 0 1.1.6l8.6-5.6a.7.7 0 0 0 0-1.2L6.6 5.8a.7.7 0 0 0-1.1.6Z"/></svg>"#,
        size,
        color,
    )
}

pub fn heart<'a>(size: f32, color: Color, filled: bool) -> svg::Svg<'a> {
    let bytes: &'static [u8] = if filled {
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor"><path d="M12 20.4s-7.5-4.7-9.4-9.2C1.2 7.8 3.6 4.5 7 4.5c2 0 3.7 1 5 2.7 1.3-1.7 3-2.7 5-2.7 3.4 0 5.8 3.3 4.4 6.7-1.9 4.5-9.4 9.2-9.4 9.2Z"/></svg>"#
    } else {
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"><path d="M12 20.4s-7.5-4.7-9.4-9.2C1.2 7.8 3.6 4.5 7 4.5c2 0 3.7 1 5 2.7 1.3-1.7 3-2.7 5-2.7 3.4 0 5.8 3.3 4.4 6.7-1.9 4.5-9.4 9.2-9.4 9.2Z"/></svg>"#
    };
    icon(bytes, size, color)
}

pub fn volume<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 5 6 9H3v6h3l5 4V5Z"/><path d="M15.5 8.5a5 5 0 0 1 0 7"/><path d="M18.5 6a8.5 8.5 0 0 1 0 12"/></svg>"#,
        size,
        color,
    )
}

pub fn volume_mute<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 5 6 9H3v6h3l5 4V5Z"/><path d="m22 9-6 6"/><path d="m16 9 6 6"/></svg>"#,
        size,
        color,
    )
}

pub fn user<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="8" r="4"/><path d="M4.5 20a7.5 7.5 0 0 1 15 0"/></svg>"#,
        size,
        color,
    )
}

pub fn back<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 18l-6-6 6-6"/></svg>"#,
        size,
        color,
    )
}

pub fn note<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18V5l10-2v13"/><circle cx="6.5" cy="18" r="2.5"/><circle cx="16.5" cy="16" r="2.5"/></svg>"#,
        size,
        color,
    )
}

pub fn shuffle<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 3h5v5"/><path d="M4 20 21 3"/><path d="M21 16v5h-5"/><path d="m15 15 6 6"/><path d="m4 4 5 5"/></svg>"#,
        size,
        color,
    )
}

pub fn repeat<'a>(size: f32, color: Color, one: bool) -> svg::Svg<'a> {
    let bytes: &'static [u8] = if one {
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m17 2 4 4-4 4"/><path d="M3 11v-1a4 4 0 0 1 4-4h14"/><path d="m7 22-4-4 4-4"/><path d="M21 13v1a4 4 0 0 1-4 4H3"/><path d="M11 10h1v4"/></svg>"#
    } else {
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m17 2 4 4-4 4"/><path d="M3 11v-1a4 4 0 0 1 4-4h14"/><path d="m7 22-4-4 4-4"/><path d="M21 13v1a4 4 0 0 1-4 4H3"/></svg>"#
    };
    icon(bytes, size, color)
}

pub fn plus<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>"#,
        size,
        color,
    )
}

pub fn trash<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/></svg>"#,
        size,
        color,
    )
}

pub fn list_plus<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M11 12H3"/><path d="M16 6H3"/><path d="M16 18H3"/><path d="M18 9v6"/><path d="M21 12h-6"/></svg>"#,
        size,
        color,
    )
}

pub fn lyrics<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/><path d="M8 8h8"/><path d="M8 12h5"/></svg>"#,
        size,
        color,
    )
}

pub fn forward<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18l6-6-6-6"/></svg>"#,
        size,
        color,
    )
}

pub fn chevron_down<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#,
        size,
        color,
    )
}

pub fn menu<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 7h16M4 12h16M4 17h12"/></svg>"#,
        size,
        color,
    )
}

pub fn hum_mark<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><path d="M5 10v4"/><path d="M9.5 6v12"/><path d="M14.5 4v16"/><path d="M19 8v8"/></svg>"#,
        size,
        color,
    )
}

pub fn sliders<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6"/></svg>"#,
        size,
        color,
    )
}

pub fn radio<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="2"/><path d="M16.2 7.8a6 6 0 0 1 0 8.4M7.8 16.2a6 6 0 0 1 0-8.4M19.1 4.9a10 10 0 0 1 0 14.2M4.9 19.1a10 10 0 0 1 0-14.2"/></svg>"#,
        size,
        color,
    )
}

pub fn sparkles<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 3-1.9 5.8a2 2 0 0 1-1.3 1.3L3 12l5.8 1.9a2 2 0 0 1 1.3 1.3L12 21l1.9-5.8a2 2 0 0 1 1.3-1.3L21 12l-5.8-1.9a2 2 0 0 1-1.3-1.3Z"/></svg>"#,
        size,
        color,
    )
}

pub fn moon<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/></svg>"#,
        size,
        color,
    )
}

pub fn maximize<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/></svg>"#,
        size,
        color,
    )
}

pub fn minimize<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 14h6v6M20 10h-6V4M14 10l7-7M3 21l7-7"/></svg>"#,
        size,
        color,
    )
}

pub fn command<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 6v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3V6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3Z"/></svg>"#,
        size,
        color,
    )
}

pub fn arrow_up<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m18 15-6-6-6 6"/></svg>"#,
        size,
        color,
    )
}

pub fn arrow_down<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#,
        size,
        color,
    )
}

pub fn wave<'a>(size: f32, color: Color) -> svg::Svg<'a> {
    icon(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 12c2.2-5 4.3-5 6.5 0s4.3 5 6.5 0 4.3-5 7 0"/></svg>"#,
        size,
        color,
    )
}
