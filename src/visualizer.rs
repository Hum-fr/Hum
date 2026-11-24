use std::f32::consts::TAU;

use iced::mouse;
use iced::widget::canvas::{self, Frame, Gradient, Path, Stroke, gradient};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

use crate::style::CoverPalette;
use crate::types::{SpectrumFrame, VisualizerMode};

pub struct AmbientBackdrop {
    pub palette: CoverPalette,
    pub bass: f32,
    pub phase: f32,
    pub intense: bool,
    pub enabled: bool,
}

impl<Message> canvas::Program<Message> for AmbientBackdrop {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let w = bounds.width;
        let h = bounds.height;
        if w <= 1.0 || h <= 1.0 {
            return vec![frame.into_geometry()];
        }

        // Deep obsidian gradient backdrop
        let base_grad = gradient::Linear::new(Point::new(0.0, 0.0), Point::new(w, h))
            .add_stop(0.0, Color::from_rgb(0.032, 0.036, 0.056))
            .add_stop(0.55, Color::from_rgb(0.025, 0.028, 0.042))
            .add_stop(1.0, Color::from_rgb(0.018, 0.020, 0.032));
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Gradient::Linear(base_grad));

        if !self.enabled {
            return vec![frame.into_geometry()];
        }

        let gain = if self.intense { 1.35 } else { 0.85 };
        let pulse = 1.0 + self.bass * 0.22;
        let dim = w.max(h);

        let orbs = [
            (
                Point::new(
                    w * (0.18 + 0.06 * self.phase.sin()),
                    h * (0.16 + 0.05 * (self.phase * 0.7).cos()),
                ),
                dim * 0.44 * pulse,
                self.palette.primary,
                (0.28 * gain + self.bass * 0.14).clamp(0.05, 0.52),
            ),
            (
                Point::new(
                    w * (0.82 + 0.05 * (self.phase * 0.6 + 1.4).cos()),
                    h * (0.28 + 0.06 * (self.phase * 0.5).sin()),
                ),
                dim * 0.42 * (1.0 + self.bass * 0.16),
                self.palette.secondary,
                (0.24 * gain + self.bass * 0.12).clamp(0.05, 0.48),
            ),
            (
                Point::new(
                    w * (0.50 + 0.08 * (self.phase * 0.45 + 2.6).sin()),
                    h * (0.86 + 0.04 * (self.phase * 0.8).cos()),
                ),
                dim * 0.36 * pulse,
                self.palette.tertiary,
                (0.16 * gain + self.bass * 0.12).clamp(0.03, 0.38),
            ),
        ];

        for (center, radius, color, alpha) in orbs {
            draw_soft_glow(&mut frame, center, radius, color, alpha);
        }

        vec![frame.into_geometry()]
    }
}

fn draw_soft_glow(frame: &mut Frame, center: Point, radius: f32, color: Color, peak_alpha: f32) {
    // 6 concentric soft rings produce a smooth bloom without texture shaders
    const RINGS: usize = 6;
    for i in (0..RINGS).rev() {
        let frac = (i + 1) as f32 / RINGS as f32;
        let r = radius * frac;
        let falloff = (1.0 - frac * 0.86).powi(2);
        let a = (peak_alpha * falloff * 0.36).clamp(0.0, 0.45);
        frame.fill(
            &Path::circle(center, r),
            Color::from_rgba(color.r, color.g, color.b, a),
        );
    }
}

pub struct AudioVisualizer {
    pub mode: VisualizerMode,
    pub spectrum: SpectrumFrame,
    pub palette: CoverPalette,
    pub phase: f32,
}

impl<Message> canvas::Program<Message> for AudioVisualizer {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        if self.mode == VisualizerMode::Off || bounds.width < 12.0 || bounds.height < 12.0 {
            return vec![frame.into_geometry()];
        }

        match self.mode {
            VisualizerMode::Bars => self.draw_bars(&mut frame, bounds.size()),
            VisualizerMode::Wave => self.draw_wave(&mut frame, bounds.size()),
            VisualizerMode::Halo => self.draw_halo(&mut frame, bounds.size()),
            VisualizerMode::Off => {}
        }

