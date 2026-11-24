use std::f32::consts::TAU;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rodio::source::SeekError;
use rodio::{ChannelCount, SampleRate, Source};

use crate::types::{DspConfig, SpectrumFrame};

pub const EQ_BAND_FREQS: [f32; 5] = [60.0, 230.0, 910.0, 3600.0, 12000.0];
pub const EQ_BAND_LABELS: [&str; 5] = ["60Hz", "230Hz", "910Hz", "3.6k", "12k"];

const FFT_SIZE: usize = 512;
const FFT_HOP: usize = 1024;

pub struct SharedDspState {
    config: Mutex<DspConfig>,
    version: AtomicU64,
}

impl SharedDspState {
    pub fn new(initial: DspConfig) -> Self {
        Self {
            config: Mutex::new(initial),
            version: AtomicU64::new(1),
        }
    }

    pub fn update(&self, cfg: DspConfig) {
        *self.config.lock().unwrap() = cfg;
        self.version.fetch_add(1, Ordering::Release);
    }

    pub fn snapshot(&self) -> (u64, DspConfig) {
        let ver = self.version.load(Ordering::Acquire);
        let cfg = *self.config.lock().unwrap();
        (ver, cfg)
    }

    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, Copy)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    bypass: bool,
}

impl BiquadCoeffs {
    const fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            bypass: true,
        }
    }

    fn design(band_idx: usize, sample_rate: f32, gain_db: f32) -> Self {
        let gain_db = gain_db.clamp(-12.0, 12.0);
        if gain_db.abs() < 0.05 {
            return Self::identity();
        }

        let freq = EQ_BAND_FREQS[band_idx].min(sample_rate * 0.45);
        let w0 = TAU * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let a = 10.0_f32.powf(gain_db / 40.0);

        let (b0, b1, b2, a0, a1, a2) = match band_idx {
            // Band 0: Low shelf
            0 => {
                let slope = 0.85_f32;
                let alpha = sin_w0 * 0.5 * ((a + 1.0 / a) * (1.0 / slope - 1.0) + 2.0).max(0.0).sqrt();
                let beta = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 + beta),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0),
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 - beta),
                    (a + 1.0) + (a - 1.0) * cos_w0 + beta,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0),
                    (a + 1.0) + (a - 1.0) * cos_w0 - beta,
                )
            }
            // Band 4: High shelf
            4 => {
                let q = 0.8_f32;
                let alpha = sin_w0 * 0.5 / q;
                let beta = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 + beta),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0),
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 - beta),
                    (a + 1.0) - (a - 1.0) * cos_w0 + beta,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos_w0),
                    (a + 1.0) - (a - 1.0) * cos_w0 - beta,
                )
            }
            // Bands 1..=3: Peaking EQ (Q = 1.1)
            _ => {
                let alpha = sin_w0 / (2.0 * 1.1);
                (
                    1.0 + alpha * a,
                    -2.0 * cos_w0,
                    1.0 - alpha * a,
                    1.0 + alpha / a,
                    -2.0 * cos_w0,
                    1.0 - alpha / a,
                )
            }
        };

        let inv_a0 = 1.0 / a0;
        Self {
            b0: b0 * inv_a0,
            b1: b1 * inv_a0,
            b2: b2 * inv_a0,
            a1: a1 * inv_a0,
            a2: a2 * inv_a0,
            bypass: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct BiquadState {
    z1: f32,
    z2: f32,
}

impl BiquadState {
    #[inline]
    fn process(&mut self, c: &BiquadCoeffs, x: f32) -> f32 {
        if c.bypass {
            return x;
        }
        // Direct Form II Transposed
        let y = c.b0 * x + self.z1;
        self.z1 = c.b1 * x - c.a1 * y + self.z2;
        self.z2 = c.b2 * x - c.a2 * y;
        y
    }

    fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

struct CombFilter {
    buf: Vec<f32>,
    idx: usize,
    filter_store: f32,
    feedback: f32,
    damp1: f32,
    damp2: f32,
}

impl CombFilter {
    fn new(len: usize, feedback: f32, damp: f32) -> Self {
        Self {
            buf: vec![0.0; len.max(1)],
            idx: 0,
            filter_store: 0.0,
            feedback,
            damp1: damp,
            damp2: 1.0 - damp,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let out = self.buf[self.idx];
        self.filter_store = out * self.damp2 + self.filter_store * self.damp1;
        self.buf[self.idx] = input + self.filter_store * self.feedback;
        self.idx += 1;
        if self.idx >= self.buf.len() {
            self.idx = 0;
        }
        out
    }

    fn reset(&mut self) {
        self.buf.fill(0.0);
        self.idx = 0;
        self.filter_store = 0.0;
    }
}

struct AllpassFilter {
    buf: Vec<f32>,
    idx: usize,
    feedback: f32,
}

impl AllpassFilter {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(1)],
            idx: 0,
            feedback: 0.5,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let buf_out = self.buf[self.idx];
        let out = -input + buf_out;
        self.buf[self.idx] = input + buf_out * self.feedback;
        self.idx += 1;
        if self.idx >= self.buf.len() {
            self.idx = 0;
        }
        out
    }

    fn reset(&mut self) {
        self.buf.fill(0.0);
        self.idx = 0;
    }
}

struct ChannelReverb {
    combs: [CombFilter; 4],
    allpasses: [AllpassFilter; 2],
}

impl ChannelReverb {
    fn new(sample_rate: u32, stereo_offset: usize) -> Self {
        let scale = (sample_rate as f32 / 44100.0).clamp(0.5, 4.0);
        let scaled = |base: usize| ((base + stereo_offset) as f32 * scale).round() as usize;
        Self {
            combs: [
                CombFilter::new(scaled(1116), 0.84, 0.24),
                CombFilter::new(scaled(1188), 0.84, 0.24),
                CombFilter::new(scaled(1277), 0.84, 0.24),
                CombFilter::new(scaled(1356), 0.84, 0.24),
            ],
            allpasses: [
                AllpassFilter::new(scaled(556)),
                AllpassFilter::new(scaled(441)),
            ],
        }
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let gain_in = x * 0.032;
        let mut acc = 0.0;
        for c in &mut self.combs {
            acc += c.process(gain_in);
        }
        let mut y = acc;
        for ap in &mut self.allpasses {
            y = ap.process(y);
        }
        y
    }

    fn reset(&mut self) {
        for c in &mut self.combs {
            c.reset();
        }
        for ap in &mut self.allpasses {
            ap.reset();
        }
    }
}

struct StereoReverb {
    left: ChannelReverb,
    right: ChannelReverb,
}

impl StereoReverb {
    fn new(sample_rate: u32) -> Self {
        Self {
            left: ChannelReverb::new(sample_rate, 0),
            right: ChannelReverb::new(sample_rate, 23),
        }
    }

    #[inline]
    fn process_stereo(&mut self, l: f32, r: f32, mix: f32) -> (f32, f32) {
        if mix <= 0.005 {
            return (l, r);
        }
        let wet = mix.clamp(0.0, 1.0);
        let dry = 1.0 - wet * 0.35;
        let wl = self.left.process(l * 0.8 + r * 0.2);
        let wr = self.right.process(r * 0.8 + l * 0.2);
        (l * dry + wl * wet, r * dry + wr * wet)
    }

    fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
    }
}

