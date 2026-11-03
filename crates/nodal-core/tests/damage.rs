use nodal_core::transient::transient;
use nodal_core::{check_damage, Circuit, DamageKind, NodeId, Part, PartKind::*};

fn n(i: u32) -> NodeId { NodeId(i) }
fn part(id: &str, kind: nodal_core::PartKind) -> Part { Part { id: id.into(), kind } }
fn warns(c: &Circuit) -> Vec<nodal_core::DamageWarning> {
    check_damage(c, &transient(c, 1e-3, 1e-4).unwrap())
}
fn led() -> nodal_core::PartKind { Led { anode: n(2), cathode: n(0), max_amps: 0.02 } }

#[test]
fn led_without_resistor_warns() {
    let c = Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 5.0, wave: None }),
        part("D1", Led { anode: n(1), cathode: n(0), max_amps: 0.02 }),
    ]};
    let w = warns(&c);
    assert_eq!(w.len(), 1);
    assert_eq!((w[0].part_id.as_str(), w[0].kind), ("D1", DamageKind::OverCurrent));
    assert!(w[0].peak > w[0].limit && w[0].limit == 0.02);
}

#[test]
fn led_with_330_ohm_is_fine() {
    let c = Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 5.0, wave: None }),
        part("R1", Resistor { a: n(1), b: n(2), ohms: 330.0, max_watts: 0.25 }),
        part("D1", led()),
    ]};
    assert!(warns(&c).is_empty());
}

#[test]
fn resistor_over_rating_warns() {
    let c = Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 12.0, wave: None }),
        part("R1", Resistor { a: n(1), b: n(0), ohms: 100.0, max_watts: 0.25 }),
    ]};
    let w = warns(&c);
    assert_eq!(w.len(), 1);
    assert_eq!((w[0].part_id.as_str(), w[0].kind, w[0].limit), ("R1", DamageKind::OverPower, 0.25));
    assert!((w[0].peak - 1.44).abs() < 0.0144, "{}", w[0].peak);
}

#[test]
fn no_warnings_when_within_limits() {
    let c = Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 5.0, wave: None }),
        part("R1", Resistor { a: n(1), b: n(0), ohms: 1000.0, max_watts: 0.25 }),
    ]};
    assert_eq!(warns(&c), vec![]);
}
