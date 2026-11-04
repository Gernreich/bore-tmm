//! bore-tmm: what a described bore plays.
//!
//!     bore-tmm check BORE.json...            each note, and how far out it is
//!     bore-tmm check --tol=5 BORE.json...    fail any note more than 5 cents out
//!     bore-tmm check --compare BORE.json...  and against the designing model
//!     bore-tmm registers BORE.json           each fingering's resonances, to 4x
//!     bore-tmm curve BORE.json NOTE [--from=HZ] [--to=HZ] [--step=HZ]
//!                                            |Z| at the drive, as CSV
//!     bore-tmm --temp=20 ...                 another air temperature, C
//!
//! check ends with one verdict line, "TUNING OK" or "TUNING FAILING", and
//! with --compare a second, "MODEL MATCH" or "MODEL MISMATCH"; it exits 1 on
//! any failure, or when it was given nothing to check.

use bore_tmm::{cents, Air, Instrument, Note};
use std::process::ExitCode;

const COMPARE_TOL: f64 = 0.01; // cents: the same physics should agree to rounding

fn load(path: &str) -> Result<Instrument, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let ins: Instrument = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
    ins.check().map_err(|e| format!("{path}: {e}"))?;
    Ok(ins)
}

fn open_of(n: &Note) -> Vec<String> {
    n.open.clone()
}

