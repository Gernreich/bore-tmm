//! Each instrument's bore as its CUT FILES draw it, as bore-tmm JSON.
//!
//!     target/release/read_cut_files            # writes bores/cut/ and bores/predict/
//!     target/release/read_cut_files --out=DIR  # DIR/cut/ and DIR/predict/ instead
//!
//! bores/cut/ holds the instruments whose drawings must play their model's
//! scale, and the tuning gate judges them. bores/predict/ holds the ones whose
//! pitch is the question rather than the check: the trumpets, and the uilleann
//! chanter, whose holes are cut small to be opened by ear.
//!
//! export_models writes what each designing model asked for. This reads what
//! the generator drew: hole positions and cut sizes measured off the SVGs in
//! each repository's cut-files/, the kerf taken from the file's own
//! description. `bore-tmm check --tol=...` over bores/cut/ then says whether
//! the parts as cut play the scale, which is what can drift when a generator
//! changes and its model does not.
//!
//! What a cut file cannot say is taken from the generator and named in each
//! reader: the bore's section (the parts are flat, and no one part carries it),
//! the ply, how far the foot is cut long to be trimmed, and the scale. Every
//! targets list is the model's, so a reader never decides what is in tune.
//!
//! Each reader fails loudly when the drawing is not the shape it expects: the
//! wrong number of holes, a window it cannot find. It never guesses.
//!
//! read_cut_files.py until 2026-11-07; the move changed no byte of bores/.
#[path = "common.rs"]
mod common;
#[path = "svgparts.rs"]
mod svgparts;
use common::*;
use lasermade_tools::py;
use lasermade_tools::re;
use lasermade_tools::turn3d::{Num, F, I, J};
use std::path::{Path, PathBuf};
use svgparts::{eq_d, kerf_of, shapes, Shape};

const BLACK: &str = "#000000";
const ORANGE: &str = "#ff8000";
const BLUE: &str = "#0000ff";

/// A reader's failure: Unreadable is reported and the next reader runs; Fatal
/// is what Python let escape as a traceback, and stops everything.
enum Err {
    Unreadable(String),
    Fatal(String),
}
use Err::*;
type R = Result<(), Err>;
fn fatal(e: String) -> Err {
    Fatal(e)
}

fn f(x: f64) -> J {
    J::N(F(x))
}
fn s(x: &str) -> J {
    J::S(x.to_string())
}
fn obj(kv: Vec<(&str, J)>) -> J {
    J::obj(kv)
}
/// m[k], which must be there
fn at<'a>(m: &'a J, k: &str) -> &'a J {
    m.get(k).unwrap_or_else(|| lasermade_tools::die(format!("KeyError: {}", py::str_repr(k))))
}
fn num(m: &J, k: &str) -> Num {
    at(m, k).num()
}
/// f'{x:g}' of an int or a float
fn ng(n: Num) -> String {
    match n {
        I(i) => i.to_string(),
        F(x) => py::g(x),
    }
}

struct Ctx {
    root: PathBuf,
}

impl Ctx {
    fn write(&self, name: &str, d: &J, kind: &str) {
        let out = self.root.join(kind);
        std::fs::create_dir_all(&out).unwrap();
        let path = out.join(format!("{name}.json"));
        write_json(&path, d);
        let n = |k: &str| d.get(k).map_or(0, |v| v.arr().len());
        println!("{}: {} holes, {} notes", rel(&path), n("holes"), n("notes"));
    }
}

fn rel(p: &Path) -> String {
    relpath(p, &git())
}

fn rust_u(repo: &str, exe: &str, args: &[&str]) -> Result<J, Err> {
    rust(repo, exe, args).map_err(Unreadable)
}

fn read(path: &Path) -> Result<(Vec<Shape>, String), Err> {
    shapes(&path.to_string_lossy()).map_err(fatal)
}

/// The black outlines that are parts, largest first (indices into shapes).
fn parts_of(sh: &[Shape]) -> Vec<usize> {
    let mut v: Vec<usize> = (0..sh.len()).filter(|&i| sh[i].colour.as_deref() == Some(BLACK) && sh[i].closed).collect();
    v.sort_by(|&a, &b| (-sh[a].area()).partial_cmp(&-sh[b].area()).unwrap());
    v
}