        vec![frame.into_geometry()]
    }
}

impl AudioVisualizer {
    fn draw_bars(&self, frame: &mut Frame, size: Size) {
        let count = 32usize;
        let pad_x = 4.0_f32;
        let avail_w = (size.width - pad_x * 2.0).max(32.0);
        let step = avail_w / count as f32;
        let bar_w = (step * 0.62).clamp(2.5, 14.0);
        let base_y = size.height * 0.78;
        let max_h = (size.height * 0.72).max(10.0);

        // Subtle baseline glow
        frame.fill_rectangle(
            Point::new(pad_x, base_y),
            Size::new(avail_w, 1.0),
            Color::from_rgba(1.0, 1.0, 1.0, 0.08),
        );

        for i in 0..count {
            let val = self.spectrum.bands[i].clamp(0.0, 1.0);
            let peak = self.spectrum.peaks[i].clamp(0.0, 1.0);
            let h = (val * max_h).max(3.0);
            let x = pad_x + i as f32 * step + (step - bar_w) * 0.5;
            let y = base_y - h;

            let mix = i as f32 / (count - 1) as f32;
            let col = lerp_color(self.palette.accent, self.palette.tertiary, mix);

            let bar_grad = gradient::Linear::new(Point::new(x, base_y), Point::new(x, y))
                .add_stop(0.0, Color::from_rgba(self.palette.primary.r, self.palette.primary.g, self.palette.primary.b, 0.55))
                .add_stop(0.5, Color::from_rgba(col.r, col.g, col.b, 0.85))
                .add_stop(1.0, Color::from_rgba(1.0, 1.0, 1.0, 0.94));

            let radius = (bar_w * 0.45).into();
            frame.fill(
                &Path::rounded_rectangle(Point::new(x, y), Size::new(bar_w, h), radius),
                Gradient::Linear(bar_grad),
            );

            // Soft underside reflection
            let refl_h = (h * 0.24).clamp(1.5, size.height - base_y - 2.0);
            if refl_h > 1.0 {
                frame.fill(
                    &Path::rounded_rectangle(
                        Point::new(x, base_y + 2.0),
                        Size::new(bar_w, refl_h),
                        radius,
                    ),
                    Color::from_rgba(col.r, col.g, col.b, 0.16),
                );
            }

            // Crisp floating peak cap
            if peak > 0.04 {
                let py = (base_y - peak * max_h - 3.5).max(1.0);
                frame.fill(
                    &Path::rounded_rectangle(
                        Point::new(x, py),
                        Size::new(bar_w, 2.2),
                        1.0.into(),
                    ),
                    Color::from_rgba(1.0, 1.0, 1.0, 0.82),
                );
            }
        }
    }

    fn draw_wave(&self, frame: &mut Frame, size: Size) {
        let mid_y = size.height * 0.56;
        let max_amp = size.height * 0.42;
        let samples = 64usize;

        let build_pts = |amp_scale: f32, phase_off: f32| -> Vec<Point> {
            let mut pts = Vec::with_capacity(samples);
            for i in 0..samples {
                let u = i as f32 / (samples - 1) as f32;
                let band_f = u * 31.0;
                let b0 = band_f.floor() as usize;
                let b1 = (b0 + 1).min(31);
                let frac = band_f - b0 as f32;
                let energy = self.spectrum.bands[b0] * (1.0 - frac) + self.spectrum.bands[b1] * frac;
                let env = (u * std::f32::consts::PI).sin();
                let wave = (u * TAU * 2.2 + self.phase * 2.4 + phase_off).sin();
                let y = mid_y - wave * (4.0 + energy * max_amp * amp_scale) * (0.25 + 0.75 * env);
                pts.push(Point::new(u * size.width, y));
            }
            pts
        };

        let pts_back = build_pts(0.75, 1.4);
        let pts_front = build_pts(1.0, 0.0);

        // Translucent fill under front ribbon
        let fill_path = Path::new(|b| {
            b.move_to(Point::new(0.0, size.height));
            for p in &pts_front {
                b.line_to(*p);
            }
            b.line_to(Point::new(size.width, size.height));
            b.close();
        });
        let area_grad = gradient::Linear::new(Point::new(0.0, mid_y - max_amp), Point::new(0.0, size.height))
            .add_stop(0.0, Color::from_rgba(self.palette.accent.r, self.palette.accent.g, self.palette.accent.b, 0.34))
            .add_stop(1.0, Color::from_rgba(self.palette.primary.r, self.palette.primary.g, self.palette.primary.b, 0.02));
        frame.fill(&fill_path, Gradient::Linear(area_grad));

        let stroke_ribbon = |frame: &mut Frame, pts: &[Point], col: Color, width: f32| {
            let p = Path::new(|b| {
                if let Some(first) = pts.first() {
                    b.move_to(*first);
                    for pt in &pts[1..] {
                        b.line_to(*pt);
                    }
                }
            });
            frame.stroke(&p, Stroke::default().with_color(col).with_width(width));
        };

        stroke_ribbon(
            frame,
            &pts_back,
            Color::from_rgba(self.palette.tertiary.r, self.palette.tertiary.g, self.palette.tertiary.b, 0.45),
            2.0,
        );
        stroke_ribbon(
            frame,
            &pts_front,
            Color::from_rgba(self.palette.accent.r, self.palette.accent.g, self.palette.accent.b, 0.95),
            2.6,
        );
    }

