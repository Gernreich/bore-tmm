//! Python 3.9's numbers, bit for bit, for the ported design models.
//!
//! The five Python models that design the chanters and flutes (models.rs)
//! are ported here so that they give the SAME BITS, not merely close ones:
//! their answers are hole positions and widths in committed cut files, and a
//! port that moved one by 1e-12 mm would rewrite every one of them. That
//! needs Python's arithmetic exactly, and three things about it are not what
//! plain Rust does:
//!
//!   * mixed arithmetic promotes. In Python 3.9, float * complex turns the
//!     float into complex(x, 0.0) and does a full complex product, and int
//!     stays int against int. The models multiply matrices holding the ints
//!     1 and 0, so N carries all three kinds and follows Python's rules.
//!   * CPython's complex product and quotient are C code that Apple's clang
//!     compiled with fused multiply-adds. Measured against CPython on 200,000
//!     random pairs: the plain formulas disagree in 26% (product) and 15%
//!     (quotient) of cases, these fused forms in none.
//!   * cmath.cos and cmath.sin go through cosh and sinh, and clang merged
//!     their sin(y) and cos(y) into one __sincos_stret, which rounds
//!     differently from separate calls in about 1 case in 1,500. And x ** 2
//!     is libm's pow, not x * x, which differs in about 1 in 760.
//!
//! Every libm function is called through a function pointer behind
//! black_box, so the compiler cannot fold, merge or replace a call the way
//! clang did inside CPython. tools/check_fast.py tested that all of this held,
//! running every model both ways and comparing bits, until the models' Python
//! was retired on 2026-11-07; the generators' reproduced files hold it now.

use std::hint::black_box;
use std::ops::{Add, Div, Mul, Neg, Sub};

#[repr(C)]
struct SinCos {
    s: f64,
    c: f64,
}

extern "C" {
    fn __sincos_stret(x: f64) -> SinCos;
    fn tan(x: f64) -> f64;
    fn tanh(x: f64) -> f64;
    fn cosh(x: f64) -> f64;
    fn sinh(x: f64) -> f64;
    fn pow(x: f64, y: f64) -> f64;
}

type F1 = unsafe extern "C" fn(f64) -> f64;

fn libm(f: F1, x: f64) -> f64 {
    unsafe { black_box(f)(x) }
}

/// Python's x ** y for floats: libm's pow.
pub fn powf(x: f64, y: f64) -> f64 {
    let f: unsafe extern "C" fn(f64, f64) -> f64 = pow;
    unsafe { black_box(f)(x, black_box(y)) }
}

/// math.tan
pub fn mtan(x: f64) -> f64 {
    libm(tan, x)
}

/// math.tanh
pub fn mtanh(x: f64) -> f64 {
    libm(tanh, x)
}

/// math.sqrt: correctly rounded in hardware, as libm's is.
pub fn msqrt(x: f64) -> f64 {
    x.sqrt()
}

fn sincos(y: f64) -> (f64, f64) {
    let f: unsafe extern "C" fn(f64) -> SinCos = __sincos_stret;
    let r = unsafe { black_box(f)(y) };
    (r.s, r.c)
}

/// A Python number: int, float or complex.
#[derive(Clone, Copy, Debug)]
pub enum N {
    I(i64),
    F(f64),
    C(f64, f64),
}

pub use N::{C, F, I};

/// The literal 1j.
pub const J1: N = C(0.0, 1.0);

impl N {
    fn float(self) -> f64 {
        match self {
            I(i) => i as f64,
            F(x) => x,
            C(..) => unreachable!("a complex where Python wants a float"),
        }
    }
    fn complex(self) -> (f64, f64) {
        match self {
            I(i) => (i as f64, 0.0),
            F(x) => (x, 0.0),
            C(r, i) => (r, i),
        }
    }
    fn is_c(self) -> bool {
        matches!(self, C(..))
    }
    /// .imag
    pub fn imag(self) -> f64 {
        match self {
            C(_, i) => i,
            _ => 0.0,
        }
    }
    /// .real, or the float itself
    pub fn real(self) -> f64 {
        match self {
            C(r, _) => r,
            x => x.float(),
        }
    }
}

