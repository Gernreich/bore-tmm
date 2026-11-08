//! Each designing model's own design, as bore-tmm JSON, with what that model
//! says every note plays (model_hz), so `bore-tmm check --compare` can show the
//! Rust engine and the models are the same physics.
//!
//!     target/release/export_models              # writes bores/models/*.json
//!     target/release/export_models --out=DIR    # writes them there instead
//!
//! Every model is Rust since 2026-11-07, in the sibling repositories, and
//! prints its own designs, which this runs, built: `naf_model --bores` in
//! square-flute, `flute_model --bores` in octagonal-flute, practice-chanters'
//! chanter_bores and `ghb_chanter --bores`. (export_models.py until
//! 2026-11-07; the move changed no byte of bores/models/.)
//!
//! These are the MODELS' bores, not the cut files'. A model's bore is what the
//! generator was asked for; a cut file's is what it drew. read_cut_files reads
//! the bore back out of a generator's own geometry, the check that catches
//! the two drifting apart.
#[path = "common.rs"]
mod common;
use common::*;

fn main() {
    let mut out = normpath(&here().join("bores").join("models"));
    for x in std::env::args().skip(1) {
        if let Some(v) = x.strip_prefix("--out=") {
            out = abspath(v);
        } else {
            eprintln!("error: {x} is not a flag this reads. It takes --out=DIR.");
            std::process::exit(1);
        }
    }
    // the plate chanter here is NOT the chanter practice_chanters builds:
    // chanter_model.rs's own design (practice-chanters/bores.rs says why it is kept)
    let runs: [(&str, &str, &[&str]); 4] = [
        ("octagonal-flute", "flute_model", &["--bores"]),
        ("square-flute", "naf_model", &["--bores"]),
        ("practice-chanters", "chanter_bores", &[]),
        ("ghb-chanter", "ghb_chanter", &["--bores"]),
    ];
    for (repo, exe, args) in runs {
        let j = rust(repo, exe, args).unwrap_or_else(|e| {
            eprintln!("error: {e}");
            std::process::exit(1)
        });
        for (name, d) in items(j.get("models").unwrap_or(&j)) {
            std::fs::create_dir_all(&out).unwrap();
            let path = out.join(format!("{name}.json"));
            write_json(&path, &d);
            println!("{}: {} notes", path.display(), d.get("notes").map_or(0, |n| n.arr().len()));
        }
    }
}
