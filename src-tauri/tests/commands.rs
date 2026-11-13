use nodal_app::commands::*;
use nodal_core::{Circuit, DamageKind};

const BARE_LED: &str = r#"{"version":1,"parts":[
 {"id":"V1","kind":"VSource","pos":1,"neg":0,"volts":5.0},
 {"id":"D1","kind":"Led","anode":1,"cathode":0,"max_amps":0.02}]}"#;

fn tmp(name: &str) -> String {
    std::env::temp_dir().join(format!("nodal_cmd_{}_{name}", std::process::id())).to_string_lossy().into_owned()
}

#[test]
fn run_simulation_returns_warnings_for_bare_led() {
    let out = run_simulation(BARE_LED.into(), 1e-3, 1e-4).unwrap();
    assert_eq!(out.times.len(), out.node_voltages.len());
    let w = out.warnings.iter().find(|w| w.part_id == "D1").expect("LED warning");
    assert_eq!(w.kind, DamageKind::OverCurrent);
    assert!(w.peak.is_finite() && w.peak > w.limit);
    serde_json::to_string(&out).unwrap();
}

#[test]
fn run_simulation_bad_json_is_string_error() {
    assert!(run_simulation("nope".into(), 1.0, 0.1).is_err());
}

#[test]
fn load_circuit_unknown_kind_returns_friendly_message() {
    let p = tmp("unknown.json");
    std::fs::write(&p, r#"{"version":1,"parts":[{"id":"X1","kind":"Transistor","a":1,"b":0}]}"#).unwrap();
    let e = load_circuit(p.clone()).unwrap_err();
    std::fs::remove_file(&p).ok();
    assert!(e.contains("Transistor") && e.contains("newer version"), "{e}");
}

#[test]
fn save_then_load_roundtrips_and_save_rejects_garbage() {
    let p = tmp("rt.json");
    save_circuit(p.clone(), BARE_LED.into()).unwrap();
    assert_eq!(load_circuit(p.clone()).unwrap(), BARE_LED);
    assert!(!std::path::Path::new(&format!("{p}.tmp")).exists());
    std::fs::remove_file(&p).ok();
    let bad = tmp("bad.json");
    assert!(save_circuit(bad.clone(), "garbage".into()).is_err());
    assert!(!std::path::Path::new(&bad).exists());
}

#[test]
fn cir_extension_saves_and_loads_spice_text() {
    let p = tmp("rt.cir");
    save_circuit(p.clone(), BARE_LED.into()).unwrap();
    let text = std::fs::read_to_string(&p).unwrap();
    assert!(text.contains(".end") && !text.contains("\"kind\""));
    let back = Circuit::from_json(&load_circuit(p.clone()).unwrap()).unwrap();
    assert_eq!(back.parts.len(), Circuit::from_json(BARE_LED).unwrap().parts.len());
    std::fs::remove_file(&p).ok();
}

#[test]
fn non_finite_peak_is_clamped_to_f64_max() {
    // exercised through the public path: JSON of any output must never contain null peaks
    let out = run_simulation(BARE_LED.into(), 1e-3, 1e-4).unwrap();
    let v: serde_json::Value = serde_json::to_value(&out).unwrap();
    for w in v["warnings"].as_array().unwrap() {
        assert!(w["peak"].is_number());
    }
    let w = clamp_peak(nodal_core::DamageWarning {
        part_id: "R".into(), kind: DamageKind::OverPower, peak: f64::INFINITY, limit: 1.0,
    });
    assert_eq!(w.peak, f64::MAX);
    assert!(serde_json::to_value(&w).unwrap()["peak"].is_number());
}

#[test]
fn run_simulation_duplicate_ids_is_error_not_panic() {
    let j = r#"{"version":1,"parts":[
     {"id":"V1","kind":"VSource","pos":1,"neg":0,"volts":5.0},
     {"id":"V1","kind":"VSource","pos":2,"neg":0,"volts":3.0},
     {"id":"R1","kind":"Resistor","a":1,"b":2,"ohms":100.0,"max_watts":1.0}]}"#;
    let e = run_simulation_impl(j, 1e-3, 1e-4).unwrap_err();
    assert!(e.contains("duplicate part id"), "{e}");
}

fn stim(id: &str) -> nodal_core::ac::AcStimulus { nodal_core::ac::AcStimulus { id: id.into(), amp: 1.0, phase_deg: 0.0 } }

#[test]
fn run_ac_returns_rc_corner_and_rejects_bad_source() {
    let rc = r#"{"version":1,"parts":[
     {"id":"V1","kind":"VSource","pos":1,"neg":0,"volts":5.0},
     {"id":"R1","kind":"Resistor","a":1,"b":2,"ohms":1000.0,"max_watts":1.0},
     {"id":"C1","kind":"Capacitor","a":2,"b":0,"farads":1e-6}]}"#;
    let fc = 1.0 / (2.0 * std::f64::consts::PI * 1000.0 * 1e-6);
    let out = run_ac_impl(rc, &[stim("V1")], fc, 1e4, 10).unwrap();
    assert!((out.magnitude[0][2] - 0.5f64.sqrt()).abs() < 1e-6);
    assert_eq!(out.freqs.len(), out.phase_deg.len());
    serde_json::to_string(&out).unwrap();
    assert!(run_ac_impl(rc, &[stim("R1")], fc, 1e4, 10).is_err());
}

#[test]
fn load_circuit_malformed_files_error_without_panic() {
    let cases: &[(&str, &str)] = &[
        ("empty.cir", ""),
        ("bin.cir", "\u{0}\u{1}\u{ff}"),
        ("short.cir", "R1\nC1 1\nV1 1\nM1 1 2 3\nQ1 1\nE1 1 2 3\nD1 1\nL1\nI1 1 0 SIN(\nV2 1 0 PULSE("),
        ("model.cir", ".model\n.model X\nQ1 1 2 3 X\nM1 1 2 3 3 X W=\nR1 1 0 abc\nR2 1 0 1e999\nR3 1 0 -"),
        ("utf.cir", "R1 é ü 1k\n+ \nV1 1 0 SIN(1 2"),
        ("deep.json", "{\"version\":1,\"parts\":[{}]}"),
        ("nul.json", "null"),
        ("big.json", "{\"version\":99999999999999999999,\"parts\":[]}"),
    ];
    for (name, body) in cases {
        let p = tmp(name);
        std::fs::write(&p, body).unwrap();
        let r = std::panic::catch_unwind(|| load_circuit(p.clone()));
        assert!(r.is_ok(), "{name} panicked");
    }
}

#[test]
fn extra_top_level_fields_such_as_ac_setup_survive_save_and_load() {
    let json = r#"{"version":1,"parts":[{"id":"V1","kind":"VSource","pos":1,"neg":0,"volts":5}],"ac":{"V1":{"amp":"2","phase":"90"}}}"#;
    let p = tmp("ac.json");
    save_circuit(p.clone(), json.into()).unwrap();
    assert_eq!(load_circuit(p.clone()).unwrap(), json);
    std::fs::remove_file(&p).ok();
}
