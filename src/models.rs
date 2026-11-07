//! The design models' inner loop, ported from Python to give the same bits.
//!
//! Five models design the chanters and flutes. Each finds a note by scanning
//! a frequency range for a rising zero of the bore's admittance (or the jet's
//! reactance) and bisecting it, and each design does that a few thousand
//! times while it narrows hole positions and widths. The loop here is the
//! models' arithmetic in the same order (py.rs says how), so it returns the
//! same float, and the cut files drawn from it do not change.
//!
//! Three kernels serve the five:
//!
//!   reed  practice-chanters' chanter_model.rs and standard_chanter.rs:
//!         a closed tube for the reed, then bore and holes, a flanged end
//!   cone  ghb-chanter's ghb_chanter.rs: spherical waves in a cone closed
//!         at its apex, holes and vents along it
//!   jet   square-flute/naf_model.py and octagonal-flute/flute_model.py:
//!         a jet driving a neck, the bore below it, a closed stub above
//!
//! The chanters are Rust since 2026-11-07 and call the reed and cone
//! kernels directly; until then they were Python, which these gave the same
//! bits as, and their generators' cut files came out byte for byte the same
//! when they moved. The two flute models are still Python: each hands over
//! only what does not depend on frequency, computed by its own expressions
//! (hole radii, segment lengths), and keeps its pure-Python path.
//! python/tmm_fast.py loads the jet through the C interface below, and
//! tools/check_fast.py compares the two paths bit for bit.
//!
//! This is NOT the bore model in lib.rs. That one is general (cones,
//! segments, any bore) and agrees with these models to 0.01 cents, which is
//! as close as a different formulation can; this one is their arithmetic.

use crate::py::*;
use std::f64::consts::PI;

fn f(x: f64) -> N {
    F(x)
}

/// A bore's wave number: 2 pi f / C - 1j 3.0e-5 sqrt(f) / rh
fn wave(fr: f64, rh: f64, c: f64) -> N {
    I(2) * f(PI) * f(fr) / f(c) - J1 * f(3.0e-5) * f(msqrt(fr)) / f(rh)
}

/// tube(b, f, L), for a bore of loss radius rh and area area (m, m^2)
fn tube(fr: f64, rh: f64, area: f64, l: f64, c: f64, rho: f64) -> Mx {
    let k = wave(fr, rh, c);
    let z = f(rho) * f(c) / f(area);
    let (cs, sn) = (ccos(k * f(l)), csin(k * f(l)));
    [[cs, J1 * z * sn], [J1 * sn / z, cs]]
}

fn pw(x: N, y: f64) -> N {
    F(powf(x.real(), y))
}

/// Keefe's tone hole, hole(b, f, w, t, is_open), for a hole of radius r (m)
/// in a bore of equivalent radius a (m), through a chimney t mm.
fn hole(fr: f64, r: f64, a: f64, t: f64, open: bool, c: f64, rho: f64) -> Mx {
    let k = I(2) * f(PI) * f(fr) / f(c);
    let (r, a) = (f(r), f(a));
    let t = f(t) / I(1000);
    let dl = r / a;
    let zh = f(rho) * f(c) / (f(PI) * r * r);
    let tk = F(mtan((k * t).real()));
    let (zs, ta);
    if open {
        let te = (I(1) / k * tk + r * (f(1.40) - f(0.58) * pw(dl, 2.0)))
            / (I(1) - f(0.61) * k * r * tk);
        zs = zh * (J1 * k * te + f(0.25) * pw(k * r, 2.0));
        ta = f(0.47) * r * pw(dl, 4.0)
            / (F(mtanh((f(1.84) * t / r).real())) + f(0.62) * pw(dl, 2.0) + f(0.64) * dl);
    } else {
        let tc = t + r * dl * (I(1) + f(0.207) * pw(dl, 3.0)) / I(8);
        zs = -J1 * zh / F(mtan((k * tc).real()));
        ta = f(0.47) * r * pw(dl, 4.0)
            / (I(1) / F(mtanh((f(1.84) * t / r).real())) + f(0.62) * pw(dl, 2.0) + f(0.64) * dl);
    }
    let za = -J1 * zh * k * ta;
    let half = [[I(1), za / I(2)], [I(0), I(1)]];
    mul(mul(half, [[I(1), I(0)], [I(1) / zs, I(1)]]), half)
}

/// A flanged or unflanged open end's impedance: z (q (ka)^2 + 1j j ka)
fn radiation(fr: f64, z: N, a: f64, q: f64, j: f64, c: f64) -> N {
    let ka = I(2) * f(PI) * f(fr) / f(c) * f(a);
    z * (f(q) * pw(ka, 2.0) + J1 * f(j) * ka)
}