    fn draw_halo(&self, frame: &mut Frame, size: Size) {
        let center = Point::new(size.width * 0.5, size.height * 0.5);
        let min_dim = size.width.min(size.height);
        let base_r = min_dim * 0.24 + self.spectrum.bass * min_dim * 0.07;

        // Soft pulsing ambient rings
        frame.fill(
            &Path::circle(center, base_r * 1.35),
            Color::from_rgba(
                self.palette.accent.r,
                self.palette.accent.g,
                self.palette.accent.b,
                0.08 + self.spectrum.rms * 0.14,
            ),
        );
        frame.stroke(
            &Path::circle(center, base_r),
            Stroke::default()
                .with_color(Color::from_rgba(
                    self.palette.tertiary.r,
                    self.palette.tertiary.g,
                    self.palette.tertiary.b,
                    0.75,
                ))
                .with_width(2.0),
        );

        let rays = 32usize;
        for i in 0..rays {
            let angle = (i as f32 / rays as f32) * TAU + self.phase * 0.35;
            let energy = self.spectrum.bands[i];
            let len = 4.0 + energy * (min_dim * 0.22);
            let p0 = Point::new(center.x + angle.cos() * (base_r + 3.0), center.y + angle.sin() * (base_r + 3.0));
            let p1 = Point::new(
                center.x + angle.cos() * (base_r + 3.0 + len),
                center.y + angle.sin() * (base_r + 3.0 + len),
            );
            let col = lerp_color(self.palette.accent, self.palette.tertiary, i as f32 / (rays - 1) as f32);
            frame.stroke(
                &Path::line(p0, p1),
                Stroke::default()
                    .with_color(Color::from_rgba(col.r, col.g, col.b, 0.85))
                    .with_width(3.0),
            );
        }
    }
}

pub struct MiniMeter {
    pub bands: [f32; 8],
    pub accent: Color,
    pub playing: bool,
}

impl<Message> canvas::Program<Message> for MiniMeter {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let count = 8usize;
        let step = bounds.width / count as f32;
        let bar_w = (step * 0.58).max(1.8);

        for i in 0..count {
            let level = if self.playing {
                self.bands[i].clamp(0.12, 1.0)
            } else {
                0.15
            };
            let h = (level * bounds.height).max(2.5);
            let x = i as f32 * step + (step - bar_w) * 0.5;
            let y = (bounds.height - h) * 0.5;
            let col = if self.playing {
                Color::from_rgba(self.accent.r, self.accent.g, self.accent.b, 0.92)
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.22)
            };
            frame.fill(
                &Path::rounded_rectangle(Point::new(x, y), Size::new(bar_w, h), 1.2.into()),
                col,
            );
        }

        vec![frame.into_geometry()]
    }
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::from_rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}