#[inline]
pub fn soft_clip(x: f32) -> f32 {
    if !x.is_finite() {
        return 0.0;
    }
    let ax = x.abs();
    if ax <= 0.90 {
        x
    } else {
        let sign = x.signum();
        let excess = (ax - 0.90) / 0.10;
        let compressed = 0.90 + 0.09 * excess.tanh();
        (sign * compressed).clamp(-0.99, 0.99)
    }
}

struct FftAnalyzer {
    ring: [f32; FFT_SIZE],
    write_pos: usize,
    frames_since_emit: usize,
    hann: [f32; FFT_SIZE],
    real: [f32; FFT_SIZE],
    imag: [f32; FFT_SIZE],
    band_edges: [(usize, usize); 32],
}

impl FftAnalyzer {
    fn new(sample_rate: u32) -> Self {
        let mut hann = [0.0_f32; FFT_SIZE];
        for (i, w) in hann.iter_mut().enumerate() {
            *w = 0.5 * (1.0 - (TAU * i as f32 / (FFT_SIZE - 1) as f32).cos());
        }

        let nyquist = (sample_rate as f32 * 0.5).max(8000.0);
        let min_hz = 40.0_f32;
        let max_hz = 16000.0_f32.min(nyquist);
        let ratio = (max_hz / min_hz).max(2.0);
        let bin_hz = sample_rate as f32 / FFT_SIZE as f32;
        let max_bin = FFT_SIZE / 2 - 1;

        let mut band_edges = [(1usize, 1usize); 32];
        for (b, edge) in band_edges.iter_mut().enumerate() {
            let f_lo = min_hz * ratio.powf(b as f32 / 32.0);
            let f_hi = min_hz * ratio.powf((b + 1) as f32 / 32.0);
            let lo = ((f_lo / bin_hz).floor() as usize).clamp(1, max_bin);
            let hi = ((f_hi / bin_hz).ceil() as usize).clamp(lo, max_bin);
            *edge = (lo, hi);
        }

        Self {
            ring: [0.0; FFT_SIZE],
            write_pos: 0,
            frames_since_emit: 0,
            hann,
            real: [0.0; FFT_SIZE],
            imag: [0.0; FFT_SIZE],
            band_edges,
        }
    }

