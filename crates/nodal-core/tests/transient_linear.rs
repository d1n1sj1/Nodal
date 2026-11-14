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
    let e = transient(&switch_circuit(true), 10.0, 1e-6).unwrap_err();
    assert!(matches!(&e, nodal_core::SimError::InvalidInput(m) if m.contains("too large")), "{e:?}");
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
        Part { id: "V1".into(), kind: VSource { pos: NodeId(1), neg: NodeId(0), volts: 1.0, wave: Some(Wave::Sine { amp: 2.0, freq: 100.0, phase: 0.0 }) } },
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
        Part { id: "V1".into(), kind: VSource { pos: NodeId(1), neg: NodeId(0), volts: 0.0, wave: Some(Wave::Square { amp: 3.0, freq, duty: 0.5, phase: 0.0 }) } },
        Part { id: "R1".into(), kind: Resistor { a: NodeId(1), b: NodeId(0), ohms: 1000.0, max_watts: 1.0 } },
    ] };
    let r = transient(&mk(100.0), 0.01, 0.001).unwrap();
    assert_eq!(r.node_voltages[2][1].round(), 3.0); // t = 2 ms, first half
    assert_eq!(r.node_voltages[7][1].round(), -3.0); // t = 7 ms, second half
    assert!(matches!(transient(&mk(0.0), 0.01, 0.001), Err(SimError::InvalidInput(_))));
}

#[test]
fn triangle_and_sawtooth_follow_their_shape_and_reject_bad_freq() {
    use nodal_core::{SimError, Wave};
    let mk = |w| Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 1.0, wave: Some(w) }),
        part("R1", Resistor { a: n(1), b: n(0), ohms: 1000.0, max_watts: 1.0 }),
    ] };
    // 100 Hz, 1 ms steps: 10 steps per period
    let tri = transient(&mk(Wave::Triangle { amp: 2.0, freq: 100.0, phase: 0.0 }), 0.03, 0.001).unwrap();
    for (k, want) in [(5, 3.0), (10, -1.0), (22, 0.6)] { assert!((tri.node_voltages[k][1] - want).abs() < 1e-9, "tri k={k}"); }
    let saw = transient(&mk(Wave::Sawtooth { amp: 2.0, freq: 100.0, phase: 0.0 }), 0.01, 0.001).unwrap();
    for (k, want) in [(5, 1.0), (9, 2.6)] { assert!((saw.node_voltages[k][1] - want).abs() < 1e-9, "saw k={k}"); }
    assert!(matches!(transient(&mk(Wave::Triangle { amp: 1.0, freq: 0.0, phase: 0.0 }), 0.01, 0.001), Err(SimError::InvalidInput(_))));
    assert!(matches!(transient(&mk(Wave::Sawtooth { amp: f64::NAN, freq: 1.0, phase: 0.0 }), 0.01, 0.001), Err(SimError::InvalidInput(_))));
}

#[test]
fn square_duty_sets_the_high_fraction() {
    use nodal_core::{SimError, Wave};
    let mk = |duty| Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 0.0, wave: Some(Wave::Square { amp: 1.0, freq: 100.0, duty, phase: 0.0 }) }),
        part("R1", Resistor { a: n(1), b: n(0), ohms: 1000.0, max_watts: 1.0 }),
    ] };
    let r = transient(&mk(0.2), 0.01, 0.001).unwrap();
    let high = r.node_voltages.iter().skip(1).take(10).filter(|v| v[1] > 0.0).count();
    assert_eq!(high, 2); // 10 steps in one period, 20% high
    for d in [0.0, 1.0] { assert!(matches!(transient(&mk(d), 0.01, 0.001), Err(SimError::InvalidInput(_)))); }
}

#[test]
fn phase_advances_every_shape_and_must_be_finite() {
    use nodal_core::{SimError, Wave};
    let mk = |w| Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 0.0, wave: Some(w) }),
        part("R1", Resistor { a: n(1), b: n(0), ohms: 1000.0, max_watts: 1.0 }),
    ] };
    // 100 Hz, 1 ms steps: 10 steps per period, so 36 degrees is one step; k = 0 is the at-rest sample
    let sine = transient(&mk(Wave::Sine { amp: 1.0, freq: 100.0, phase: 90.0 }), 0.01, 0.001).unwrap();
    assert!((sine.node_voltages[1][1] - (std::f64::consts::TAU * 0.35).sin()).abs() < 1e-9); // 90 + 36 degrees
    let saw = transient(&mk(Wave::Sawtooth { amp: 2.0, freq: 100.0, phase: 180.0 }), 0.01, 0.001).unwrap();
    assert!((saw.node_voltages[2][1] - 0.8).abs() < 1e-9); // x = 0.2 + 0.5 = 0.7 -> 2 * (2 * 0.7 - 1)
    let sq = transient(&mk(Wave::Square { amp: 1.0, freq: 100.0, duty: 0.5, phase: -90.0 }), 0.01, 0.001).unwrap();
    assert_eq!(sq.node_voltages[2][1], -1.0); // x = 0.2 - 0.25 wraps to 0.95: low
    assert_eq!(sq.node_voltages[6][1], 1.0); // x = 0.6 - 0.25 = 0.35: high
    assert!(matches!(transient(&mk(Wave::Sine { amp: 1.0, freq: 100.0, phase: f64::NAN }), 0.01, 0.001), Err(SimError::InvalidInput(_))));
}