fn zin(m: Mx, zr: N) -> N {
    (m[0][0] * zr + m[0][1]) / (m[1][0] * zr + m[1][1])
}

/// sounding()'s search: scan n steps across [near/span, near*span] for a
/// rising zero, bisect each one bis times, keep the root nearest `near`.
/// NaN when there is none, and the Python side raises its own error.
pub fn scan(near: f64, span: f64, n: i64, bis: i64, y: impl Fn(f64) -> f64) -> f64 {
    let (lo, hi) = (near / span, near * span);
    let (mut pf, mut py) = (lo, y(lo));
    let mut best: Option<f64> = None;
    for i in 1..=n {
        let fr = lo + (hi - lo) * i as f64 / n as f64;
        let yy = y(fr);
        if py < 0.0 && 0.0 <= yy {
            let (mut a, mut c) = (pf, fr);
            for _ in 0..bis {
                let mid = (a + c) / 2.0;
                if y(mid) < 0.0 {
                    a = mid;
                } else {
                    c = mid;
                }
            }
            let root = (a + c) / 2.0;
            if best.map_or(true, |b| (root - near).abs() < (b - near).abs()) {
                best = Some(root);
            }
        }
        pf = fr;
        py = yy;
    }
    best.unwrap_or(f64::NAN)
}

// --- the reed: chanter_model's and standard_chanter's admittance -------------

pub struct Reed<'a> {
    pub c: f64,
    pub rho: f64,
    /// the reed's closed tube: its bore's loss radius and area, its length (m)
    pub rh0: f64,
    pub area0: f64,
    pub l0: f64,
    /// the bore: loss radius, area, equivalent radius
    pub rh: f64,
    pub area: f64,
    pub a: f64,
    /// down the bore: the tube before each hole (m), the hole's radius (m),
    /// its chimney (mm), whether it is open; then the last tube to the end
    pub seg: &'a [f64],
    pub r: &'a [f64],
    pub t: &'a [f64],
    pub open: &'a [u8],
    pub last: f64,
    /// the end's radiation: q (ka)^2 + 1j j ka
    pub q: f64,
    pub j: f64,
}

impl Reed<'_> {
    pub fn y(&self, fr: f64) -> f64 {
        let (c, rho) = (self.c, self.rho);
        let z = f(rho) * f(c) / f(self.area);
        let zr = radiation(fr, z, self.a, self.q, self.j, c);
        let mut m = tube(fr, self.rh0, self.area0, self.l0, c, rho);
        for i in 0..self.seg.len() {
            m = mul(m, tube(fr, self.rh, self.area, self.seg[i], c, rho));
            m = mul(m, hole(fr, self.r[i], self.a, self.t[i], self.open[i] != 0, c, rho));
        }
        m = mul(m, tube(fr, self.rh, self.area, self.last, c, rho));
        (I(1) / zin(m, zr)).imag()
    }
}

// --- the cone: ghb_chanter's admittance ---------------------------------------

pub struct Cone<'a> {
    pub c: f64,
    pub rho: f64,
    /// REF_CONE: the bore across at the seat (mm) and its slope; the apex
    /// and the sole, mm down from the seat
    pub c0: f64,
    pub c1: f64,
    pub apex: f64,
    pub sole: f64,
    /// down the bore: each hole's depth (mm), radius (m), the bore's
    /// equivalent radius there (m), chimney (mm), whether open
    pub zh: &'a [f64],
    pub r: &'a [f64],
    pub a: &'a [f64],
    pub t: &'a [f64],
    pub open: &'a [u8],
}