    #[inline]
    fn push_mono(&mut self, sample: f32, target: &Arc<Mutex<SpectrumFrame>>) {
        self.ring[self.write_pos] = sample;
        self.write_pos = (self.write_pos + 1) % FFT_SIZE;
        self.frames_since_emit += 1;

        if self.frames_since_emit >= FFT_HOP {
            self.frames_since_emit = 0;
            let frame = self.compute_frame();
            if let Ok(mut guard) = target.try_lock() {
                *guard = frame;
            }
        }
    }

    fn compute_frame(&mut self) -> SpectrumFrame {
        let mut energy_sum = 0.0_f32;
        for i in 0..FFT_SIZE {
            let idx = (self.write_pos + i) % FFT_SIZE;
            let raw = self.ring[idx];
            energy_sum += raw * raw;
            self.real[i] = raw * self.hann[i];
            self.imag[i] = 0.0;
        }
        let rms = (energy_sum / FFT_SIZE as f32).sqrt().clamp(0.0, 1.0);

        fft_in_place(&mut self.real, &mut self.imag);

        let norm = FFT_SIZE as f32 * 0.25;
        let mut bands = [0.0_f32; 32];
        for (b, &(lo, hi)) in self.band_edges.iter().enumerate() {
            let mut sum = 0.0_f32;
            let count = (hi - lo + 1) as f32;
            for k in lo..=hi {
                let re = self.real[k];
                let im = self.imag[k];
                sum += (re * re + im * im).sqrt() / norm;
            }
            let mag = sum / count;
            // Perceptual log compression so quiet highs and loud bass look natural.
            let db_norm = ((1.0 + mag * 14.0).ln() / 3.4).clamp(0.0, 1.0);
            bands[b] = db_norm;
        }

        let bass = (bands[0] + bands[1] + bands[2] + bands[3] + bands[4]) / 5.0;
        SpectrumFrame {
            bands,
            peaks: bands,
            bass,
            rms,
        }
    }

    fn reset(&mut self) {
        self.ring.fill(0.0);
        self.write_pos = 0;
        self.frames_since_emit = 0;
    }
}

fn fft_in_place(real: &mut [f32; FFT_SIZE], imag: &mut [f32; FFT_SIZE]) {
    // Bit-reversal permutation
    let mut j = 0usize;
    for i in 1..FFT_SIZE {
        let mut bit = FFT_SIZE >> 1;
        while (j & bit) != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            real.swap(i, j);
            imag.swap(i, j);
        }
    }

    // Cooley-Tukey butterfly stages
    let mut len = 2usize;
    while len <= FFT_SIZE {
        let half = len >> 1;
        let angle_step = -TAU / len as f32;
        let w_step_re = angle_step.cos();
        let w_step_im = angle_step.sin();

        let mut start = 0usize;
        while start < FFT_SIZE {
            let mut w_re = 1.0_f32;
            let mut w_im = 0.0_f32;
            for k in 0..half {
                let even = start + k;
                let odd = even + half;
                let t_re = w_re * real[odd] - w_im * imag[odd];
                let t_im = w_re * imag[odd] + w_im * real[odd];

                real[odd] = real[even] - t_re;
                imag[odd] = imag[even] - t_im;
                real[even] += t_re;
                imag[even] += t_im;

                let next_re = w_re * w_step_re - w_im * w_step_im;
                w_im = w_re * w_step_im + w_im * w_step_re;
                w_re = next_re;
            }
            start += len;
        }
        len <<= 1;
    }
}

