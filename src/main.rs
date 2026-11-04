//! bore-tmm: what a described bore plays.
//!
//!     bore-tmm check BORE.json...            each note, and how far out it is
//!     bore-tmm check --tol=5 BORE.json...    fail any note more than 5 cents out
//!     bore-tmm check --compare BORE.json...  and against the designing model
//!     bore-tmm registers BORE.json           each fingering's resonances, to 4x
//!                                            (no notes: the bare bore's, to 1500Hz)
//!     bore-tmm curve BORE.json NOTE [--from=HZ] [--to=HZ] [--step=HZ]   (NOTE - : all closed)
//!                                            |Z| at the drive, as CSV
//!     bore-tmm fit BORE.json --set=PATH --range=LO,HI [--heard=NAME=HZ,...] [--write=OUT.json]
//!                                            the value at PATH (bore.0.len_mm, drive.mouth,
//!                                            holes.3.d_mm, ...) that best plays the heard
//!                                            notes, or the notes' own targets
//!     bore-tmm sweep OUT.wav [--from=HZ] [--to=HZ] [--secs=S] [--rate=R]
//!                                            the swept sine to play into the rig
//!     bore-tmm analyse REC.wav [--ref=REF.wav --ref-bore=REF.json] [--csv=OUT.csv]
//!                                            the resonances a rig recording shows
//!     bore-tmm compare BORE.json NOTE REC.wav [--ref=... --ref-bore=...]
//!                                            those against the model's, peak by peak
//!     bore-tmm simulate BORE.json NOTE OUT.wav   the recording the rig would make
//!                                            of the model: for testing the analysis
//!   sweep, analyse, compare and simulate share --from, --to (Hz), --secs, --rate
//!     bore-tmm --temp=20 ...                 another air temperature, C
//!
//! check ends with one verdict line, "TUNING OK" or "TUNING FAILING", and
//! with --compare a second, "MODEL MATCH" or "MODEL MISMATCH"; it exits 1 on
//! any failure, or when it was given nothing to check.

