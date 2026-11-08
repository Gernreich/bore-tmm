//! The shapes in a LaserMadeMusic cut file: every path, as a polygon with its
//! stroke colour, and the few measures a bore reader needs of it.
//!
//! The generators draw only M, L, A and Z (and the odd H/V), in absolute
//! millimetre coordinates, with every path inside a <g stroke=...> of one
//! colour. That is all this reads; anything else is an error rather than a
//! guess. (svgparts.py until 2026-11-07.)
use lasermade_tools::py::{self, matan2 as atan2, mcos as cos, msin as sin};
use lasermade_tools::re;
use std::f64::consts::PI;

pub type P = (f64, f64);

/// A path's polygon, or a <circle> (its measures exact, its points a 72-gon
/// for inside()).
#[derive(Clone, Debug)]
pub struct Shape {
    pub colour: Option<String>,
    pub pts: Vec<P>,
    pub closed: bool,
    pub circle: Option<(f64, f64, f64)>,
}

fn fsum(v: impl IntoIterator<Item = f64>) -> f64 {
    v.into_iter().fold(0.0, |a, b| a + b)
}
fn fmin(v: impl IntoIterator<Item = f64>) -> f64 {
    let mut it = v.into_iter();
    let first = it.next().unwrap();
    it.fold(first, |a, b| if b < a { b } else { a })
}
fn fmax(v: impl IntoIterator<Item = f64>) -> f64 {
    let mut it = v.into_iter();
    let first = it.next().unwrap();
    it.fold(first, |a, b| if b > a { b } else { a })
}

impl Shape {
    pub fn is_circle(&self) -> bool {
        self.circle.is_some()
    }
    pub fn r(&self) -> f64 {
        self.circle.unwrap().2
    }
    pub fn area(&self) -> f64 {
        if let Some((_, _, r)) = self.circle {
            return PI * py::powf(r, 2.0);
        }
        let p = &self.pts;
        let n = p.len();
        fsum((0..n).map(|i| {
            let j = (i + n - 1) % n;
            p[i].0 * p[j].1 - p[j].0 * p[i].1
        }))
        .abs()
            / 2.0
    }
    pub fn perimeter(&self) -> f64 {
        if let Some((_, _, r)) = self.circle {
            return 2.0 * PI * r;
        }
        let p = &self.pts;
        let n = if self.closed { p.len() } else { p.len() - 1 };
        fsum((0..n).map(|i| py::dist(p[i], p[(i + 1) % p.len()])))
    }
    pub fn centre(&self) -> P {
        if let Some((x, y, _)) = self.circle {
            return (x, y);
        }
        let (x0, y0, x1, y1) = self.bbox();
        ((x0 + x1) / 2.0, (y0 + y1) / 2.0)
    }
    pub fn bbox(&self) -> (f64, f64, f64, f64) {
        let xs = self.pts.iter().map(|q| q.0);
        let ys = self.pts.iter().map(|q| q.1);
        (fmin(xs.clone()), fmin(ys.clone()), fmax(xs), fmax(ys))
    }
    /// bbox as a list, for [ax] and [ax + 2]
    pub fn bb(&self, k: usize) -> f64 {
        let b = self.bbox();
        [b.0, b.1, b.2, b.3][k]
    }
    /// The area once cut, for a HOLE drawn kerf/2 inside its line: the beam
    /// takes kerf/2 more all round.
    pub fn cut_area(&self, kerf: f64) -> f64 {
        let h = kerf / 2.0;
        self.area() + self.perimeter() * h + PI * h * h
    }
    pub fn inside(&self, x: f64, y: f64) -> bool {
        let p = &self.pts;
        let n = p.len();
        let mut c = false;
        for i in 0..n {
            let ((x1, y1), (x2, y2)) = (p[i], p[(i + n - 1) % n]);
            if (y1 > y) != (y2 > y) && x < x1 + (y - y1) * (x2 - x1) / (y2 - y1) {
                c = !c;
            }
        }
        c
    }
}