fn usage() -> ExitCode {
    eprintln!(
        "usage: bore-tmm check [--tol=CENTS] [--compare] BORE.json...\n       \
         bore-tmm registers BORE.json\n       \
         bore-tmm curve BORE.json NOTE [--from=HZ] [--to=HZ] [--step=HZ]\n       \
         any of them with --temp=C"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut tol: Option<f64> = None;
    let mut compare = false;
    let mut temp: Option<f64> = None;
    let (mut from, mut to, mut step) = (None, None, 1.0);
    let mut rest = vec![];
    for a in &args {
        let num = |s: &str| s.split_once('=').and_then(|(_, v)| v.parse::<f64>().ok());
        if a.starts_with("--tol=") {
            tol = num(a);
        } else if a == "--compare" {
            compare = true;
        } else if a.starts_with("--temp=") {
            temp = num(a);
        } else if a.starts_with("--from=") {
            from = num(a);
        } else if a.starts_with("--to=") {
            to = num(a);
        } else if a.starts_with("--step=") {
            step = num(a).unwrap_or(1.0);
        } else if a.starts_with("--") {
            eprintln!("error: {a} is not a flag bore-tmm reads");
            return usage();
        } else {
            rest.push(a.clone());
        }
    }
    let Some(cmd) = rest.first().cloned() else { return usage() };
    let files = &rest[1..];
    match cmd.as_str() {
        "check" => check(files, tol, compare, temp),
        "registers" if files.len() == 1 => registers(&files[0], temp),
        "curve" if files.len() == 2 => curve(&files[0], &files[1], from, to, step, temp),
        _ => usage(),
    }
}

fn check(files: &[String], tol: Option<f64>, compare: bool, temp: Option<f64>) -> ExitCode {
    if files.is_empty() {
        eprintln!("error: nothing to check");
        return ExitCode::from(1);
    }
    let (mut notes, mut out_of_tune, mut unfound) = (0, 0, 0);
    let (mut compared, mut mismatched) = (0, 0);
    let mut worst_model: f64 = 0.0;
    let mut broken = 0;
    for path in files {
        let ins = match load(path) {
            Ok(i) => i,
            Err(e) => {
                println!("FAIL {e}");
                broken += 1;
                continue;
            }
        };
        let air = Air::at(temp.unwrap_or(ins.temp_c));
        println!(
            "{}  ({}, {:.1}mm of bore, air at {:.0}C)",
            ins.name,
            path,
            ins.length_mm(),
            temp.unwrap_or(ins.temp_c)
        );
        for n in &ins.notes {
            notes += 1;
            let near = n.near_hz.unwrap_or(n.hz);
            let got = ins.sounding(&air, &open_of(n), near, n.span.unwrap_or(1.3));
            let Some(f) = got else {
                println!("  FAIL {:8} no resonance near {:.1}Hz", n.name, near);
                unfound += 1;
                continue;
            };
            let c = cents(f, n.hz);
            let bad = tol.map_or(false, |t| c.abs() > t);
            if bad {
                out_of_tune += 1;
            }
            let mut line = format!(
                "  {} {:8} {:8.2}Hz plays {:8.2}Hz {:+7.1} cents",
                if bad { "FAIL" } else { "    " },
                n.name,
                n.hz,
                f,
                c
            );
            if compare {
                if let Some(m) = n.model_hz {
                    let d = cents(f, m);
                    compared += 1;
                    worst_model = worst_model.max(d.abs());
                    if d.abs() > COMPARE_TOL {
                        mismatched += 1;
                    }
                    line += &format!("   model {:8.2}Hz, {:+.4}c", m, d);
                }
            }
            println!("{line}");
        }
    }
    let mut fail = broken > 0 || unfound > 0 || notes == 0;
    match tol {
        Some(t) if out_of_tune + unfound + broken == 0 && notes > 0 => {
            println!("TUNING OK: {notes} notes in {} files, every one within {t} cents", files.len())
        }
        Some(t) => {
            fail = true;
            println!(
                "TUNING FAILING: {out_of_tune} notes more than {t} cents out, {unfound} not found, \
                 {broken} files unread, of {notes} notes in {} files",
                files.len()
            )
        }
        None if !fail => println!("{notes} notes in {} files (no --tol: not judged)", files.len()),
        None => println!("FAILING: {unfound} notes not found, {broken} files unread"),
    }
    if compare {
        if compared > 0 && mismatched == 0 {
            println!("MODEL MATCH: {compared} notes, worst {worst_model:.5} cents from the designing model");
        } else {
            fail = true;
            println!(
                "MODEL MISMATCH: {mismatched} of {compared} notes more than {COMPARE_TOL} cents from \
                 the designing model, worst {worst_model:.4}"
            );
        }
    }
    if fail { ExitCode::from(1) } else { ExitCode::SUCCESS }
}

fn registers(path: &str, temp: Option<f64>) -> ExitCode {
    let ins = match load(path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    let air = Air::at(temp.unwrap_or(ins.temp_c));
    println!("{}: each fingering's resonances up to 4x its note, cents from its note's harmonics", ins.name);
    for n in &ins.notes {
        let near = n.near_hz.unwrap_or(n.hz);
        let rs = ins.resonances(&air, &open_of(n), near * 0.7, near * 4.2);
        let parts: Vec<String> = rs
            .iter()
            .map(|&f| {
                let h = (f / n.hz).round().max(1.0);
                format!("{:.1} ({}x{:+.0}c)", f, h, cents(f, n.hz * h))
            })
            .collect();
        println!("  {:8} {}", n.name, parts.join("  "));
    }
    ExitCode::SUCCESS
}

fn curve(path: &str, note: &str, from: Option<f64>, to: Option<f64>, step: f64, temp: Option<f64>) -> ExitCode {
    let ins = match load(path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    let Some(n) = ins.notes.iter().find(|n| n.name == note) else {
        eprintln!("error: {path} has no note {note}");
        return ExitCode::from(1);
    };
    let air = Air::at(temp.unwrap_or(ins.temp_c));
    let lo = from.unwrap_or(50.0);
    let hi = to.unwrap_or(n.hz * 6.0);
    println!("hz,z_mag_pa_s_per_m3,z_phase_rad,drive_x");
    let mut f = lo;
    while f <= hi {
        let z = ins.bore_z(&air, f, &open_of(n));
        println!("{:.2},{:.6e},{:.5},{:.6e}", f, z.abs(), z.arg(), ins.drive_x(&air, f, &open_of(n)));
        f += step;
    }
    ExitCode::SUCCESS
}