use bore_tmm::measure::{self, Sweep};
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
         bore-tmm fit BORE.json --set=PATH --range=LO,HI [--heard=NAME=HZ,...] [--write=OUT.json]\n       \
         bore-tmm sweep OUT.wav | analyse REC.wav | compare BORE.json NOTE REC.wav\n       \
         bore-tmm simulate BORE.json NOTE OUT.wav\n       \
           [--from=HZ --to=HZ --secs=S --rate=R --ref=REF.wav --ref-bore=REF.json --csv=OUT.csv]\n       \
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
    let (mut set, mut range, mut heard, mut write) = (None, None, None, None);
    let (mut secs, mut rate, mut refw, mut refb, mut csv) = (6.0, 48000u32, None, None, None);
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
        } else if let Some(v) = a.strip_prefix("--set=") {
            set = Some(v.to_string());
        } else if let Some(v) = a.strip_prefix("--range=") {
            range = v.split_once(',').and_then(|(l, h)| Some((l.parse::<f64>().ok()?, h.parse::<f64>().ok()?)));
            if range.is_none() {
                eprintln!("error: --range= takes LO,HI");
                return usage();
            }
        } else if let Some(v) = a.strip_prefix("--heard=") {
            heard = Some(v.to_string());
        } else if a.starts_with("--secs=") {
            secs = num(a).unwrap_or(6.0);
        } else if a.starts_with("--rate=") {
            rate = num(a).map(|r| r as u32).unwrap_or(48000);
        } else if let Some(v) = a.strip_prefix("--ref=") {
            refw = Some(v.to_string());
        } else if let Some(v) = a.strip_prefix("--ref-bore=") {
            refb = Some(v.to_string());
        } else if let Some(v) = a.strip_prefix("--csv=") {
            csv = Some(v.to_string());
        } else if let Some(v) = a.strip_prefix("--write=") {
            write = Some(v.to_string());
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
        "sweep" | "analyse" | "compare" | "simulate" => {
            let sw = Sweep { rate, f1: from.unwrap_or(40.0), f2: to.unwrap_or(4000.0), secs };
            match (cmd.as_str(), files.len()) {
                ("sweep", 1) => sweep(&files[0], &sw),
                ("analyse", 1) => analyse(None, &files[0], &sw, refw.as_deref(), refb.as_deref(), csv.as_deref(), temp),
                ("compare", 3) => analyse(Some((&files[0], &files[1])), &files[2], &sw, refw.as_deref(),
                                          refb.as_deref(), csv.as_deref(), temp),
                ("simulate", 3) => simulate(&files[0], &files[1], &files[2], &sw, temp),
                _ => usage(),
            }
        }
        "fit" if files.len() == 1 => match (set, range) {
            (Some(p), Some(r)) => fit(&files[0], &p, r, heard.as_deref(), write.as_deref(), temp),
            _ => {
                eprintln!("error: fit needs --set=PATH and --range=LO,HI");
                usage()
            }
        },
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
        if let Some(t) = &ins.targets {
            println!("  targets: {t}");
        }
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
            let own = tol.map(|t| n.tol_cents.unwrap_or(t));
            let bad = own.map_or(false, |t| c.abs() > t);
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
            if let (Some(t), Some(nt)) = (tol, n.tol_cents) {
                if nt != t {
                    line += &format!("   (allowed {nt}: {})", n.tol_why.as_deref().unwrap_or("no reason given"));
                }
            }
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
            println!("TUNING OK: {notes} notes in {} files, every one within {t} cents or its own stated tolerance", files.len())
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
    if ins.holes.is_empty() {
        let rs = ins.resonances(&air, &[], 40.0, 1500.0);
        println!("{}: no holes; the bore's resonances, 40 to 1500Hz", ins.name);
        for f in rs {
            println!("  {:7.1}Hz  {}", f, note_name(f));
        }
        return ExitCode::SUCCESS;
    }
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
    // "-" is the bore with every hole closed: a trumpet's only fingering
    let bare = bare_note();
    let n = if note == "-" {
        &bare
    } else {
        let Some(n) = ins.notes.iter().find(|n| n.name == note) else {
            eprintln!("error: {path} has no note {note} (\"-\" is every hole closed)");
            return ExitCode::from(1);
        };
        n
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

/// The nearest equal-tempered note, A4 = 440, and how far off.
fn note_name(f: f64) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let m = 69.0 + 12.0 * (f / 440.0).log2();
    let k = m.round();
    let i = (k as i64).rem_euclid(12) as usize;
    let oct = (k as i64).div_euclid(12) - 1;
    format!("{}{} {:+.0}c", NAMES[i], oct, (m - k) * 100.0)
}

/// The JSON value at a dotted path: object keys, and array indices as numbers.
fn at_path<'a>(v: &'a mut serde_json::Value, path: &str) -> Option<&'a mut serde_json::Value> {
    let mut cur = v;
    for k in path.split('.') {
        cur = match k.parse::<usize>() {
            Ok(i) => cur.get_mut(i)?,
            Err(_) => cur.get_mut(k)?,
        };
    }
    Some(cur)
}