fn inside_part(part: usize, sh: &[Shape], colour: &str) -> Vec<usize> {
    (0..sh.len())
        .filter(|&i| {
            let c = sh[i].centre();
            sh[i].colour.as_deref() == Some(colour) && i != part && sh[part].inside(c.0, c.1)
        })
        .collect()
}

/// The part's long axis: 0 for x, 1 for y.
fn along(part: &Shape) -> usize {
    let (x0, y0, x1, y1) = part.bbox();
    if x1 - x0 >= y1 - y0 {
        0
    } else {
        1
    }
}

fn c(s: &Shape, ax: usize) -> f64 {
    let p = s.centre();
    if ax == 0 {
        p.0
    } else {
        p.1
    }
}

fn sort_key(v: &mut [usize], key: impl Fn(usize) -> f64) {
    v.sort_by(|&a, &b| key(a).partial_cmp(&key(b)).unwrap());
}

/// [x for the first n of a JSON list]
fn first_n(l: &J, n: usize) -> J {
    J::A(l.arr().iter().take(n).cloned().collect())
}

// --- the octagonal flute --------------------------------------------------------

fn octagonal_flute(cx: &Ctx) -> R {
    let all = rust_u("octagonal-flute", "flute_model", &["--bores"])?;
    let m = at(&all, "reader");
    let fp = git().join("octagonal-flute/cut-files/octagonal-flute-D4-sheet1-cut-files.svg");
    let (sh, desc) = read(&fp)?;
    let kerf = kerf_of(&desc).map_err(fatal)?;
    // the top wall carries the two engraved lines: the cork's face and the foot
    let blues: Vec<usize> = (0..sh.len()).filter(|&i| sh[i].colour.as_deref() == Some(BLUE)).collect();
    let top: Vec<usize> = parts_of(&sh)
        .into_iter()
        .filter(|&p| blues.iter().all(|&b| {
            let q = sh[b].centre();
            sh[p].inside(q.0, q.1)
        }))
        .collect();
    if blues.len() != 2 || top.len() != 1 {
        return Err(Unreadable(format!("{}: no single top wall carrying the two engraved lines", rel(&fp))));
    }
    let top = top[0];
    let ax = along(&sh[top]);
    let mut cuts = inside_part(top, &sh, ORANGE);
    // the embouchure is the cut nearer the cork's line; the rest are finger holes
    let bc: Vec<f64> = blues.iter().map(|&b| c(&sh[b], ax)).collect();
    let cork = bc[1..].iter().fold(bc[0], |a, &b| if b < a { b } else { a });
    let foot = bc[1..].iter().fold(bc[0], |a, &b| if b > a { b } else { a });
    sort_key(&mut cuts, |i| c(&sh[i], ax));
    let emb = cuts[0];
    let mut holes = cuts[1..].to_vec();
    let mh = at(m, "holes");
    if holes.len() != mh.arr().len() {
        return Err(Unreadable(format!("{}: {} finger holes on the top wall, not {}", rel(&fp), holes.len(), mh.arr().len())));
    }
    let ec = c(&sh[emb], ax);
    // foot-most first, as the model numbers them
    sort_key(&mut holes, |i| -c(&sh[i], ax));
    let d = obj(vec![
        ("name", s("octagonal flute in D, as cut")),
        (
            "source",
            s(&format!(
                "{}: embouchure, holes, cork and foot lines measured, kerf {}; section, wall, lip cover and mouth from flute_model.rs",
                rel(&fp),
                py::repr(kerf)
            )),
        ),
        ("temp_c", at(m, "temp_c").clone()),
        (
            "drive",
            obj(vec![
                ("kind", s("jet")),
                ("area_mm2", J::N(num(m, "cover") * F(sh[emb].cut_area(kerf)))),
                ("chimney_mm", at(m, "chimney_mm").clone()),
                ("mouth", at(m, "mouth").clone()),
                ("stub_mm", f(ec - cork)),
            ]),
        ),
        (
            "bore",
            J::A(vec![obj(vec![
                ("len_mm", f(foot - ec)),
                ("d_mm", at(m, "d_mm").clone()),
                ("loss_ratio", at(m, "loss_ratio").clone()),
            ])]),
        ),
        ("end", s("unflanged")),
        (
            "holes",
            J::A(
                holes
                    .iter()
                    .enumerate()
                    .map(|(i, &h)| {
                        obj(vec![
                            ("name", mh.arr()[i].clone()),
                            ("at_mm", f(c(&sh[h], ax) - ec)),
                            ("d_mm", f(eq_d(sh[h].cut_area(kerf)))),
                            ("chimney_mm", at(m, "wall_mm").clone()),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("notes", notes(at(m, "notes"), |n| first_n(mh, n), 1.4)),
    ]);
    cx.write("octagonal-flute", &d, "cut");
    Ok(())
}

/// [{'name': nm, 'hz': hz, 'open': open(n), 'span': span} for n, (nm, hz) in enumerate(list)]
fn notes(l: &J, open: impl Fn(usize) -> J, span: f64) -> J {
    J::A(
        l.arr()
            .iter()
            .enumerate()
            .map(|(n, p)| obj(vec![("name", p.arr()[0].clone()), ("hz", p.arr()[1].clone()), ("open", open(n)), ("span", f(span))]))
            .collect(),
    )
}

// --- the square-bore flutes -----------------------------------------------------

fn square_flutes(cx: &Ctx) -> R {
    let all = rust_u("square-flute", "naf_model", &["--bores"])?;
    let m = at(&all, "reader");
    let d = git().join("square-flute/cut-files");
    let mut names: Vec<String> = std::fs::read_dir(&d)
        .map_err(|e| fatal(e.to_string()))?
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for name in names {
        let files: Vec<String> = std::fs::read_dir(d.join(&name))
            .map_err(|e| fatal(format!("NotADirectoryError: {e}")))?
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|x| x.ends_with(".svg"))
            .collect();
        if files.len() != 1 {
            return Err(Unreadable(format!("square-flute/cut-files/{name}: {} sheets, expected 1", files.len())));
        }
        let fp = d.join(&name).join(&files[0]);
        let (sh, desc) = read(&fp)?;
        let kerf = kerf_of(&desc).map_err(fatal)?;
        let variant = name.strip_suffix("-flat").unwrap_or(&name);
        let v = at(at(m, "variants"), variant);
        let wall = at(v, "wall").clone();
        // the top plate: the part with the five finger holes and the window
        let circles: Vec<usize> = (0..sh.len()).filter(|&i| sh[i].is_circle() && sh[i].colour.as_deref() == Some(ORANGE)).collect();
        let tops: Vec<usize> = parts_of(&sh)
            .into_iter()
            .filter(|&p| {
                circles
                    .iter()
                    .filter(|&&ci| {
                        let q = sh[ci].centre();
                        sh[p].inside(q.0, q.1)
                    })
                    .count()
                    == 5
            })
            .collect();
        // a doubled wall carries the holes twice, in the top and the doubler:
        // the top is the one that also has the window
        let tops: Vec<usize> = tops.into_iter().filter(|&p| inside_part(p, &sh, ORANGE).iter().any(|&i| !sh[i].is_circle())).collect();
        if tops.len() != 1 {
            return Err(Unreadable(format!("{}: {} parts look like the top plate", rel(&fp), tops.len())));
        }
        let top = tops[0];
        let ax = along(&sh[top]);
        let inner = inside_part(top, &sh, ORANGE);
        let mut holes: Vec<usize> = inner.iter().copied().filter(|&i| sh[i].is_circle()).collect();
        let slots: Vec<usize> = inner.iter().copied().filter(|&i| !sh[i].is_circle()).collect();
        if slots.len() != 2 {
            return Err(Unreadable(format!("{}: {} slots in the top plate, not the exit hole and the window", rel(&fp), slots.len())));
        }
        let hx = holes.iter().fold(0.0, |a, &h| a + c(&sh[h], ax)) / holes.len() as f64;
        // the window is the slot nearer the finger holes; the foot is the far end
        let key = |i: usize| (c(&sh[i], ax) - hx).abs();
        let win = slots[1..].iter().fold(slots[0], |a, &b| if key(b) < key(a) { b } else { a });
        let wc = c(&sh[win], ax);
        let (lo, hi) = (sh[top].bb(ax), sh[top].bb(ax + 2));
        let sign: f64 = if hx > wc { 1.0 } else { -1.0 };
        let end = if sign > 0.0 { hi } else { lo };
        let drawn = sign * (end - wc);
        let (x0, y0, x1, y1) = sh[win].bbox();
        let span = [x1 - x0, y1 - y0];
        let (wl, ww) = (span[ax] + kerf, span[1 - ax] + kerf);
        sort_key(&mut holes, |h| -sign * c(&sh[h], ax));
        let hn: Vec<String> = (1..=5).map(|i| format!("h{i}")).collect();
        let spare = num(m, "foot_spare");
        let d = obj(vec![
            ("name", s(&format!("square-bore flute {name}, as cut, foot trimmed {}mm", ng(spare)))),
            (
                "source",
                s(&format!(
                    "{}: window, holes and plate length measured, kerf {}; section, wall and the foot's {}mm to trim from naf_model.rs",
                    rel(&fp),
                    py::repr(kerf),
                    ng(spare)
                )),
            ),
            ("temp_c", at(m, "temp_c").clone()),
            (
                "drive",
                obj(vec![
                    ("kind", s("jet")),
                    ("area_mm2", f(ww * wl)),
                    ("chimney_mm", at(m, "window_t").clone()),
                    ("mouth", at(m, "mouth").clone()),
                    ("stub_mm", f(wl / 2.0)),
                ]),
            ),
            (
                "bore",
                J::A(vec![obj(vec![
                    ("len_mm", J::N(F(drawn) - spare)),
                    ("d_mm", at(v, "d_mm").clone()),
                    ("loss_ratio", at(v, "loss_ratio").clone()),
                ])]),
            ),
            ("end", s("unflanged")),
            (
                "holes",
                J::A(
                    holes
                        .iter()
                        .enumerate()
                        .map(|(i, &h)| {
                            obj(vec![
                                ("name", s(&hn[i])),
                                ("at_mm", f(sign * (c(&sh[h], ax) - wc))),
                                ("d_mm", f(2.0 * sh[h].r() + kerf)),
                                ("chimney_mm", wall.clone()),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("notes", notes(at(m, "notes"), |n| J::A(hn.iter().take(n).map(|x| s(x)).collect()), 1.4)),
        ]);
        cx.write(&format!("square-flute-{name}"), &d, "cut");
    }
    Ok(())
}

// --- the chanters -------------------------------------------------------------------
//
// A chanter's cut file cannot give its bore: the plate chanter's 131 plates are
// packed on the sheet by size, not in stack order, and a strip's holes do not say
// which end is the reed's. So the chanters are read from the PARTS the generator
// draws its sheet from -- each part's place down the body, each hole's cut -- which
// the repository's .repro gate holds equal to the committed SVG. The generators are
// Rust since 2026-11-07 and read their own parts: practice-chanters'
// chanter_bores and `ghb_chanter --bores` print each chanter's bore as this
// writes it (bores.rs and bores() there say how); this runs them, built.

fn practice_chanters(cx: &Ctx) -> R {
    for (name, d) in items(at(&rust_u("practice-chanters", "chanter_bores", &[])?, "cut")) {
        cx.write(&name, &d, "cut");
    }
    Ok(())
}

fn ghb_chanters(cx: &Ctx) -> R {
    for (name, d) in items(at(&rust_u("ghb-chanter", "ghb_chanter", &["--bores"])?, "cut")) {
        cx.write(&name, &d, "cut");
    }
    Ok(())
}

// --- the trumpets -------------------------------------------------------------------
//
// A trumpet here is three parts: the mouthpiece and the bell, shared by every
// 10mm bore, each a stack of rings whose apertures the cut file gives in
// assembly order from ring 0 at the bore; and the bore between them, a 10mm
// square channel whose centreline length its generator reports. The model
// takes every ring as a short cylinder of its aperture's area, exactly the
// staircase that is cut, and the lips as a reed at the rim: the instrument
// sounds near the peaks of the impedance they see.
//
// NOT modelled: the bore's turns. A 90-degree lattice corner grows the
// section by up to 41% for a block, and the spiral's facets meet at 45 degrees;
// both are taken as straight tube of the centreline's length. Nor the bell's
// square port on the spiral. These bores are in bores/predict/, not bores/cut/,
// so the tuning gate does not judge them: what they play is the question,
// and one measured note is not yet a scale.

fn trumpet() -> PathBuf {
    git().join("trumpet-elbows-allowed")
}

/// Each ring's aperture, round-equivalent diameter, in the file's order.
fn apertures(svg: &Path) -> Result<(Vec<f64>, f64), Err> {
    let (sh, desc) = read(svg)?;
    let rx = re::rx(r"([\d.]+)mm of rise");
    let rise = match re::first(&rx, &desc) {
        Some(m) => py::float(re::g(&m, 1)).ok_or_else(|| fatal(format!("could not convert string to float: {}", py::str_repr(re::g(&m, 1)))))?,
        None => 3.0,
    };
    Ok((sh.iter().filter(|x| x.colour.as_deref() == Some(ORANGE)).map(|x| eq_d(x.area())).collect(), rise))
}

fn rings(aps: impl Iterator<Item = f64>, rise: f64) -> Vec<J> {
    aps.map(|d| obj(vec![("len_mm", f(rise)), ("d_mm", f(d))])).collect()
}

/// A trumpet tool run for what it prints or writes; a failure is fatal, as
/// Python's check=True was.
fn tool(exe: &Path, args: &[&str]) -> Result<String, Err> {
    let r = std::process::Command::new(exe).args(args).output().map_err(|e| fatal(format!("{}: {e}", exe.display())))?;
    if !r.status.success() {
        return Err(fatal(format!("{} {} exited {}: {}", exe.display(), args.join(" "), r.status, String::from_utf8_lossy(&r.stderr))));
    }
    Ok(String::from_utf8_lossy(&r.stdout).into_owned())
}

/// The mouthpiece's rings, lip first. The shipped sheet is the trumpet
/// layout; the legacy layout, the one on the three-turn trumpet, is drawn by
/// the same generator into a temporary file.
fn mouthpiece(layout: &str) -> Result<(Vec<J>, String), Err> {
    let (aps, rise, src) = if layout == "trumpet" {
        let f = trumpet().join("parts/mouthpiece/cut-files/mouthpiece-bore10-trumpet-parts-cut-files.svg");
        let (a, r) = apertures(&f)?;
        (a, r, rel(&f))
    } else {
        let t = lasermade_tools::TempDir::new("read-cut-files");
        let f = t.0.join("legacy.svg");
        tool(&trumpet().join("target/release/mouthpiece-round"), &["--layout=legacy", &f.to_string_lossy()])?;
        let (a, r) = apertures(&f)?;
        (a, r, "mouthpiece-round --layout=legacy".to_string())
    };
    Ok((rings(aps.into_iter().rev(), rise), src))
}

fn bell() -> Result<(Vec<J>, String), Err> {
    let f = trumpet().join("parts/bell/cut-files/bell-round10-153mm-17rings-x3-rim86-cut-files.svg");
    let (aps, rise) = apertures(&f)?;
    Ok((rings(aps.into_iter(), rise), rel(&f)))
}

fn centreline(walk: &str) -> Result<f64, Err> {
    let out = tool(Path::new("python3"), &[&trumpet().join("tools/bore_split.py").to_string_lossy(), walk, "--bore=10", "--straight=30", "--no-write"])?;
    let rx = re::rx(r"\(([\d.]+)mm of centreline\)");
    match re::first(&rx, &out) {
        Some(m) => Ok(py::float(re::g(&m, 1)).unwrap()),
        None => Err(Unreadable("bore_split.py did not report a centreline length".into())),
    }
}

fn square10(len: f64) -> J {
    obj(vec![
        ("len_mm", f(len)),
        ("d_mm", f(2.0 * (100.0 / std::f64::consts::PI).sqrt())),
        ("loss_ratio", f((std::f64::consts::PI / 4.0).sqrt())),
    ])
}

fn trumpets(cx: &Ctx) -> R {
    let (b, bsrc) = bell()?;
    let walks = std::fs::read_to_string(trumpet().join("tools/walks/coil-3t.txt")).map_err(|e| fatal(e.to_string()))?;
    let walk = walks.split_inclusive('\n').next().unwrap_or("").trim().to_string();
    let l = centreline(&walk)?;
    for (layout, tag, why) in [
        ("legacy", "built", "as built: the 24-ring legacy mouthpiece"),
        ("trumpet", "designed", "as designed: the shipped 30-ring mouthpiece"),
    ] {
        let (mp, msrc) = mouthpiece(layout)?;
        let mut bore = mp;
        bore.push(square10(l));
        bore.extend(b.iter().cloned());
        let d = obj(vec![
            ("name", s(&format!("three-turn trumpet, {why}"))),
            (
                "source",
                s(&format!("{msrc}; {}mm of 10mm square bore, bore_split.py on tools/walks/coil-3t.txt with --straight=30; {bsrc}", py::g(l))),
            ),
            ("drive", obj(vec![("kind", s("reed"))])),
            ("bore", J::A(bore)),
            ("end", s("flanged")),
            ("targets", s("the one note measured off the built instrument (three-turn/README.md)")),
            (
                "notes",
                J::A(if tag == "built" { vec![obj(vec![("name", s("F4 measured")), ("hz", f(349.2)), ("span", f(1.15))])] } else { vec![] }),
            ),
        ]);
        cx.write(&format!("three-turn-{tag}"), &d, "predict");
    }
    let (mp, msrc) = mouthpiece("trumpet")?;
    let mut bore = mp;
    bore.push(square10(1000.0));
    bore.extend(b);
    let d = obj(vec![
        ("name", s("swept-curve spiral, 1000mm, with the shipped mouthpiece and the bell")),
        (
            "source",
            s(&format!(
                "{msrc} (which mouthpiece it carries is not recorded: assumed the shipped one); 1000mm of 10mm square ribbon bore, swept-curve/README.md; {bsrc}"
            )),
        ),
        ("drive", obj(vec![("kind", s("reed"))])),
        ("bore", J::A(bore)),
        ("end", s("flanged")),
        ("notes", J::A(vec![])),
    ]);
    cx.write("swept-curve-spiral", &d, "predict");
    Ok(())
}

// --- the uilleann chanter --------------------------------------------------------------
//
// Fischer's square cone under a round throat, his eight holes where the lid and
// back strips cut them. Nothing in uilleann_chanter.rs is modelled, and his
// holes are DRILLED SMALL, to be opened toward his final sizes by ear, so the
// chanter as cut should play flat; the second file has his final sizes. Not
// modelled either: his four dishes, the bore enlargements under the lid, each
// of which raises the notes above it.
//
// The reed and its staple are the uncalibrated part, as on every chanter here:
// a closed length of the throat's bore above the top, fitted (bore-tmm fit) so
// that the chanter as cut sounds low D with every hole closed. A piper sets
// that by how far the reed goes in.
//
// Fingering: the "closed" uilleann fingering, each note opening its own hole
// and keeping every other one shut; D' is the thumb's hole on the back.

const UILLEANN_NOTES: [(&str, i32, &str); 9] = [
    ("D4", 0, ""),
    ("Eb4", 1, "Eb"),
    ("E4", 2, "E"),
    ("F#4", 4, "F#"),
    ("G4", 5, "G"),
    ("A4", 7, "A"),
    ("B4", 9, "B"),
    ("C#5", 11, "C#"),
    ("D5", 12, "D'"),
];

fn uilleann(cx: &Ctx) -> R {
    let u = rust_u("uilleann-chanter", "uilleann_chanter", &["--bores"])?;
    let (x0, sole, found) = (num(&u, "x0"), num(&u, "sole"), at(&u, "found").arr().to_vec());
    let rf = at(&u, "ref").arr().to_vec();
    if found.len() != rf.len() {
        return Err(Unreadable(format!("uilleann_chanter.rs: {} holes cut, not {}", found.len(), rf.len())));
    }
    let side: Vec<f64> = at(&u, "side").nums().iter().map(|&x| (x * I(2)).f() / std::f64::consts::PI.sqrt()).collect();
    let sq = (std::f64::consts::PI / 4.0).sqrt();
    let d4 = 440.0 * py::powf(2.0, -7.0 / 12.0);
    let mut reed = 0.0;
    for tag in ["as-cut", "final"] {
        let sizes: Vec<J> = if tag == "as-cut" {
            found.iter().map(|x| x.arr()[1].clone()).collect()
        } else {
            rf.iter().map(|h| J::N(h.arr()[3].num() * num(&u, "inch"))).collect()
        };
        let mut ins = obj(vec![
            ("name", s(&format!("uilleann chanter, Fischer's square bore, holes {}", tag.replace('-', " ")))),
            (
                "source",
                s(&format!(
                    "uilleann-chanter/uilleann_chanter.rs build(): hole places, {}, strip depths; the throat and the square cone from its constants; the reed allowance fitted to low D",
                    if tag == "as-cut" { "their drilled sizes" } else { "Fischer's final sizes" }
                )),
            ),
            ("drive", obj(vec![("kind", s("reed"))])),
            (
                "bore",
                J::A(vec![
                    obj(vec![("len_mm", f(30.0)), ("d_mm", at(&u, "throat").clone())]),
                    obj(vec![("len_mm", J::N(x0)), ("d_mm", at(&u, "throat").clone())]),
                    obj(vec![("len_mm", J::N(sole - x0)), ("d1_mm", f(side[0])), ("d2_mm", f(side[1])), ("loss_ratio", f(sq))]),
                ]),
            ),
            ("end", s("unflanged")),
            (
                "holes",
                J::A(
                    rf.iter()
                        .zip(&found)
                        .zip(&sizes)
                        .map(|((h, fz), d)| {
                            let fz = fz.arr();
                            obj(vec![
                                ("name", h.arr()[0].clone()),
                                ("at_mm", J::N(F(30.0) + fz[0].num())),
                                ("d_mm", d.clone()),
                                ("chimney_mm", fz[2].clone()),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "notes",
                J::A(
                    UILLEANN_NOTES
                        .iter()
                        .map(|&(nm, k, o)| {
                            obj(vec![
                                ("name", s(nm)),
                                ("hz", f(d4 * py::powf(2.0, k as f64 / 12.0))),
                                ("open", J::A(if o.is_empty() { vec![] } else { vec![s(o)] })),
                                ("span", f(1.3)),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]);
        if tag == "as-cut" {
            let t = lasermade_tools::TempDir::new("read-cut-files");
            let fp = t.0.join("u.json");
            let mut txt = String::new();
            ins.dump(&mut txt);
            std::fs::write(&fp, txt).unwrap();
            let fs = fp.to_string_lossy().into_owned();
            let r = std::process::Command::new(here().join("target/release/bore-tmm"))
                .args(["fit", &fs, "--set=bore.0.len_mm", "--range=0,250", &format!("--heard=D4={}", py::repr(d4)), &format!("--write={fs}")])
                .output()
                .map_err(|e| fatal(e.to_string()))?;
            if !r.status.success() {
                return Err(Unreadable(format!(
                    "bore-tmm fit for the uilleann reed: {}{}",
                    String::from_utf8_lossy(&r.stdout),
                    String::from_utf8_lossy(&r.stderr)
                )));
            }
            let back = J::parse(&std::fs::read_to_string(&fp).unwrap()).map_err(fatal)?;
            reed = py::round_dp(num(&at(&back, "bore").arr()[0], "len_mm").f(), 2);
        }
        if let J::O(kv) = &mut ins {
            for (k, v) in kv.iter_mut() {
                if k == "bore" {
                    if let J::A(b) = v {
                        b[0].set("len_mm", f(reed));
                    }
                }
            }
        }
        let src = format!("{} ({}mm, on the holes as cut)", at(&ins, "source").str(), py::repr(reed));
        ins.set("source", s(&src));
        cx.write(&format!("uilleann-chanter-{tag}"), &ins, "predict");
    }
    Ok(())
}

fn main() {
    let mut root = normpath(&here().join("bores"));
    for x in std::env::args().skip(1) {
        if let Some(v) = x.strip_prefix("--out=") {
            root = abspath(v);
        } else {
            eprintln!("error: {x} is not a flag this reads. It takes --out=DIR.");
            std::process::exit(1);
        }
    }
    let cx = Ctx { root };
    let readers: [fn(&Ctx) -> R; 6] = [octagonal_flute, square_flutes, practice_chanters, ghb_chanters, trumpets, uilleann];
    let mut bad = 0;
    for r in readers {
        match r(&cx) {
            Ok(()) => {}
            Err(Unreadable(e)) => {
                bad += 1;
                println!("UNREADABLE {e}");
            }
            Err(Fatal(e)) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
    }
    std::process::exit(if bad > 0 { 1 } else { 0 });
}

