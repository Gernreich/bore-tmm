//! Sanity checks against textbook results, independent of the Python models.
use bore_tmm::*;

fn pipe(drive: &str, end: &str, l: f64, d: f64) -> Instrument {
    serde_json::from_str(&format!(
        r#"{{"name":"pipe","drive":{drive},"end":"{end}",
            "bore":[{{"len_mm":{l},"d_mm":{d}}}],
            "notes":[{{"name":"n","hz":100}}]}}"#
    ))
    .unwrap()
}

#[test]
fn reed_on_a_cylinder_is_a_quarter_wave() {
    // closed at the reed, open at the foot: c / 4(L + 0.6133a)
    let air = Air::at(25.0);
    let p = pipe(r#"{"kind":"reed"}"#, "unflanged", 400.0, 16.0);
    let f = p.sounding(&air, &[], 210.0, 1.3).unwrap();
    let want = air.c / (4.0 * (0.400 + 0.6133 * 0.008));
    assert!(cents(f, want).abs() < 5.0, "{f} against {want}");
}

#[test]
fn reed_on_a_whole_cone_is_a_half_wave() {
    // a cone closed at its apex sounds as an open pipe of its length: c / 2L,
    // here the cone from a 3mm throat to 20mm, the apex added by the reed
    let air = Air::at(25.0);
    let p: Instrument = serde_json::from_str(
        r#"{"name":"cone","drive":{"kind":"reed","apex":true},"end":"unflanged",
            "bore":[{"len_mm":600,"d1_mm":3,"d2_mm":20}],
            "notes":[{"name":"n","hz":245}]}"#,
    )
    .unwrap();
    let whole = 0.600 * 20.0 / 17.0; // the apex is 106mm above the throat
    let f = p.sounding(&air, &[], 245.0, 1.3).unwrap();
    let want = air.c / (2.0 * (whole + 0.6133 * 0.010));
    assert!(cents(f, want).abs() < 5.0, "{f} against {want}");
}

#[test]
fn a_cone_of_no_taper_is_a_cylinder() {
    let air = Air::at(25.0);
    let a = cylinder(&air, 440.0, 0.008, 0.008, 0.3).0;
    let b = cone(&air, 440.0, 0.008, 0.008, 0.008, 0.3).0;
    for i in 0..2 {
        for j in 0..2 {
            assert!((a[i][j] - b[i][j]).abs() <= 1e-12 * a[i][j].abs().max(1.0));
        }
    }
}
