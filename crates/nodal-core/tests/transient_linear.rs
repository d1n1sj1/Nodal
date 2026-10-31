use nodal_core::transient::transient;
use nodal_core::{Circuit, NodeId, Part, PartKind::*};

fn n(i: u32) -> NodeId { NodeId(i) }
fn part(id: &str, kind: nodal_core::PartKind) -> Part { Part { id: id.into(), kind } }
fn at(times: &[f64], t: f64) -> usize { times.iter().position(|&x| (x - t).abs() < 1e-12).unwrap() }

#[test]
fn rc_charging_matches_analytic() {
    let c = Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 5.0 }),
        part("R1", Resistor { a: n(1), b: n(2), ohms: 1000.0, max_watts: 1.0 }),
        part("C1", Capacitor { a: n(2), b: n(0), farads: 1e-6 }),
    ]};
    let r = transient(&c, 5e-3, 1e-6).unwrap();
    assert_eq!(r.times.len(), 5001);
    let k = at(&r.times, 1e-3);
    let want = 5.0 * (1.0 - (-1.0f64).exp());
    assert!((r.node_voltages[k][2] - want).abs() / want < 0.005, "{}", r.node_voltages[k][2]);
    assert!((r.part_currents["R1"][k] - (5.0 - want) / 1000.0).abs() < 1e-5);
}

#[test]
fn rl_current_rise_matches_analytic() {
    let c = Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 5.0 }),
        part("R1", Resistor { a: n(1), b: n(2), ohms: 100.0, max_watts: 1.0 }),
        part("L1", Inductor { a: n(2), b: n(0), henries: 10e-3 }),
    ]};
    let r = transient(&c, 500e-6, 0.1e-6).unwrap();
    let k = at(&r.times, 100e-6);
    let want = (5.0 / 100.0) * (1.0 - (-1.0f64).exp());
    assert!((r.part_currents["L1"][k] - want).abs() / want < 0.005);
}

fn switch_circuit(closed: bool) -> Circuit {
    Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 5.0 }),
        part("S1", Switch { a: n(1), b: n(2), closed }),
        part("R1", Resistor { a: n(2), b: n(0), ohms: 1000.0, max_watts: 1.0 }),
    ]}
}

#[test]
fn open_switch_blocks_current() {
    let r = transient(&switch_circuit(false), 1e-3, 1e-4).unwrap();
    assert!(r.part_currents["R1"].iter().all(|i| i.abs() < 1e-9));
}

#[test]
fn closed_switch_conducts() {
    let r = transient(&switch_circuit(true), 1e-3, 1e-4).unwrap();
    assert!(r.part_currents["R1"].iter().all(|i| (i - 0.005).abs() < 1e-9));
}

#[test]
fn step_must_be_positive() {
    let c = switch_circuit(true);
    assert!(transient(&c, 1e-3, 0.0).is_err());
    assert!(transient(&c, 1e-3, -1.0).is_err());
    assert!(transient(&c, 0.0, 1e-4).is_err());
    assert!(transient(&c, f64::NAN, 1e-4).is_err());
    assert!(transient(&c, 1e6, 1e-6).is_err()); // > 10 million steps
}
