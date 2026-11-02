use nodal_core::dc::dc_op;
use nodal_core::transient::transient;
use nodal_core::{check_damage, Circuit, DamageKind, NodeId, Part, PartKind, PartKind::*, SimError};

fn n(i: u32) -> NodeId { NodeId(i) }
fn part(id: &str, kind: PartKind) -> Part { Part { id: id.into(), kind } }
fn res(id: &str, a: u32, b: u32, ohms: f64) -> Part { part(id, Resistor { a: n(a), b: n(b), ohms, max_watts: 10.0 }) }
fn amp(out: u32, plus: u32, minus: u32) -> Part { part("U1", Opamp { out: n(out), plus: n(plus), minus: n(minus), gain: 1e5, vsat: 12.0, max_amps: 0.025 }) }

/// Inverting: Vin (node 1) -> 1k -> minus (2); 10k from out (3) to minus; plus at ground.
fn inverting(vin: f64) -> Circuit {
    Circuit { parts: vec![part("VIN", VSource { pos: n(1), neg: n(0), volts: vin }), res("R1", 1, 2, 1000.0), res("RF", 3, 2, 10e3), amp(3, 0, 2)] }
}

#[test]
fn inverting_amp_gain_is_minus_ten() {
    let r = dc_op(&inverting(1.0)).unwrap();
    assert!((r.node_voltages[3] + 10.0).abs() < 5e-3, "vout = {}", r.node_voltages[3]);
    assert!(r.node_voltages[2].abs() < 1e-3, "virtual ground = {}", r.node_voltages[2]);
}

#[test]
fn output_saturates_at_the_limit() {
    let r = dc_op(&inverting(2.0)).unwrap(); // would be -20 V
    assert!((r.node_voltages[3] + 12.0).abs() < 0.01, "vout = {}", r.node_voltages[3]);
}

#[test]
fn follower_and_comparator() {
    let f = Circuit { parts: vec![part("VIN", VSource { pos: n(1), neg: n(0), volts: 3.0 }), amp(2, 1, 2), res("RL", 2, 0, 1000.0)] };
    assert!((dc_op(&f).unwrap().node_voltages[2] - 3.0).abs() < 1e-3);
    // Open loop, plus above minus: output pins at the positive limit.
    let c = Circuit { parts: vec![part("VP", VSource { pos: n(1), neg: n(0), volts: 1.0 }), part("VM", VSource { pos: n(2), neg: n(0), volts: 0.5 }), amp(3, 1, 2), res("RL", 3, 0, 10e3)] };
    assert!((dc_op(&c).unwrap().node_voltages[3] - 12.0).abs() < 0.01);
}

#[test]
fn opamp_transient_damage_and_validation() {
    // Follower driving 100 ohm at 3 V = 30 mA > 25 mA (negative: current flows out of the output pin).
    let c = Circuit { parts: vec![part("VIN", VSource { pos: n(1), neg: n(0), volts: 3.0 }), amp(2, 1, 2), res("RL", 2, 0, 100.0)] };
    let t = transient(&c, 1e-3, 1e-4).unwrap();
    assert!((t.part_currents["U1"].last().unwrap() + 0.03).abs() < 1e-4);
    let w = check_damage(&c, &t);
    assert_eq!((w.len(), w[0].part_id.as_str(), w[0].kind), (1, "U1", DamageKind::OverCurrent));
    let mut bad = inverting(1.0);
    bad.parts[3].kind = Opamp { out: n(3), plus: n(0), minus: n(2), gain: 0.0, vsat: 12.0, max_amps: 0.025 };
    assert!(matches!(dc_op(&bad), Err(SimError::InvalidInput(_))));
}

#[test]
fn opamp_json_roundtrip() {
    let c = inverting(1.0);
    assert_eq!(Circuit::from_json(&c.to_json()).unwrap(), c);
}
