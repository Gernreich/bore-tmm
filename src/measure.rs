//! Measuring a built bore: a swept sine to play into it, and the analysis that
//! turns a recording of it back into the bore's resonances.
//!
//! The rig (README, "Measuring a bore") drives the bore's input through a
//! capillary, a narrow tube whose impedance is far above the bore's, so the
//! flow into the bore barely depends on the bore: the pressure a microphone
//! hears at the input is then the bore's input impedance times a constant flow.
//! Its peaks are the bore's resonances, the same peaks `bore-tmm curve`
//! computes, with no reed or lip in the way.
//!
//! The sweep is exponential (Farina, 2000): a recording divided by the sweep
//! in the frequency domain gives the transfer from the sweep to the
//! microphone, the room and the loudspeaker and the microphone included. A
//! second recording of a REFERENCE, a closed tube of known size, measured the
//! same way, cancels them: the bore's impedance is the ratio of the two
//! transfers times the reference's computed impedance. Without a reference
//! the peaks still sit near the right frequencies, but the loudspeaker's own
//! response tilts them.

use crate::{cx, Cx, ZERO};
use std::f64::consts::PI;

// --- FFT -------------------------------------------------------------------------

/// In place, radix 2; the length must be a power of two. inverse divides by n.
pub fn fft(a: &mut [Cx], inverse: bool) {
    let n = a.len();
    assert!(n.is_power_of_two(), "fft length {n} is not a power of two");
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            a.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = 2.0 * PI / len as f64 * if inverse { 1.0 } else { -1.0 };
        let wl = cx(ang.cos(), ang.sin());
        for i in (0..n).step_by(len) {
            let mut w = cx(1.0, 0.0);
            for k in 0..len / 2 {
                let u = a[i + k];
                let v = a[i + k + len / 2] * w;
                a[i + k] = u + v;
                a[i + k + len / 2] = u - v;
                w = w * wl;
            }
        }
        len <<= 1;
    }
    if inverse {
        for x in a.iter_mut() {
            *x = *x / n as f64;
        }
    }
}

// --- the sweep ---------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct Sweep {
    pub rate: u32,
    pub f1: f64,
    pub f2: f64,
    pub secs: f64,
}

impl Sweep {
    /// The exponential sweep, with 10ms fades at each end against clicks.
    pub fn samples(&self) -> Vec<f64> {
        let n = (self.secs * self.rate as f64) as usize;
        let k = (self.f2 / self.f1).ln();
        let fade = (0.01 * self.rate as f64) as usize;
        (0..n)
            .map(|i| {
                let t = i as f64 / self.rate as f64;
                let ph = 2.0 * PI * self.f1 * self.secs / k * ((t / self.secs * k).exp() - 1.0);
                let g = if i < fade {
                    i as f64 / fade as f64
                } else if i + fade > n {
                    (n - i) as f64 / fade as f64
                } else {
                    1.0
                };
                0.5 * g * ph.sin()
            })
            .collect()
    }
}

// --- WAV -------------------------------------------------------------------------------

pub fn write_wav(path: &str, x: &[f64], rate: u32) -> std::io::Result<()> {
    let mut b: Vec<u8> = vec![];
    let data = (x.len() * 2) as u32;
    b.extend(b"RIFF");
    b.extend((36 + data).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes()); // PCM
    b.extend(1u16.to_le_bytes()); // mono
    b.extend(rate.to_le_bytes());
    b.extend((rate * 2).to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(data.to_le_bytes());
    for &v in x {
        let s = (v.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        b.extend(s.to_le_bytes());
    }
    std::fs::write(path, b)
}

/// A WAV's first channel and its rate: PCM 16, 24 or 32 bit, or 32-bit float.
pub fn read_wav(path: &str) -> Result<(Vec<f64>, u32), String> {
    let b = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err(format!("{path}: not a WAV file"));
    }
    let u16_at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let u32_at = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let (mut fmt, mut chans, mut rate, mut bits) = (0u16, 0u16, 0u32, 0u16);
    let mut i = 12;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let len = u32_at(i + 4) as usize;
        let body = i + 8;
        if id == b"fmt " {
            fmt = u16_at(body);
            chans = u16_at(body + 2);
            rate = u32_at(body + 4);
            bits = u16_at(body + 14);
            if fmt == 0xfffe && len >= 26 {
                fmt = u16_at(body + 24); // WAVE_FORMAT_EXTENSIBLE: the sub-format
            }
        } else if id == b"data" {
            if chans == 0 {
                return Err(format!("{path}: data before fmt"));
            }
            let width = (bits / 8) as usize;
            let frame = width * chans as usize;
            let end = (body + len).min(b.len());
            let mut out = Vec::with_capacity(len / frame.max(1));
            let mut p = body;
            while p + frame <= end {
                let v = match (fmt, bits) {
                    (1, 16) => i16::from_le_bytes([b[p], b[p + 1]]) as f64 / 32768.0,
                    (1, 24) => {
                        (i32::from_le_bytes([0, b[p], b[p + 1], b[p + 2]]) >> 8) as f64 / 8388608.0
                    }
                    (1, 32) => i32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]]) as f64
                        / 2147483648.0,
                    (3, 32) => f32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]]) as f64,
                    _ => return Err(format!("{path}: format {fmt}, {bits} bits: not read")),
                };
                out.push(v);
                p += frame;
            }
            return Ok((out, rate));
        }
        i = body + len + (len & 1);
    }
    Err(format!("{path}: no data chunk"))
}

