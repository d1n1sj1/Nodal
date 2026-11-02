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

/// Inverting Schmitt trigger: Vin (node 1) on minus; out (2) fed back to plus (3) through a 10k/10k divider.
fn schmitt(vin: f64) -> Circuit {
    Circuit { parts: vec![
        part("VIN", VSource { pos: n(1), neg: n(0), volts: vin }),
        res("R1", 2, 3, 10e3), res("R2", 3, 0, 10e3), res("RL", 2, 0, 10e3),
        amp(2, 3, 1),
    ]}
}

#[test]
fn schmitt_trigger_settles_in_the_consistent_state() {
    // Thresholds are +-6 V (half of the 12 V limit). Inside them both rails are stable and the solver picks +12 V;
    // outside, only one state is self-consistent.
    let low = dc_op(&schmitt(1.0)).unwrap().node_voltages[2];
    let high = dc_op(&schmitt(8.0)).unwrap().node_voltages[2];
    assert!((dc_op(&schmitt(-8.0)).unwrap().node_voltages[2] - 12.0).abs() < 0.05);
    assert!((low - 12.0).abs() < 0.05, "vin = 1 V: out = {low}");
    assert!((high + 12.0).abs() < 0.05, "vin = 8 V: out = {high}");
}

#[test]
fn schmitt_trigger_at_zero_volts_picks_the_positive_rail() {
    let out = dc_op(&schmitt(0.0)).unwrap().node_voltages[2]; // 0 V in: out = 0 V is an unstable equilibrium too
    assert!((out - 12.0).abs() < 0.05, "out = {out}"); // a stable rail, not the unstable mid-point
}

#[test]
fn relaxation_oscillator_runs_at_the_textbook_period() {
    // Out -> R -> minus, C from minus to ground, equal divider to plus: T = 2 R C ln 3.
    let c = Circuit { parts: vec![
        res("R", 2, 1, 10e3), part("C", Capacitor { a: n(1), b: n(0), farads: 10e-9 }),
        res("RA", 2, 3, 10e3), res("RB", 3, 0, 10e3),
        amp(2, 3, 1),
    ]};
    let t = transient(&c, 2e-3, 1e-6).unwrap();
    let out: Vec<f64> = t.node_voltages.iter().map(|v| v[2]).collect();
    let rises: Vec<f64> = (1..out.len()).filter(|&i| out[i - 1] < 0.0 && out[i] >= 0.0).map(|i| t.times[i]).collect();
    assert!(rises.len() >= 5, "rising edges: {}", rises.len());
    let period = (rises[rises.len() - 1] - rises[1]) / (rises.len() - 2) as f64;
    let want = 2.0 * 10e3 * 10e-9 * 3f64.ln();
    assert!((period - want).abs() / want < 0.03, "period {period} vs {want}");
}