/// The one value at PATH that best plays the heard notes: least squares in
/// cents, scanned across the range and then narrowed by golden section, so a
/// single note is matched exactly and several are fitted as well as one
/// number allows. What the fit leaves over is printed note by note: that
/// residue is what one parameter cannot explain.
fn fit(path: &str, set: &str, (lo, hi): (f64, f64), heard: Option<&str>, write: Option<&str>,
       temp: Option<f64>) -> ExitCode {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {path}: {e}");
            return ExitCode::from(1);
        }
    };
    let base: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {path}: {e}");
            return ExitCode::from(1);
        }
    };
    let mut probe = base.clone();
    let Some(was) = at_path(&mut probe, set).and_then(|v| v.as_f64()) else {
        eprintln!("error: {path} has no number at {set}");
        return ExitCode::from(1);
    };
    let ins0: Instrument = match serde_json::from_value(base.clone()) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("error: {path}: {e}");
            return ExitCode::from(1);
        }
    };
    // the notes to fit, and what each was heard at
    let mut goals: Vec<(Note, f64)> = vec![];
    match heard {
        Some(h) => {
            for item in h.split(',') {
                let Some((name, hz)) = item.rsplit_once('=') else {
                    eprintln!("error: --heard= takes NAME=HZ pairs, comma-separated");
                    return ExitCode::from(2);
                };
                let Some(n) = ins0.notes.iter().find(|n| n.name == name.trim()) else {
                    eprintln!("error: {path} has no note {name}");
                    return ExitCode::from(1);
                };
                let Ok(hz) = hz.trim().parse::<f64>() else {
                    eprintln!("error: {item}: not a frequency");
                    return ExitCode::from(2);
                };
                goals.push((n.clone(), hz));
            }
        }
        None => goals = ins0.notes.iter().map(|n| (n.clone(), n.hz)).collect(),
    }
    if goals.is_empty() {
        eprintln!("error: no notes to fit");
        return ExitCode::from(1);
    }
    let make = |x: f64| -> Option<Instrument> {
        let mut v = base.clone();
        *at_path(&mut v, set)? = serde_json::json!(x);
        let i: Instrument = serde_json::from_value(v).ok()?;
        i.check().ok()?;
        Some(i)
    };
    let plays = |ins: &Instrument, n: &Note, hz: f64| -> Option<f64> {
        let air = Air::at(temp.unwrap_or(ins.temp_c));
        ins.sounding(&air, &n.open, hz, n.span.unwrap_or(1.3).max(1.3))
    };
    let cost = |x: f64| -> f64 {
        let Some(ins) = make(x) else { return f64::INFINITY };
        goals.iter().map(|(n, hz)| match plays(&ins, n, *hz) {
            Some(f) => cents(f, *hz).powi(2),
            None => 1e9,
        }).sum()
    };
    let scan = 60;
    let xs: Vec<f64> = (0..=scan).map(|i| lo + (hi - lo) * i as f64 / scan as f64).collect();
    let cs: Vec<f64> = xs.iter().map(|&x| cost(x)).collect();
    let k = (0..=scan).min_by(|&a, &b| cs[a].partial_cmp(&cs[b]).unwrap()).unwrap();
    let (mut a, mut b) = (xs[k.saturating_sub(1)], xs[(k + 1).min(scan)]);
    let g = (5f64.sqrt() - 1.0) / 2.0;
    let (mut c, mut d) = (b - g * (b - a), a + g * (b - a));
    let (mut fc, mut fd) = (cost(c), cost(d));
    for _ in 0..80 {
        if fc < fd {
            b = d;
            d = c;
            fd = fc;
            c = b - g * (b - a);
            fc = cost(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + g * (b - a);
            fd = cost(d);
        }
    }
    let x = (a + b) / 2.0;
    let edge = (x - lo).abs() < (hi - lo) * 1e-3 || (hi - x).abs() < (hi - lo) * 1e-3;
    let ins = make(x).unwrap();
    println!("{}: {set} {was} -> {x:.3}{}", ins.name,
             if edge { "  (AT THE END OF --range: widen it)" } else { "" });
    let mut sq = 0.0;
    for (n, hz) in &goals {
        match plays(&ins, n, *hz) {
            Some(f) => {
                let c = cents(f, *hz);
                sq += c * c;
                println!("  {:8} heard {:8.2}Hz, model {:8.2}Hz {:+7.1} cents", n.name, hz, f, c);
            }
            None => println!("  {:8} heard {:8.2}Hz, no resonance near it", n.name, hz),
        }
    }
    println!("  rms {:.1} cents over {} notes", (sq / goals.len() as f64).sqrt(), goals.len());
    if let Some(out) = write {
        let mut v = base.clone();
        *at_path(&mut v, set).unwrap() = serde_json::json!(x);
        let s = serde_json::to_string_pretty(&v).unwrap() + "\n";
        if let Err(e) = std::fs::write(out, s) {
            eprintln!("error: {out}: {e}");
            return ExitCode::from(1);
        }
        println!("  written to {out}");
    }
    if edge { ExitCode::from(1) } else { ExitCode::SUCCESS }
}

fn sweep(out: &str, sw: &Sweep) -> ExitCode {
    let mut x = sw.samples();
    // a second of silence after, for the bore's ring-down
    x.extend(std::iter::repeat(0.0).take(sw.rate as usize));
    match measure::write_wav(out, &x, sw.rate) {
        Ok(()) => {
            println!("{out}: {:.0} to {:.0}Hz over {}s at {}Hz, then 1s of silence. Play it into \
                      the rig, record the microphone from the start of playback, and pass the same \
                      --from, --to, --secs and --rate to analyse.", sw.f1, sw.f2, sw.secs, sw.rate);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {out}: {e}");
            ExitCode::from(1)
        }
    }
}

fn note_of<'a>(ins: &'a Instrument, note: &str, bare: &'a Note) -> Option<&'a Note> {
    if note == "-" { Some(bare) } else { ins.notes.iter().find(|n| n.name == note) }
}

fn bare_note() -> Note {
    Note { name: "-".into(), hz: 400.0, open: vec![], span: None, model_hz: None, near_hz: None,
           tol_cents: None, tol_why: None }
}