/// An SVG elliptical arc as points, the end included (SVG 1.1 F.6.5).
#[allow(clippy::too_many_arguments)]
fn arc(p0: P, rx: f64, ry: f64, rot: f64, large: f64, sweep: f64, p1: P) -> Vec<P> {
    let step = py::radians(2.0);
    if rx == 0.0 || ry == 0.0 {
        return vec![p1];
    }
    let (mut rx, mut ry) = (rx, ry);
    let phi = py::radians(rot);
    let (cp, sp) = (cos(phi), sin(phi));
    let (dx, dy) = ((p0.0 - p1.0) / 2.0, (p0.1 - p1.1) / 2.0);
    let (x1, y1) = (cp * dx + sp * dy, -sp * dx + cp * dy);
    let lam = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lam > 1.0 {
        (rx, ry) = (rx * lam.sqrt(), ry * lam.sqrt());
    }
    let num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let mut co = py::max(0.0, num / (rx * rx * y1 * y1 + ry * ry * x1 * x1)).sqrt();
    if large == sweep {
        co = -co;
    }
    let (cx1, cy1) = (co * rx * y1 / ry, -co * ry * x1 / rx);
    let cx_ = cp * cx1 - sp * cy1 + (p0.0 + p1.0) / 2.0;
    let cy_ = sp * cx1 + cp * cy1 + (p0.1 + p1.1) / 2.0;
    let ang = |ux: f64, uy: f64, vx: f64, vy: f64| atan2(ux * vy - uy * vx, ux * vx + uy * vy);
    let t1 = ang(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut dt = ang((x1 - cx1) / rx, (y1 - cy1) / ry, (-x1 - cx1) / rx, (-y1 - cy1) / ry);
    if sweep == 0.0 && dt > 0.0 {
        dt -= 2.0 * PI;
    } else if sweep != 0.0 && dt < 0.0 {
        dt += 2.0 * PI;
    }
    let n = 2usize.max((dt.abs() / step) as usize + 1);
    let mut out: Vec<P> = (1..=n)
        .map(|k| {
            let t = t1 + dt * k as f64 / n as f64;
            let (x, y) = (rx * cos(t), ry * sin(t));
            (cp * x - sp * y + cx_, sp * x + cp * y + cy_)
        })
        .collect();
    out[n - 1] = p1;
    out
}

fn parse_d(d: &str) -> Result<Vec<(Vec<P>, bool)>, String> {
    let rx = re::rx(r"[MLHVAZmlhvaz]|-?\d*\.?\d+(?:e-?\d+)?");
    let toks: Vec<String> = re::findall1(&rx, d, 0);
    let f = |s: &str| -> Result<f64, String> { s.parse::<f64>().map_err(|_| format!("could not convert string to float: '{s}'")) };
    let (mut out, mut cur): (Vec<(Vec<P>, bool)>, Option<Vec<P>>) = (vec![], None);
    let mut pos = (0.0, 0.0);
    let mut cmd: Option<char> = None;
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i].as_str();
        if "MLHVAZ".contains(t) && t.len() == 1 {
            cmd = t.chars().next();
            i += 1;
            if t == "Z" {
                if let Some(c) = cur.take() {
                    if !c.is_empty() {
                        out.push((c, true));
                    }
                }
            }
            continue;
        }
        if t.len() == 1 && "mlhvaz".contains(t) {
            return Err(format!("relative path command {t}: not read"));
        }
        match cmd {
            Some('M') => {
                if let Some(c) = cur.take() {
                    if !c.is_empty() {
                        out.push((c, false));
                    }
                }
                pos = (f(&toks[i])?, f(&toks[i + 1])?);
                cur = Some(vec![pos]);
                i += 2;
                cmd = Some('L');
            }
            Some('L') => {
                pos = (f(&toks[i])?, f(&toks[i + 1])?);
                cur.as_mut().ok_or("'NoneType' object has no attribute 'append'")?.push(pos);
                i += 2;
            }
            Some('A') => {
                let v: Vec<f64> = toks[i..i + 7].iter().map(|s| f(s)).collect::<Result<_, _>>()?;
                let pts = arc(pos, v[0], v[1], v[2], v[3], v[4], (v[5], v[6]));
                cur.as_mut().ok_or("'NoneType' object has no attribute 'extend'")?.extend(pts);
                pos = (v[5], v[6]);
                i += 7;
            }
            Some('H') => {
                pos = (f(t)?, pos.1);
                cur.as_mut().ok_or("'NoneType' object has no attribute 'append'")?.push(pos);
                i += 1;
            }
            Some('V') => {
                pos = (pos.0, f(t)?);
                cur.as_mut().ok_or("'NoneType' object has no attribute 'append'")?.push(pos);
                i += 1;
            }
            _ => return Err(format!("unexpected {t}")),
        }
    }
    if let Some(c) = cur {
        if !c.is_empty() {
            out.push((c, false));
        }
    }
    Ok(out)
}

/// Every shape in the file, with its colour, and the file's <desc>.
pub fn shapes(path: &str) -> Result<(Vec<Shape>, String), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let doc = roxmltree::Document::parse(&text).map_err(|e| format!("{path}: {e}"))?;
    let root = doc.root_element();
    let desc = root
        .children()
        .find(|c| c.is_element() && c.tag_name().name() == "desc")
        .map(|c| c.text().unwrap_or("").to_string())
        .unwrap_or_default();
    let mut found = Vec::new();
    fn walk(el: roxmltree::Node, colour: Option<String>, path: &str, found: &mut Vec<Shape>) -> Result<(), String> {
        let colour = el.attribute("stroke").map(String::from).or(colour);
        let tag = el.tag_name().name();
        match tag {
            "path" => {
                for (mut pts, closed) in parse_d(el.attribute("d").unwrap_or(""))? {
                    if closed && pts.len() > 1 && pts[0] == pts[pts.len() - 1] {
                        pts.pop();
                    }
                    found.push(Shape { colour: colour.clone(), pts, closed, circle: None });
                }
            }
            "circle" => {
                let g = |k: &str| -> Result<f64, String> {
                    let v = el.attribute(k).unwrap_or("");
                    v.trim().parse::<f64>().map_err(|_| format!("could not convert string to float: '{v}'"))
                };
                let (x, y, r) = (g("cx")?, g("cy")?, g("r")?);
                let pts = (0..72)
                    .map(|i| (x + r * cos(2.0 * PI * i as f64 / 72.0), y + r * sin(2.0 * PI * i as f64 / 72.0)))
                    .collect();
                found.push(Shape { colour: colour.clone(), pts, closed: true, circle: Some((x, y, r)) });
            }
            "ellipse" | "rect" | "polygon" | "polyline" | "line" => {
                return Err(format!("{path}: a <{tag}>, which this does not read"));
            }
            _ => {}
        }
        for c in el.children().filter(|c| c.is_element()) {
            walk(c, colour.clone(), path, found)?;
        }
        Ok(())
    }
    walk(root, None, path, &mut found)?;
    Ok((found, desc))
}

pub fn kerf_of(desc: &str) -> Result<f64, String> {
    let rx = re::rx(r"drawn for a ([\d.]+)mm kerf");
    match re::first(&rx, desc) {
        Some(m) => Ok(re::g(&m, 1).parse().unwrap()),
        None => Err("the file does not say what kerf it is drawn for".into()),
    }
}

pub fn eq_d(area: f64) -> f64 {
    (4.0 * area / PI).sqrt()
}