impl Cone<'_> {
    /// phi(k, w, z): the cone's two spherical waves at z
    fn phi(&self, k: N, w: N, z: N) -> Mx {
        let x = (z - f(self.apex)) / I(1000);
        let s_area = f(PI) * pw(f(self.c1) / I(2), 2.0) * x * x;
        let (cs, sn) = (ccos(k * x), csin(k * x));
        let da = -k * sn / x - cs / (x * x);
        let db = k * cs / x - sn / (x * x);
        let q = -s_area / (J1 * w * f(self.rho));
        [[cs / x, sn / x], [q * da, q * db]]
    }

    fn cone(&self, fr: f64, z1: N, z2: N) -> Mx {
        let w = I(2) * f(PI) * f(fr);
        let rh = (f(self.c0) + f(self.c1) * (z1 + z2) / I(2)) / I(2000);
        let k = w / f(self.c) - J1 * f(3.0e-5) * f(msqrt(fr)) / rh;
        let (a, b) = (self.phi(k, w, z1), self.phi(k, w, z2));
        let det = b[0][0] * b[1][1] - b[0][1] * b[1][0];
        let inv = [[b[1][1] / det, -b[0][1] / det], [-b[1][0] / det, b[0][0] / det]];
        mul(a, inv)
    }

    pub fn y(&self, fr: f64) -> f64 {
        let (c, rho) = (self.c, self.rho);
        let mut z = f(0.0);
        let mut m = IDENT;
        for i in 0..self.zh.len() {
            m = mul(m, self.cone(fr, z, f(self.zh[i])));
            m = mul(m, hole(fr, self.r[i], self.a[i], self.t[i], self.open[i] != 0, c, rho));
            z = f(self.zh[i]);
        }
        m = mul(m, self.cone(fr, z, f(self.sole)));
        let a = (f(self.c0) + f(self.c1) * f(self.sole)) / I(2000);
        let z0 = f(rho) * f(c) / (f(PI) * a * a);
        let zr = radiation(fr, z0, a.real(), 0.25, 0.6133, c);
        let zi = zin(m, zr);
        let w = I(2) * f(PI) * f(fr);
        let rh = f(self.c0) / I(2000);
        let k = w / f(c) - J1 * f(3.0e-5) * f(msqrt(fr)) / rh;
        let p = self.phi(k, w, f(0.0));
        (I(1) / zi - p[1][1] / p[0][1]).imag()
    }
}

// --- the jet: naf_model.reactance, flute_model.reactance ----------------------

pub struct Jet<'a> {
    pub c: f64,
    pub rho: f64,
    /// the bore: loss radius, area, equivalent radius
    pub rh: f64,
    pub area: f64,
    pub a: f64,
    /// the chain from the jet down: kind 0 a tube of len (m), kind 1 a hole
    /// of radius r (m), open or not, through the wall t (mm)
    pub kind: &'a [u8],
    pub len: &'a [f64],
    pub r: &'a [f64],
    pub open: &'a [u8],
    pub t: f64,
    /// the closed stub above the jet (m)
    pub stub: f64,
    /// the neck the jet drives: its area (m^2), equivalent radius and
    /// effective length (m)
    pub neck_area: f64,
    pub neck_a: f64,
    pub leff: f64,
}

impl Jet<'_> {
    pub fn y(&self, fr: f64) -> f64 {
        let (c, rho) = (self.c, self.rho);
        let k = wave(fr, self.rh, c);
        let z = f(rho) * f(c) / f(self.area);
        let zr = radiation(fr, z, self.a, 0.25, 0.6133, c);
        let mut m = IDENT;
        for i in 0..self.kind.len() {
            m = mul(
                m,
                if self.kind[i] == 0 {
                    tube(fr, self.rh, self.area, self.len[i], c, rho)
                } else {
                    hole(fr, self.r[i], self.a, self.t, self.open[i] != 0, c, rho)
                },
            );
        }
        let zi = zin(m, zr);
        let stub = -J1 * z / ctan(k * f(self.stub));
        let zb = I(1) / (I(1) / zi + I(1) / stub);
        // window_z(f) / emb_z(f)
        let w = I(2) * f(PI) * f(fr);
        let k2 = w / f(c);
        let neck = f(rho) * f(c) / f(self.neck_area) * (f(0.5) * pw(k2 * f(self.neck_a), 2.0))
            + J1 * w * f(rho) * f(self.leff) / f(self.neck_area);
        (neck + zb).imag()
    }
}

// --- the C interface, for python/tmm_fast.py: the jet, for the two flutes ----

unsafe fn sl<'a, T>(p: *const T, n: usize) -> &'a [T] {
    if n == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(p, n)
    }
}

/// Jet: the resonance nearest `near` (sounding), or with nscan < 0 the
/// reactance at `near` itself (for tests).
#[no_mangle]
pub unsafe extern "C" fn tmm_jet(
    c: f64, rho: f64, rh: f64, area: f64, a: f64,
    n: usize, kind: *const u8, len: *const f64, r: *const f64, open: *const u8, t: f64,
    stub: f64, neck_area: f64, neck_a: f64, leff: f64,
    near: f64, span: f64, nscan: i64, bis: i64,
) -> f64 {
    let m = Jet {
        c, rho, rh, area, a,
        kind: sl(kind, n), len: sl(len, n), r: sl(r, n), open: sl(open, n), t,
        stub, neck_area, neck_a, leff,
    };
    if nscan < 0 { m.y(near) } else { scan(near, span, nscan, bis, |x| m.y(x)) }
}