#[test]
fn parallel_capacitors_and_a_capacitor_across_a_source_start() {
    let r = |extra: Vec<Part>| {
        let mut parts = vec![
            part("V1", VSource { pos: n(1), neg: n(0), volts: 5.0, wave: None }),
            part("R1", Resistor { a: n(1), b: n(2), ohms: 1000.0, max_watts: 1.0 }),
            part("C1", Capacitor { a: n(2), b: n(0), farads: 1e-6 }),
        ];
        parts.extend(extra);
        transient(&Circuit { parts }, 5e-3, 1e-5).unwrap()
    };
    // second capacitor in parallel: RC with 2 uF, tau = 2 ms
    let p = r(vec![part("C2", Capacitor { a: n(2), b: n(0), farads: 1e-6 })]);
    let k = at(&p.times, 2e-3);
    assert!((p.node_voltages[k][2] - 5.0 * (1.0 - (-1.0f64).exp())).abs() < 0.05, "{}", p.node_voltages[k][2]);
    // capacitor straight across the battery: no error, node 1 stays at 5 V from the first sample
    let s = r(vec![part("C2", Capacitor { a: n(1), b: n(0), farads: 1e-6 })]);
    assert!(s.node_voltages.iter().all(|v| (v[1] - 5.0).abs() < 1e-9));
    assert_eq!(s.part_currents["C2"][0], 0.0);
}

#[test]
fn capacitive_divider_shares_a_step_by_charge() {
    let c = Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 6.0, wave: None }),
        part("C1", Capacitor { a: n(1), b: n(2), farads: 2e-6 }),
        part("C2", Capacitor { a: n(2), b: n(0), farads: 1e-6 }),
        part("R1", Resistor { a: n(2), b: n(0), ohms: 1e9, max_watts: 1.0 }),
    ]};
    let r = transient(&c, 1e-3, 1e-5).unwrap();
    // 2 uF over 1 uF: the middle node takes 2/3 of the step
    assert!((r.node_voltages[1][2] - 4.0).abs() < 1e-3, "{}", r.node_voltages[1][2]);
}

#[test]
fn series_inductors_carry_one_current() {
    let c = Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 5.0, wave: None }),
        part("L1", Inductor { a: n(1), b: n(2), henries: 10e-3 }),
        part("L2", Inductor { a: n(2), b: n(3), henries: 10e-3 }),
        part("R1", Resistor { a: n(3), b: n(0), ohms: 100.0, max_watts: 1.0 }),
    ]};
    let r = transient(&c, 500e-6, 0.1e-6).unwrap();
    assert!(r.part_currents["L1"][0].abs() < 1e-9);
    let k = at(&r.times, 200e-6); // tau = 20 mH / 100 ohm = 200 us
    let want = 0.05 * (1.0 - (-1.0f64).exp());
    assert!((r.part_currents["L2"][k] - want).abs() / want < 0.01, "{}", r.part_currents["L2"][k]);
    assert!((r.part_currents["L1"][k] - r.part_currents["L2"][k]).abs() < 1e-9);
}

#[test]
fn first_sample_sees_the_wave() {
    use nodal_core::Wave;
    let go = |w: Wave| transient(&Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts: 0.0, wave: Some(w) }),
        part("R1", Resistor { a: n(1), b: n(0), ohms: 100.0, max_watts: 1.0 }),
    ]}, 1e-3, 1e-4).unwrap().node_voltages[0][1];
    assert!((go(Wave::Sine { amp: 1.0, freq: 100.0, phase: 90.0 }) - 1.0).abs() < 1e-9);
    assert!((go(Wave::Square { amp: 1.0, freq: 100.0, duty: 0.5, phase: 0.0 }) - 1.0).abs() < 1e-9);
    assert!((go(Wave::Triangle { amp: 1.0, freq: 100.0, phase: 0.0 }) + 1.0).abs() < 1e-9);
}
