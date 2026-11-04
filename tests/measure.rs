//! The rig's analysis, end to end on the model: a recording simulated through
//! a coloured loudspeaker with noise, calibrated against the reference cavity,
//! must give back the model's resonances.
use bore_tmm::measure::{self, Sweep};
use bore_tmm::*;

fn load(p: &str) -> Instrument {
    let s = std::fs::read_to_string(format!("{}/{}", env!("CARGO_MANIFEST_DIR"), p)).unwrap();
    serde_json::from_str(&s).unwrap()
}

/// A loudspeaker that is nothing like flat: a 150Hz high-pass and a 6dB bump at 900Hz.
fn speaker(f: f64) -> Cx {
    let hp = cx(0.0, f / 150.0) / cx(1.0, f / 150.0);
    let q = 2.0;
    let bump = cx(1.0 - (f / 900.0).powi(2), 2.0 * (f / 900.0) / q)
        / cx(1.0 - (f / 900.0).powi(2), (f / 900.0) / q);
    hp * bump
}

fn noisy(mut x: Vec<f64>, level: f64, seed: u64) -> Vec<f64> {
    let mut s = seed;
    let peak = x.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    for v in x.iter_mut() {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let u = (s >> 11) as f64 / (1u64 << 53) as f64 - 0.5;
        *v = *v / peak + level * u;
    }
    x
}

#[test]
fn the_rig_gives_back_the_models_resonances() {
    let air = Air::at(25.0);
    let flute = load("bores/cut/octagonal-flute.json");
    let cavity = load("bores/rig/reference-cavity.json");
    let open = flute.notes[1].open.clone();
    let sw = Sweep { rate: 16000, f1: 60.0, f2: 3000.0, secs: 3.0 };
    // the same rig, the same loudspeaker, two recordings; each normalised on
    // its own, as a recorder's gain would be
    let rec = noisy(measure::simulate(&sw, |f| flute.bore_z(&air, f, &open) * speaker(f), 0.5), 1e-3, 1);
    let r0 = noisy(measure::simulate(&sw, |f| cavity.bore_z(&air, f, &[]) * speaker(f), 0.5), 1e-3, 2);
    let grid = measure::log_grid(70.0, 2700.0, 96);
    let h = measure::transfer(&sw, &rec, &grid, 1.0 / 24.0);
    let h0 = measure::transfer(&sw, &r0, &grid, 1.0 / 24.0);
    let z: Vec<f64> = grid.iter().enumerate()
        .map(|(i, &f)| h[i] / h0[i] * cavity.bore_z(&air, f, &[]).abs()).collect();
    let got = measure::peaks(&grid, &z, 3.0);
    let model: Vec<f64> = grid.iter().map(|&f| flute.bore_z(&air, f, &open).abs()).collect();
    let want = measure::peaks(&grid, &model, 3.0);
    assert!(want.len() >= 6, "the model has {} peaks", want.len());
    for m in &want {
        let near = got.iter().map(|g| cents(*g, *m).abs()).fold(f64::INFINITY, f64::min);
        assert!(near < 5.0, "model peak {m:.1}Hz: nearest measured {near:.1} cents away; got {got:?}");
    }
}

#[test]
fn a_wav_reads_back() {
    let p = std::env::temp_dir().join("bore-tmm-wav-test.wav");
    let p = p.to_str().unwrap();
    let x: Vec<f64> = (0..1000).map(|i| (i as f64 * 0.05).sin() * 0.5).collect();
    measure::write_wav(p, &x, 44100).unwrap();
    let (y, rate) = measure::read_wav(p).unwrap();
    assert_eq!(rate, 44100);
    assert_eq!(y.len(), x.len());
    assert!(x.iter().zip(&y).all(|(a, b)| (a - b).abs() < 1e-4));
}