pub struct DspSource<S> {
    inner: S,
    channels: usize,
    sample_rate: u32,
    shared: Arc<SharedDspState>,
    spectrum: Arc<Mutex<SpectrumFrame>>,
    last_version: u64,
    eq_coeffs: [BiquadCoeffs; 5],
    eq_state_l: [BiquadState; 5],
    eq_state_r: [BiquadState; 5],
    reverb_mix: f32,
    reverb: StereoReverb,
    analyzer: FftAnalyzer,
    frame_chan: usize,
    pending_r: Option<f32>,
    last_left_raw: f32,
}

impl<S> DspSource<S>
where
    S: Source<Item = f32>,
{
    pub fn new(inner: S, shared: Arc<SharedDspState>, spectrum: Arc<Mutex<SpectrumFrame>>) -> Self {
        let channels = inner.channels().get().max(1) as usize;
        let sample_rate = inner.sample_rate().get().max(8000);
        let (ver, cfg) = shared.snapshot();
        let eq_coeffs = design_eq_bank(sample_rate as f32, &cfg.eq_gains);

        Self {
            inner,
            channels,
            sample_rate,
            shared,
            spectrum,
            last_version: ver,
            eq_coeffs,
            eq_state_l: [BiquadState::default(); 5],
            eq_state_r: [BiquadState::default(); 5],
            reverb_mix: cfg.reverb_mix.clamp(0.0, 1.0),
            reverb: StereoReverb::new(sample_rate),
            analyzer: FftAnalyzer::new(sample_rate),
            frame_chan: 0,
            pending_r: None,
            last_left_raw: 0.0,
        }
    }

    fn sync_config_if_needed(&mut self) {
        let ver = self.shared.version();
        if ver != self.last_version {
            let (new_ver, cfg) = self.shared.snapshot();
            self.last_version = new_ver;
            self.eq_coeffs = design_eq_bank(self.sample_rate as f32, &cfg.eq_gains);
            self.reverb_mix = cfg.reverb_mix.clamp(0.0, 1.0);
        }
    }

    #[inline]
    fn apply_eq_left(&mut self, mut x: f32) -> f32 {
        for (st, c) in self.eq_state_l.iter_mut().zip(self.eq_coeffs.iter()) {
            x = st.process(c, x);
        }
        x
    }

    #[inline]
    fn apply_eq_right(&mut self, mut x: f32) -> f32 {
        for (st, c) in self.eq_state_r.iter_mut().zip(self.eq_coeffs.iter()) {
            x = st.process(c, x);
        }
        x
    }

    fn reset_states(&mut self) {
        for st in &mut self.eq_state_l {
            st.reset();
        }
        for st in &mut self.eq_state_r {
            st.reset();
        }
        self.reverb.reset();
        self.analyzer.reset();
        self.pending_r = None;
        self.frame_chan = 0;
    }
}

fn design_eq_bank(sample_rate: f32, gains: &[f32; 5]) -> [BiquadCoeffs; 5] {
    [
        BiquadCoeffs::design(0, sample_rate, gains[0]),
        BiquadCoeffs::design(1, sample_rate, gains[1]),
        BiquadCoeffs::design(2, sample_rate, gains[2]),
        BiquadCoeffs::design(3, sample_rate, gains[3]),
        BiquadCoeffs::design(4, sample_rate, gains[4]),
    ]
}

