use nodal_core::*;

fn sample() -> Circuit {
    let n = NodeId;
    Circuit { parts: vec![
        Part { id: "R1".into(), kind: PartKind::Resistor { a: n(1), b: n(2), ohms: 100.0, max_watts: 0.25 } },
        Part { id: "C1".into(), kind: PartKind::Capacitor { a: n(2), b: n(0), farads: 1e-6 } },
        Part { id: "L1".into(), kind: PartKind::Inductor { a: n(2), b: n(3), henries: 1e-3 } },
        Part { id: "V1".into(), kind: PartKind::VSource { pos: n(1), neg: n(0), volts: 5.0, wave: None } },
        Part { id: "I1".into(), kind: PartKind::ISource { from: n(3), to: n(0), amps: 0.01 } },
        Part { id: "D1".into(), kind: PartKind::Diode { anode: n(3), cathode: n(0) } },
        Part { id: "LED1".into(), kind: PartKind::Led { anode: n(3), cathode: n(0), max_amps: 0.02 } },
        Part { id: "S1".into(), kind: PartKind::Switch { a: n(1), b: n(3), closed: true } },
    ] }
}

#[test]
fn roundtrip_json_keeps_all_parts() {
    let c = sample();
    let back = Circuit::from_json(&c.to_json()).unwrap();
    assert_eq!(back, c);
    assert_eq!(back.node_count(), 4);
    assert_eq!(Circuit { parts: vec![] }.node_count(), 0);
}

#[test]
fn file_has_no_mode_field() {
    let j = sample().to_json();
    assert!(!j.contains("\"mode\""));
    assert!(j.contains("\"version\":1"));
}

#[test]
fn unknown_part_kind_returns_unknown_part_kind_error() {
    let j = r#"{"version":1,"parts":[{"id":"Q1","kind":"Transistor","a":1}]}"#;
    assert_eq!(Circuit::from_json(j), Err(SimError::UnknownPartKind("Transistor".into())));
    assert!(matches!(Circuit::from_json("{nope"), Err(SimError::BadFile(_))));
}

#[test]
fn duplicate_part_id_is_bad_file() {
    let j = r#"{"version":1,"parts":[
     {"id":"V1","kind":"VSource","pos":1,"neg":0,"volts":5.0},
     {"id":"V1","kind":"VSource","pos":2,"neg":0,"volts":3.0}]}"#;
    assert_eq!(Circuit::from_json(j), Err(SimError::BadFile("duplicate part id \"V1\"".into())));
}

#[test]
fn newer_file_version_is_rejected_and_missing_version_is_accepted() {
    let j = |v: &str| format!(r#"{{{v}"parts":[{{"id":"S1","kind":"Switch","a":1,"b":0,"closed":true}}]}}"#);
    assert!(Circuit::from_json(&j(r#""version":1,"#)).is_ok());
    assert!(Circuit::from_json(&j("")).is_ok());
    let e = Circuit::from_json(&j(r#""version":2,"#)).unwrap_err();
    assert!(matches!(&e, SimError::BadFile(m) if m.contains("version 2") && m.contains("newer")), "{e}");
}