impl Add for N {
    type Output = N;
    fn add(self, o: N) -> N {
        match (self, o) {
            (I(a), I(b)) => I(a + b),
            (a, b) if a.is_c() || b.is_c() => {
                let (a, b) = (a.complex(), b.complex());
                C(a.0 + b.0, a.1 + b.1)
            }
            (a, b) => F(a.float() + b.float()),
        }
    }
}

impl Sub for N {
    type Output = N;
    fn sub(self, o: N) -> N {
        match (self, o) {
            (I(a), I(b)) => I(a - b),
            (a, b) if a.is_c() || b.is_c() => {
                let (a, b) = (a.complex(), b.complex());
                C(a.0 - b.0, a.1 - b.1)
            }
            (a, b) => F(a.float() - b.float()),
        }
    }
}

impl Mul for N {
    type Output = N;
    fn mul(self, o: N) -> N {
        match (self, o) {
            (I(a), I(b)) => I(a * b),
            (a, b) if a.is_c() || b.is_c() => {
                // CPython's _Py_c_prod, as clang fused it
                let (a, b) = (a.complex(), b.complex());
                C(a.0.mul_add(b.0, -(a.1 * b.1)), a.0.mul_add(b.1, a.1 * b.0))
            }
            (a, b) => F(a.float() * b.float()),
        }
    }
}

impl Div for N {
    type Output = N;
    fn div(self, o: N) -> N {
        if self.is_c() || o.is_c() {
            // CPython 3.9's _Py_c_quot (Smith's method), as clang fused it
            let (a, b) = (self.complex(), o.complex());
            let (abr, abi) = (b.0.abs(), b.1.abs());
            if abr >= abi {
                assert!(abr != 0.0, "complex division by zero");
                let ratio = b.1 / b.0;
                let denom = b.1.mul_add(ratio, b.0);
                C(a.1.mul_add(ratio, a.0) / denom, (-a.0).mul_add(ratio, a.1) / denom)
            } else if abi >= abr {
                let ratio = b.0 / b.1;
                let denom = b.0.mul_add(ratio, b.1);
                C(a.0.mul_add(ratio, a.1) / denom, a.1.mul_add(ratio, -a.0) / denom)
            } else {
                C(f64::NAN, f64::NAN)
            }
        } else {
            // int / int is true division; for the small ints here exact
            F(self.float() / o.float())
        }
    }
}

impl Neg for N {
    type Output = N;
    fn neg(self) -> N {
        match self {
            I(a) => I(-a),
            F(x) => F(-x),
            C(r, i) => C(-r, -i),
        }
    }
}

/// cmath.cos: cosh(iz), with sin and cos of the real part from one sincos.
pub fn ccos(z: N) -> N {
    let (re, im) = z.complex();
    let (x, y) = (-im, re);
    let (s, c) = sincos(y);
    C(c * libm(cosh, x), s * libm(sinh, x))
}

/// cmath.sin: -i sinh(iz).
pub fn csin(z: N) -> N {
    let (re, im) = z.complex();
    let (x, y) = (-im, re);
    let (s, c) = sincos(y);
    let sh = (c * libm(sinh, x), s * libm(cosh, x));
    C(sh.1, -sh.0)
}

/// cmath.tan: -i tanh(iz), CPython's tanh formula, fused.
pub fn ctan(z: N) -> N {
    let (re, im) = z.complex();
    let (x, y) = (-im, re);
    let tx = libm(tanh, x);
    let ty = libm(tan, y);
    let cx = 1.0 / libm(cosh, x);
    let txty = tx * ty;
    let denom = txty.mul_add(txty, 1.0);
    let th = (tx * ty.mul_add(ty, 1.0) / denom, ((ty / denom) * cx) * cx);
    C(th.1, -th.0)
}

/// A 2x2 matrix of Python numbers, as the models' nested tuples.
pub type Mx = [[N; 2]; 2];

pub const IDENT: Mx = [[I(1), I(0)], [I(0), I(1)]];

/// The models' mul(m, n), term for term.
pub fn mul(m: Mx, n: Mx) -> Mx {
    [
        [m[0][0] * n[0][0] + m[0][1] * n[1][0], m[0][0] * n[0][1] + m[0][1] * n[1][1]],
        [m[1][0] * n[0][0] + m[1][1] * n[1][0], m[1][0] * n[0][1] + m[1][1] * n[1][1]],
    ]
}