impl<S> Iterator for DspSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.channels == 1 {
            let s = self.inner.next()?;
            self.sync_config_if_needed();
            let eq_s = self.apply_eq_left(s);
            let (out_l, _) = self.reverb.process_stereo(eq_s, eq_s, self.reverb_mix);
            let limited = soft_clip(out_l);
            self.analyzer.push_mono(limited, &self.spectrum);
            return Some(limited);
        }

        if self.frame_chan == 0 {
            self.sync_config_if_needed();
            let l_raw = self.inner.next()?;
            self.last_left_raw = l_raw;

            let r_raw = self.inner.next().unwrap_or(l_raw);
            let eq_l = self.apply_eq_left(l_raw);
            let eq_r = self.apply_eq_right(r_raw);
            let (out_l, out_r) = self.reverb.process_stereo(eq_l, eq_r, self.reverb_mix);
            let lim_l = soft_clip(out_l);
            let lim_r = soft_clip(out_r);

            self.analyzer.push_mono((lim_l + lim_r) * 0.5, &self.spectrum);
            self.pending_r = Some(lim_r);
            self.frame_chan = 1;
            return Some(lim_l);
        }

        if self.frame_chan == 1 {
            self.frame_chan = if self.channels == 2 { 0 } else { 2 };
            if let Some(r_out) = self.pending_r.take() {
                return Some(r_out);
            }
        }

        // Extra surround channels (>= 3) pass through cleanly
        let extra = self.inner.next()?;
        self.frame_chan += 1;
        if self.frame_chan >= self.channels {
            self.frame_chan = 0;
        }
        Some(soft_clip(extra))
    }
}

impl<S> Source for DspSource<S>
where
    S: Source<Item = f32>,
{
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        let res = self.inner.try_seek(pos);
        if res.is_ok() {
            self.reset_states();
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biquad_flat_is_identity() {
        let coeffs = design_eq_bank(44100.0, &[0.0; 5]);
        let mut states = [BiquadState::default(); 5];
        for &x in &[0.0_f32, 0.25, -0.5, 0.85, -0.12] {
            let mut y = x;
            for (st, c) in states.iter_mut().zip(coeffs.iter()) {
                y = st.process(c, y);
            }
            assert!((y - x).abs() < 1e-6);
        }
    }

    #[test]
    fn fft_detects_sine_frequency_band() {
        let sr = 44100_u32;
        let mut analyzer = FftAnalyzer::new(sr);
        let target = Arc::new(Mutex::new(SpectrumFrame::default()));

        // Feed 1024 samples of crisp 1 kHz sine (around upper-mid band) and 1024 of 100 Hz
        for i in 0..FFT_HOP {
            let t = i as f32 / sr as f32;
            let sample = (TAU * 1000.0 * t).sin() * 0.7;
            analyzer.push_mono(sample, &target);
        }
        let frame_1k = *target.lock().unwrap();
        let peak_1k_band = frame_1k
            .bands
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(idx, _)| idx)
            .unwrap();
        assert!(
            (13..=20).contains(&peak_1k_band),
            "expected 1kHz peak around mid bands 13..=20, got band {peak_1k_band}: {:?}",
            frame_1k.bands
        );
        assert!(frame_1k.rms > 0.35);
    }

    #[test]
    fn soft_clip_stays_bounded() {
        for raw in [-5.0_f32, -1.5, -0.95, 0.0, 0.5, 0.95, 2.0, 10.0, f32::NAN] {
            let y = soft_clip(raw);
            assert!(y.is_finite());
            assert!((-0.99..=0.99).contains(&y), "got out-of-range sample {y}");
        }
        assert!((soft_clip(0.42) - 0.42).abs() < 1e-6);
    }

    #[test]
    fn reverb_produces_decay_tail() {
        let mut rev = StereoReverb::new(44100);
        // Feed an initial impulse then silence and check tail appears later
        let _ = rev.process_stereo(1.0, 1.0, 0.6);
        let mut tail_energy = 0.0_f32;
        for _ in 0..3000 {
            let (l, r) = rev.process_stereo(0.0, 0.0, 0.6);
            tail_energy += l.abs() + r.abs();
        }
        assert!(tail_energy > 0.5, "expected audible reverb tail, got {tail_energy}");
    }
}
