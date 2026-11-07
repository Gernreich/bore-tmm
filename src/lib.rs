//! A transfer-matrix model of a wind instrument's bore.
//!
//! A bore is described from its driven end down: lengths of tube, each a
//! cylinder or a cone, tone holes along it, and how it ends. The drive is a
//! REED (a cane reed, or lips: the bore sounds where the impedance it sees
//! peaks, where Im(Y) rises through zero) or a JET (a flute's: the bore sounds
//! where the reactance the jet drives rises through zero).
//!
//! The physics is the four Python models' in the LaserMadeMusic repositories
//! (square-flute/naf_model.py, octagonal-flute/flute_model.py,
//! practice-chanters/chanter_model.py, ghb-chanter/ghb_chanter.py), in one
//! place: Benade's wall loss on a loss radius, plane waves in a cylinder,
//! spherical waves in a cone, Keefe's (1990) tone hole, a flanged or
//! unflanged open end. Those models DESIGN an instrument; this one only
//! answers what a described bore plays, which is the question a cut file
//! has to answer.

pub mod measure;
pub mod models;
pub mod py;

use serde::Deserialize;
use std::f64::consts::PI;
use std::ops::{Add, Div, Mul, Neg, Sub};

// --- complex numbers ---------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cx {
    pub re: f64,
    pub im: f64,
}

pub const ONE: Cx = Cx { re: 1.0, im: 0.0 };
pub const ZERO: Cx = Cx { re: 0.0, im: 0.0 };
pub const J: Cx = Cx { re: 0.0, im: 1.0 };

pub fn cx(re: f64, im: f64) -> Cx {
    Cx { re, im }
}

pub fn re(x: f64) -> Cx {
    Cx { re: x, im: 0.0 }
}

impl Add for Cx {
    type Output = Cx;
    fn add(self, o: Cx) -> Cx {
        cx(self.re + o.re, self.im + o.im)
    }
}
impl Sub for Cx {
    type Output = Cx;
    fn sub(self, o: Cx) -> Cx {
        cx(self.re - o.re, self.im - o.im)
    }
}
impl Mul for Cx {
    type Output = Cx;
    fn mul(self, o: Cx) -> Cx {
        cx(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
}
impl Mul<f64> for Cx {
    type Output = Cx;
    fn mul(self, o: f64) -> Cx {
        cx(self.re * o, self.im * o)
    }
}
impl Div for Cx {
    type Output = Cx;
    fn div(self, o: Cx) -> Cx {
        let d = o.re * o.re + o.im * o.im;
        cx(
            (self.re * o.re + self.im * o.im) / d,
            (self.im * o.re - self.re * o.im) / d,
        )
    }
}
impl Div<f64> for Cx {
    type Output = Cx;
    fn div(self, o: f64) -> Cx {
        cx(self.re / o, self.im / o)
    }
}
impl Neg for Cx {
    type Output = Cx;
    fn neg(self) -> Cx {
        cx(-self.re, -self.im)
    }
}

impl Cx {
    pub fn cos(self) -> Cx {
        cx(self.re.cos() * self.im.cosh(), -self.re.sin() * self.im.sinh())
    }
    pub fn sin(self) -> Cx {
        cx(self.re.sin() * self.im.cosh(), self.re.cos() * self.im.sinh())
    }
    pub fn tan(self) -> Cx {
        self.sin() / self.cos()
    }
    pub fn inv(self) -> Cx {
        ONE / self
    }
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }
}

/// A 2x2 transfer matrix on (pressure, volume flow): the state at a
/// segment's input is the matrix times the state at its output.
#[derive(Clone, Copy, Debug)]
pub struct M2(pub [[Cx; 2]; 2]);

pub const IDENT: M2 = M2([[ONE, ZERO], [ZERO, ONE]]);

impl Mul for M2 {
    type Output = M2;
    fn mul(self, n: M2) -> M2 {
        let m = self.0;
        let n = n.0;
        M2([
            [
                m[0][0] * n[0][0] + m[0][1] * n[1][0],
                m[0][0] * n[0][1] + m[0][1] * n[1][1],
            ],
            [
                m[1][0] * n[0][0] + m[1][1] * n[1][0],
                m[1][0] * n[0][1] + m[1][1] * n[1][1],
            ],
        ])
    }
}

impl M2 {
    /// The impedance at the input, for a load zl at the output.
    pub fn load(&self, zl: Cx) -> Cx {
        let m = self.0;
        (m[0][0] * zl + m[0][1]) / (m[1][0] * zl + m[1][1])
    }
}

// --- air -------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct Air {
    pub c: f64,
    pub rho: f64,
}

