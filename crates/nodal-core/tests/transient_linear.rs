use nodal_core::transient::transient;
use nodal_core::{Circuit, NodeId, Part, PartKind::*};

fn n(i: u32) -> NodeId { NodeId(i) }
fn part(id: &str, kind: nodal_core::PartKind) -> Part { Part { id: id.into(), kind } }
fn at(times: &[f64], t: f64) -> usize { times.iter().position(|&x| (x - t).abs() < 1e-12).unwrap() }

#[test]
fn rc_charging_matches_analytic() {
    let c = Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 5.0, wave: None }),
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
        part("V1", VSource { pos: n(1), neg: n(0), volts: 5.0, wave: None }),
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
        part("V1", VSource { pos: n(1), neg: n(0), volts: 5.0, wave: None }),
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
fn oversized_result_is_rejected_before_allocating() {
    // 1e7 steps is within the step cap, but 1e7 * (3 nodes + 3 parts) exceeds the stored-value cap.
    assert!(transient(&switch_circuit(true), 10.0, 1e-6).is_err());
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

#[test]
fn sine_source_drives_a_resistor_divider() {
    use nodal_core::Wave;
    let c = Circuit { parts: vec![
        Part { id: "V1".into(), kind: VSource { pos: NodeId(1), neg: NodeId(0), volts: 1.0, wave: Some(Wave::Sine { amp: 2.0, freq: 100.0 }) } },
        Part { id: "R1".into(), kind: Resistor { a: NodeId(1), b: NodeId(0), ohms: 1000.0, max_watts: 1.0 } },
    ] };
    let r = transient(&c, 0.01, 0.0001).unwrap();
    for (t, v) in r.times.iter().zip(&r.node_voltages) {
        if *t > 0.0 { assert!((v[1] - (1.0 + 2.0 * (std::f64::consts::TAU * 100.0 * t).sin())).abs() < 1e-9, "t={t} v={}", v[1]); }
    }
}

#[test]
fn square_source_flips_at_half_period_and_bad_freq_is_rejected() {
    use nodal_core::{SimError, Wave};
    let mk = |freq| Circuit { parts: vec![
        Part { id: "V1".into(), kind: VSource { pos: NodeId(1), neg: NodeId(0), volts: 0.0, wave: Some(Wave::Square { amp: 3.0, freq, duty: 0.5 }) } },
        Part { id: "R1".into(), kind: Resistor { a: NodeId(1), b: NodeId(0), ohms: 1000.0, max_watts: 1.0 } },
    ] };
    let r = transient(&mk(100.0), 0.01, 0.001).unwrap();
    assert_eq!(r.node_voltages[2][1].round(), 3.0); // t = 2 ms, first half
    assert_eq!(r.node_voltages[7][1].round(), -3.0); // t = 7 ms, second half
    assert!(matches!(transient(&mk(0.0), 0.01, 0.001), Err(SimError::InvalidInput(_))));
}

#[test]
fn square_duty_sets_the_high_fraction() {
    use nodal_core::{SimError, Wave};
    let mk = |duty| Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 0.0, wave: Some(Wave::Square { amp: 1.0, freq: 100.0, duty }) }),
        part("R1", Resistor { a: n(1), b: n(0), ohms: 1000.0, max_watts: 1.0 }),
    ] };
    let r = transient(&mk(0.2), 0.01, 0.001).unwrap();
    let high = r.node_voltages.iter().skip(1).take(10).filter(|v| v[1] > 0.0).count();
    assert_eq!(high, 2); // 10 steps in one period, 20% high
    for d in [0.0, 1.0] { assert!(matches!(transient(&mk(d), 0.01, 0.001), Err(SimError::InvalidInput(_)))); }
}