/// What a rig recording shows: |Z| up to a constant, its peaks, and with a
/// bore file and a note, the model's peaks beside them.
fn analyse(model: Option<(&String, &String)>, rec: &str, sw: &Sweep, refw: Option<&str>,
           refb: Option<&str>, csv: Option<&str>, temp: Option<f64>) -> ExitCode {
    let fail = |e: String| {
        eprintln!("error: {e}");
        ExitCode::from(1)
    };
    let (r, rate) = match measure::read_wav(rec) {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    if rate != sw.rate {
        return fail(format!("{rec} is {rate}Hz; pass --rate={rate} and the sweep's other settings"));
    }
    let grid = measure::log_grid(sw.f1 * 1.1, sw.f2 / 1.1, 96);
    let mut h = measure::transfer(sw, &r, &grid, 1.0 / 24.0);
    let mut how = "uncalibrated: the loudspeaker and microphone are in it";
    if let Some(rf) = refw {
        let Some(rb) = refb else { return fail("--ref= needs --ref-bore=, the reference's bore file".into()) };
        let (r0, rate0) = match measure::read_wav(rf) {
            Ok(v) => v,
            Err(e) => return fail(e),
        };
        if rate0 != rate {
            return fail(format!("{rf} is {rate0}Hz, {rec} {rate}Hz"));
        }
        let rins = match load(rb) {
            Ok(i) => i,
            Err(e) => return fail(e),
        };
        let air = Air::at(temp.unwrap_or(rins.temp_c));
        let h0 = measure::transfer(sw, &r0, &grid, 1.0 / 24.0);
        for (i, f) in grid.iter().enumerate() {
            h[i] = h[i] / h0[i] * rins.bore_z(&air, *f, &[]).abs();
        }
        how = "calibrated against the reference";
    }
    if let Some(out) = csv {
        let mut s = String::from("hz,z_rel\n");
        for (f, v) in grid.iter().zip(&h) {
            s += &format!("{:.2},{:.6e}\n", f, v);
        }
        if let Err(e) = std::fs::write(out, s) {
            return fail(format!("{out}: {e}"));
        }
    }
    let got = measure::peaks(&grid, &h, 3.0);
    println!("{rec}: {} resonances between {:.0} and {:.0}Hz ({how})", got.len(), grid[0], grid[grid.len() - 1]);
    let Some((bore, note)) = model else {
        for f in &got {
            println!("  {:8.1}Hz", f);
        }
        return ExitCode::SUCCESS;
    };
    let ins = match load(bore) {
        Ok(i) => i,
        Err(e) => return fail(e),
    };
    let bare = bare_note();
    let Some(n) = note_of(&ins, note, &bare) else { return fail(format!("{bore} has no note {note}")) };
    let air = Air::at(temp.unwrap_or(ins.temp_c));
    let zm: Vec<f64> = grid.iter().map(|&f| ins.bore_z(&air, f, &n.open).abs()).collect();
    let want = measure::peaks(&grid, &zm, 3.0);
    println!("  model ({}, {}): {} resonances", ins.name, n.name, want.len());
    let mut sq = (0.0, 0);
    for &m in &want {
        let near = got.iter().copied().min_by(|a, b| cents(*a, m).abs().partial_cmp(&cents(*b, m).abs()).unwrap());
        match near {
            Some(g) if cents(g, m).abs() < 100.0 => {
                let c = cents(g, m);
                sq = (sq.0 + c * c, sq.1 + 1);
                println!("  model {:8.1}Hz  measured {:8.1}Hz  {:+6.1} cents", m, g, c);
            }
            _ => println!("  model {:8.1}Hz  measured: none within 100 cents", m),
        }
    }
    if sq.1 > 0 {
        println!("  rms {:.1} cents over {} matched resonances", (sq.0 / sq.1 as f64).sqrt(), sq.1);
    }
    ExitCode::SUCCESS
}

fn simulate(bore: &str, note: &str, out: &str, sw: &Sweep, temp: Option<f64>) -> ExitCode {
    let ins = match load(bore) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    let bare = bare_note();
    let Some(n) = note_of(&ins, note, &bare) else {
        eprintln!("error: {bore} has no note {note}");
        return ExitCode::from(1);
    };
    let air = Air::at(temp.unwrap_or(ins.temp_c));
    let x = measure::simulate(sw, |f| ins.bore_z(&air, f, &n.open), 1.0);
    let peak = x.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let x: Vec<f64> = x.iter().map(|v| v / peak * 0.8).collect();
    match measure::write_wav(out, &x, sw.rate) {
        Ok(()) => {
            println!("{out}: the rig's recording of the model of {} ({}), as if measured", ins.name, n.name);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {out}: {e}");
            ExitCode::from(1)
        }
    }
}