impl Air {
    /// The models' air: 331.3 m/s at 0C scaled by sqrt(T), and a fixed
    /// density, the one they all use.
    pub fn at(temp_c: f64) -> Air {
        Air { c: 331.3 * (1.0 + temp_c / 273.15).sqrt(), rho: 1.184 }
    }

    /// The wave number in a tube of loss radius rh (m): Benade's wall loss,
    /// 3.0e-5 sqrt(f) / rh per metre.
    pub fn k(&self, f: f64, rh: f64) -> Cx {
        cx(2.0 * PI * f / self.c, -3.0e-5 * f.sqrt() / rh)
    }
}

// --- the description ---------------------------------------------------------

/// A length of bore. Diameters are EQUIVALENT diameters, the round bore of the
/// same area, so any section is described by its area; loss_ratio is the
/// section's loss radius (2A/P, the hydraulic radius in the models' sense) over
/// its equivalent radius: 1 for a round bore, sqrt(pi / (n tan(pi/n))) for a
/// regular n-gon.
#[derive(Clone, Debug, Deserialize)]
pub struct Segment {
    pub len_mm: f64,
    #[serde(default)]
    pub d_mm: Option<f64>,
    #[serde(default)]
    pub d1_mm: Option<f64>,
    #[serde(default)]
    pub d2_mm: Option<f64>,
    #[serde(default = "one")]
    pub loss_ratio: f64,
}

fn one() -> f64 {
    1.0
}