// --- analysis ----------------------------------------------------------------------------

/// The transfer from the sweep to a recording, |H| on a grid of frequencies:
/// the recording's spectrum over the sweep's, regularised where the sweep
/// has no energy, then smoothed over `smooth` of an octave.
pub fn transfer(sweep: &Sweep, rec: &[f64], freqs: &[f64], smooth: f64) -> Vec<f64> {
    let x = sweep.samples();
    let n = (x.len().max(rec.len()) * 2).next_power_of_two();
    let mut a: Vec<Cx> = (0..n).map(|i| cx(*x.get(i).unwrap_or(&0.0), 0.0)).collect();
    let mut r: Vec<Cx> = (0..n).map(|i| cx(*rec.get(i).unwrap_or(&0.0), 0.0)).collect();
    fft(&mut a, false);
    fft(&mut r, false);
    let peak = a.iter().map(|v| v.abs()).fold(0.0, f64::max);
    let eps = (peak * 1e-3).powi(2);
    let df = sweep.rate as f64 / n as f64;
    let h: Vec<f64> = (0..n / 2)
        .map(|k| {
            let num = r[k] * cx(a[k].re, -a[k].im);
            (num / (a[k].abs().powi(2) + eps)).abs()
        })
        .collect();
    freqs
        .iter()
        .map(|&f| {
            let (lo, hi) = (f * 2f64.powf(-smooth / 2.0), f * 2f64.powf(smooth / 2.0));
            let (k0, k1) = ((lo / df).floor() as usize, ((hi / df).ceil() as usize).min(h.len() - 1));
            let k0 = k0.min(k1);
            let s: f64 = h[k0..=k1].iter().map(|v| v * v).sum();
            (s / (k1 - k0 + 1) as f64).sqrt()
        })
        .collect()
}

/// Local maxima of y over the grid, refined by a parabola through each and
/// its neighbours on a log-frequency axis; only peaks standing `min_db`
/// above both neighbouring minima count.
pub fn peaks(freqs: &[f64], y: &[f64], min_db: f64) -> Vec<f64> {
    let db: Vec<f64> = y.iter().map(|v| 20.0 * v.max(1e-300).log10()).collect();
    let mut out = vec![];
    for i in 1..db.len() - 1 {
        if db[i] > db[i - 1] && db[i] >= db[i + 1] {
            // prominence: the drop to the lowest point before a higher peak, each side
            let side = |it: &mut dyn Iterator<Item = &f64>| {
                let mut m = db[i];
                for &v in it {
                    if v > db[i] {
                        break;
                    }
                    m = m.min(v);
                }
                m
            };
            let lmin = side(&mut db[..i].iter().rev());
            let rmin = side(&mut db[i + 1..].iter());
            if db[i] - lmin.max(rmin) < min_db {
                continue;
            }
            let (x0, x1, x2) = (freqs[i - 1].ln(), freqs[i].ln(), freqs[i + 1].ln());
            let (y0, y1, y2) = (db[i - 1], db[i], db[i + 1]);
            let den = (x0 - x1) * (x0 - x2) * (x1 - x2);
            let a = (x2 * (y1 - y0) + x1 * (y0 - y2) + x0 * (y2 - y1)) / den;
            let b = (x2 * x2 * (y0 - y1) + x1 * x1 * (y2 - y0) + x0 * x0 * (y1 - y2)) / den;
            let xv = if a < 0.0 { -b / (2.0 * a) } else { x1 };
            out.push(xv.clamp(x0, x2).exp());
        }
    }
    out
}

/// A log-spaced grid, `per_oct` points an octave.
pub fn log_grid(f1: f64, f2: f64, per_oct: usize) -> Vec<f64> {
    let n = ((f2 / f1).log2() * per_oct as f64).ceil() as usize;
    (0..=n).map(|i| f1 * 2f64.powf(i as f64 / per_oct as f64)).collect()
}

/// A recording the rig WOULD make of a bore whose input impedance is z(f):
/// the sweep through a constant-flow source into z, as a convolution. For
/// testing the analysis against the model, not for anything measured.
pub fn simulate(sweep: &Sweep, z: impl Fn(f64) -> Cx, tail_secs: f64) -> Vec<f64> {
    let x = sweep.samples();
    let total = x.len() + (tail_secs * sweep.rate as f64) as usize;
    let n = total.next_power_of_two();
    let mut a: Vec<Cx> = (0..n).map(|i| cx(*x.get(i).unwrap_or(&0.0), 0.0)).collect();
    fft(&mut a, false);
    let df = sweep.rate as f64 / n as f64;
    let mut scale = 0.0f64;
    let zs: Vec<Cx> = (0..=n / 2)
        .map(|k| {
            let f = k as f64 * df;
            if f < 10.0 {
                return ZERO;
            }
            let v = z(f);
            scale = scale.max(v.abs());
            v
        })
        .collect();
    for k in 0..=n / 2 {
        let h = zs[k] / scale;
        a[k] = a[k] * h;
        if k > 0 && k < n / 2 {
            a[n - k] = cx(a[k].re, -a[k].im);
        }
    }
    fft(&mut a, true);
    a.iter().take(total).map(|v| v.re).collect()
}