impl Segment {
    pub fn ends(&self) -> (f64, f64) {
        match (self.d_mm, self.d1_mm, self.d2_mm) {
            (Some(d), None, None) => (d, d),
            (None, Some(a), Some(b)) => (a, b),
            _ => panic!("a segment takes d_mm, or d1_mm and d2_mm"),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Hole {
    pub name: String,
    /// down the bore from its driven end
    pub at_mm: f64,
    /// the round hole of the hole's area, whatever its shape
    pub d_mm: f64,
    /// the wall it goes through
    pub chimney_mm: f64,
    /// a vent: open for every note
    #[serde(default)]
    pub always_open: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Note {
    pub name: String,
    /// what it should play
    pub hz: f64,
    /// the holes open for it, by name (vents are open anyway)
    #[serde(default)]
    pub open: Vec<String>,
    /// the resonance is looked for between hz/span and hz*span
    #[serde(default)]
    pub span: Option<f64>,
    /// what the model that designed the bore says it plays, if it said
    #[serde(default)]
    pub model_hz: Option<f64>,
    /// this note's own tolerance, in place of check's --tol, and why
    #[serde(default)]
    pub tol_cents: Option<f64>,
    #[serde(default)]
    pub tol_why: Option<String>,
    /// look near here instead of hz: a note whose target is not where the
    /// bore's resonance is, as when the target is the reference's pitch
    #[serde(default)]
    pub near_hz: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Drive {
    /// A reed, or lips, at the top of the bore. apex: the first segment's cone
    /// carries on above the top to its apex, closed there -- a conical bore's
    /// reed stands in for the missing tip.
    Reed {
        #[serde(default)]
        apex: bool,
    },
    /// A flute's jet, blowing across a window or embouchure hole AT the top of
    /// the bore's described length, with a closed stub (to the plug or the
    /// cork) of the first segment's section beside it. The hole is a neck of
    /// area_mm2, chimney_mm deep, flanged both ends (0.82 a each), its end
    /// correction scaled by mouth.
    Jet {
        area_mm2: f64,
        chimney_mm: f64,
        #[serde(default = "one")]
        mouth: f64,
        stub_mm: f64,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum End {
    Unflanged,
    Flanged,
    Closed,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Instrument {
    pub name: String,
    #[serde(default = "warm")]
    pub temp_c: f64,
    pub drive: Drive,
    pub bore: Vec<Segment>,
    pub end: End,
    #[serde(default)]
    pub holes: Vec<Hole>,
    pub notes: Vec<Note>,
    /// where the description came from: a generator, a model, a measurement
    #[serde(default)]
    pub source: Option<String>,
    /// what the notes' hz are, when not the scale itself: a reference
    /// chanter's notes in the same model, say
    #[serde(default)]
    pub targets: Option<String>,
}

fn warm() -> f64 {
    25.0
}

// --- the elements --------------------------------------------------------------

/// A cylinder of equivalent radius a (m), loss radius rh (m), length l (m).
pub fn cylinder(air: &Air, f: f64, a: f64, rh: f64, l: f64) -> M2 {
    let k = air.k(f, rh);
    let z = air.rho * air.c / (PI * a * a);
    let (c, s) = ((k * l).cos(), (k * l).sin());
    M2([[c, J * s * z], [J * s / z, c]])
}

/// The cone's two spherical waves at x from its apex (signed, m), cos(kx)/x and
/// sin(kx)/x, as columns of (p, U); slope is the radius's per unit length.
fn phi(air: &Air, k: Cx, w: f64, slope: f64, x: f64) -> M2 {
    let s_area = PI * slope * slope * x * x;
    let (c, s) = ((k * x).cos(), (k * x).sin());
    let da = -(k * s) / x - c / (x * x);
    let db = (k * c) / x - s / (x * x);
    let q = -re(s_area) / (J * (w * air.rho));
    M2([[c / x, s / x], [q * da, q * db]])
}

fn inverse(m: M2) -> M2 {
    let m = m.0;
    let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
    M2([[m[1][1] / det, -m[0][1] / det], [-m[1][0] / det, m[0][0] / det]])
}

/// A cone from radius a1 to a2 (m) over l (m), loss radius rh.
pub fn cone(air: &Air, f: f64, a1: f64, a2: f64, rh: f64, l: f64) -> M2 {
    let slope = (a2 - a1) / l;
    if slope.abs() * l < 1e-9 * a1.max(a2) {
        return cylinder(air, f, (a1 + a2) / 2.0, rh, l);
    }
    let w = 2.0 * PI * f;
    let k = air.k(f, rh);
    let (x1, x2) = (a1 / slope, a2 / slope);
    phi(air, k, w, slope, x1) * inverse(phi(air, k, w, slope, x2))
}

/// The admittance looking up a cone from radius a (m) to its apex, closed
/// there: the wave sin(kx)/x alone.
pub fn apex_admittance(air: &Air, f: f64, a: f64, slope: f64, rh: f64) -> Cx {
    let w = 2.0 * PI * f;
    let k = air.k(f, rh);
    let p = phi(air, k, w, slope, a / slope).0;
    -(p[1][1] / p[0][1])
}

/// Keefe's (1990) tone hole of radius r (m) through a wall t (m) in a bore of
/// equivalent radius a (m): series half, shunt, series half.
pub fn tone_hole(air: &Air, f: f64, a: f64, r: f64, t: f64, open: bool) -> M2 {
    let k = 2.0 * PI * f / air.c;
    let dl = r / a;
    let zh = air.rho * air.c / (PI * r * r);
    let (zs, ta) = if open {
        let te = ((1.0 / k) * (k * t).tan() + r * (1.40 - 0.58 * dl * dl))
            / (1.0 - 0.61 * k * r * (k * t).tan());
        let zs = cx(0.25 * (k * r) * (k * r), k * te) * zh;
        let ta = 0.47 * r * dl.powi(4)
            / ((1.84 * t / r).tanh() + 0.62 * dl * dl + 0.64 * dl);
        (zs, ta)
    } else {
        let tc = t + r * dl * (1.0 + 0.207 * dl.powi(3)) / 8.0;
        let zs = cx(0.0, -zh / (k * tc).tan());
        let ta = 0.47 * r * dl.powi(4)
            / (1.0 / (1.84 * t / r).tanh() + 0.62 * dl * dl + 0.64 * dl);
        (zs, ta)
    };
    let za = cx(0.0, -zh * k * ta);
    let half = M2([[ONE, za / 2.0], [ZERO, ONE]]);
    half * M2([[ONE, ZERO], [zs.inv(), ONE]]) * half
}

/// The radiation impedance of an open end of radius a (m).
pub fn radiation(air: &Air, f: f64, a: f64, end: End) -> Cx {
    let z0 = air.rho * air.c / (PI * a * a);
    let ka = 2.0 * PI * f / air.c * a;
    match end {
        End::Unflanged => cx(0.25 * ka * ka, 0.6133 * ka) * z0,
        End::Flanged => cx(0.5 * ka * ka, 0.8216 * ka) * z0,
        // a closed end, as a very large impedance
        End::Closed => cx(0.0, -1e30),
    }
}

// --- the bore -----------------------------------------------------------------------

impl Instrument {
    pub fn length_mm(&self) -> f64 {
        self.bore.iter().map(|s| s.len_mm).sum()
    }

    /// The equivalent diameter and loss ratio at x mm down the bore.
    pub fn section_at(&self, x: f64) -> (f64, f64) {
        let mut z = 0.0;
        for s in &self.bore {
            if x <= z + s.len_mm || std::ptr::eq(s, self.bore.last().unwrap()) {
                let (d1, d2) = s.ends();
                let u = ((x - z) / s.len_mm).clamp(0.0, 1.0);
                return (d1 + (d2 - d1) * u, s.loss_ratio);
            }
            z += s.len_mm;
        }
        unreachable!()
    }

    pub fn check(&self) -> Result<(), String> {
        if self.bore.is_empty() {
            return Err("the bore has no segments".into());
        }
        for (i, s) in self.bore.iter().enumerate() {
            let ok = matches!(
                (s.d_mm, s.d1_mm, s.d2_mm),
                (Some(_), None, None) | (None, Some(_), Some(_))
            );
            if !ok {
                return Err(format!("segment {}: give d_mm, or d1_mm and d2_mm", i + 1));
            }
            if !(s.len_mm >= 0.0) {
                return Err(format!("segment {}: a negative length", i + 1));
            }
        }
        let l = self.length_mm();
        for h in &self.holes {
            if !(h.at_mm > 0.0 && h.at_mm < l) {
                return Err(format!("hole {} at {}mm is off the {:.1}mm bore", h.name, h.at_mm, l));
            }
        }
        for n in &self.notes {
            for o in &n.open {
                if !self.holes.iter().any(|h| &h.name == o) {
                    return Err(format!("note {} opens {}, which is not a hole", n.name, o));
                }
            }
        }
        Ok(())
    }

    /// The bore's input impedance at the top of its described length, f Hz,
    /// the named holes open. The drive's own parts (the jet's neck and stub, the
    /// reed's apex) are not included.
    pub fn bore_z(&self, air: &Air, f: f64, open: &[String]) -> Cx {
        // cut the bore into pieces at the holes, each piece a cylinder or cone
        let mut holes: Vec<&Hole> = self.holes.iter().collect();
        holes.sort_by(|a, b| a.at_mm.partial_cmp(&b.at_mm).unwrap());
        let mut cuts: Vec<f64> = vec![0.0];
        let mut z = 0.0;
        for s in &self.bore {
            z += s.len_mm;
            cuts.push(z);
        }
        let mut m = IDENT;
        let mut x = 0.0;
        let mut hi = 0;
        let mut seg = 0;
        let mut seg_top = 0.0;
        loop {
            // the next stop: a hole or the end of this segment
            let seg_end = seg_top + self.bore[seg].len_mm;
            let next_hole = holes.get(hi).map(|h| h.at_mm);
            let stop = match next_hole {
                Some(h) if h <= seg_end => h,
                _ => seg_end,
            };
            if stop > x {
                let (d1, d2) = self.bore[seg].ends();
                let s = &self.bore[seg];
                let at = |p: f64| d1 + (d2 - d1) * (p - seg_top) / s.len_mm;
                let (a1, a2) = (at(x) / 2000.0, at(stop) / 2000.0);
                let rh = s.loss_ratio * (a1 + a2) / 2.0;
                m = m * cone(air, f, a1, a2, rh, (stop - x) / 1000.0);
                x = stop;
            }
            if next_hole.map_or(false, |h| h <= seg_end) {
                let h = holes[hi];
                let (d, _) = self.section_at(h.at_mm);
                let is_open = h.always_open || open.iter().any(|o| o == &h.name);
                m = m * tone_hole(air, f, d / 2000.0, h.d_mm / 2000.0, h.chimney_mm / 1000.0, is_open);
                hi += 1;
                continue;
            }
            seg += 1;
            seg_top = seg_end;
            if seg == self.bore.len() {
                break;
            }
        }
        let (d_end, _) = self.section_at(self.length_mm());
        m.load(radiation(air, f, d_end / 2000.0, self.end))
    }

    /// The quantity that rises through zero where the bore sounds: Im(Y) at
    /// a reed, the reactance the jet drives at a flute.
    pub fn drive_x(&self, air: &Air, f: f64, open: &[String]) -> f64 {
        let zin = self.bore_z(air, f, open);
        let s0 = &self.bore[0];
        let (d1, d2) = s0.ends();
        match self.drive {
            Drive::Reed { apex } => {
                let mut y = zin.inv();
                if apex {
                    let a = d1 / 2000.0;
                    let slope = (d2 - d1) / 2000.0 / (s0.len_mm / 1000.0);
                    // the loss radius at the throat, the cone's own
                    y = y + apex_admittance(air, f, a, slope, s0.loss_ratio * a);
                }
                y.im
            }
            Drive::Jet { area_mm2, chimney_mm, mouth, stub_mm } => {
                let w = 2.0 * PI * f;
                let k = w / air.c;
                let aw_area = area_mm2 / 1e6;
                let aw = (aw_area / PI).sqrt();
                let leff = chimney_mm / 1000.0 + mouth * 2.0 * 0.82 * aw;
                let zw = cx(
                    air.rho * air.c / aw_area * (0.5 * (k * aw) * (k * aw)),
                    w * air.rho * leff / aw_area,
                );
                let a = d1 / 2000.0;
                let z0 = air.rho * air.c / (PI * a * a);
                let kb = air.k(f, s0.loss_ratio * a);
                let stub = -(J * z0) / (kb * (stub_mm / 1000.0)).tan();
                (zw + (zin.inv() + stub.inv()).inv()).im
            }
        }
    }

    /// The note a fingering plays nearest `near`: a rising zero of drive_x,
    /// found by scanning between near/span and near*span, then bisecting.
    pub fn sounding(&self, air: &Air, open: &[String], near: f64, span: f64) -> Option<f64> {
        let (lo, hi) = (near / span, near * span);
        let n = 240;
        let g = |f: f64| self.drive_x(air, f, open);
        let (mut pf, mut py) = (lo, g(lo));
        let mut best: Option<f64> = None;
        for i in 1..=n {
            let f = lo + (hi - lo) * i as f64 / n as f64;
            let y = g(f);
            if py < 0.0 && y >= 0.0 {
                let (mut a, mut c) = (pf, f);
                for _ in 0..60 {
                    let mid = (a + c) / 2.0;
                    if g(mid) < 0.0 {
                        a = mid;
                    } else {
                        c = mid;
                    }
                }
                let r = (a + c) / 2.0;
                if best.map_or(true, |b| (r - near).abs() < (b - near).abs()) {
                    best = Some(r);
                }
            }
            pf = f;
            py = y;
        }
        best
    }

    /// Every resonance between lo and hi Hz for a fingering: its registers.
    pub fn resonances(&self, air: &Air, open: &[String], lo: f64, hi: f64) -> Vec<f64> {
        let n = ((hi - lo) / 2.0).ceil().max(50.0) as usize;
        let g = |f: f64| self.drive_x(air, f, open);
        let mut out = vec![];
        let (mut pf, mut py) = (lo, g(lo));
        for i in 1..=n {
            let f = lo + (hi - lo) * i as f64 / n as f64;
            let y = g(f);
            if py < 0.0 && y >= 0.0 {
                let (mut a, mut c) = (pf, f);
                for _ in 0..60 {
                    let mid = (a + c) / 2.0;
                    if g(mid) < 0.0 {
                        a = mid;
                    } else {
                        c = mid;
                    }
                }
                out.push((a + c) / 2.0);
            }
            pf = f;
            py = y;
        }
        out
    }
}

pub fn cents(f: f64, reference: f64) -> f64 {
    1200.0 * (f / reference).log2()
}
