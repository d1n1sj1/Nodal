use nodal_core::ac::ac_sweep;
use nodal_core::dc::dc_op;
use nodal_core::mna::VT;
use nodal_core::{Circuit, NodeId, Part, PartKind, PartKind::*, SimError};

fn n(i: u32) -> NodeId { NodeId(i) }
fn part(id: &str, kind: PartKind) -> Part { Part { id: id.into(), kind } }
fn res(id: &str, a: u32, b: u32, ohms: f64) -> Part { part(id, Resistor { a: n(a), b: n(b), ohms, max_watts: 10.0 }) }
fn vs(id: &str, pos: u32, volts: f64) -> Part { part(id, VSource { pos: n(pos), neg: n(0), volts, wave: None }) }

/// Index of the sweep point closest to `f`.
fn at(r: &nodal_core::ac::AcResult, f: f64) -> usize {
    (0..r.freqs.len()).min_by(|&a, &b| (r.freqs[a] / f).ln().abs().total_cmp(&(r.freqs[b] / f).ln().abs())).unwrap()
}

#[test]
fn rc_low_pass_has_minus_3_db_and_45_degrees_at_the_corner() {
    // 1k, 1 uF: fc = 1 / (2 pi R C) = 159.15 Hz
    let c = Circuit { parts: vec![vs("V1", 1, 5.0), res("R1", 1, 2, 1000.0), part("C1", Capacitor { a: n(2), b: n(0), farads: 1e-6 })] };
    let fc = 1.0 / (2.0 * std::f64::consts::PI * 1000.0 * 1e-6);
    let r = ac_sweep(&c, "V1", fc, 1e5, 10).unwrap();
    assert!((r.freqs[0] - fc).abs() < 1e-9 && (r.freqs.last().unwrap() - 1e5).abs() < 1e-6);
    assert!((r.magnitude[0][2] - 1.0 / 2f64.sqrt()).abs() < 1e-6, "|H| = {}", r.magnitude[0][2]);
    assert!((r.phase_deg[0][2] + 45.0).abs() < 1e-4, "phase = {}", r.phase_deg[0][2]);
    // One decade up: -20 dB per decade roll-off.
    let k = at(&r, 10.0 * fc);
    let want = 1.0 / (1.0 + (r.freqs[k] / fc).powi(2)).sqrt();
    assert!((r.magnitude[k][2] - want).abs() < 1e-6);
    assert!(r.magnitude[0][1] == 1.0 && r.magnitude[0][0] == 0.0); // source node follows the 1 V stimulus
}

#[test]
fn rl_high_pass_across_the_inductor() {
    // V -> R 100 -> node 2 -> L 10 mH -> ground. |H| = wL / sqrt(R^2 + (wL)^2), fc = R / (2 pi L) = 1591.5 Hz
    let c = Circuit { parts: vec![vs("V1", 1, 1.0), res("R1", 1, 2, 100.0), part("L1", Inductor { a: n(2), b: n(0), henries: 10e-3 })] };
    let fc = 100.0 / (2.0 * std::f64::consts::PI * 10e-3);
    let r = ac_sweep(&c, "V1", fc, 1e5, 10).unwrap();
    assert!((r.magnitude[0][2] - 1.0 / 2f64.sqrt()).abs() < 1e-6);
    assert!((r.phase_deg[0][2] - 45.0).abs() < 1e-4, "phase = {}", r.phase_deg[0][2]);
}

#[test]
fn inverting_opamp_gain_and_phase() {
    let c = Circuit { parts: vec![
        vs("VIN", 1, 1.0), res("R1", 1, 2, 1000.0), res("RF", 3, 2, 10e3),
        part("U1", Opamp { out: n(3), plus: n(0), minus: n(2), gain: 1e5, vsat: 12.0, max_amps: 0.025 }),
    ]};
    let r = ac_sweep(&c, "VIN", 10.0, 1e3, 5).unwrap();
    assert!((r.magnitude[0][3] - 10.0).abs() < 0.01, "gain = {}", r.magnitude[0][3]);
    assert!((r.phase_deg[0][3].abs() - 180.0).abs() < 0.01, "phase = {}", r.phase_deg[0][3]);
}

#[test]
fn common_emitter_gain_matches_the_hybrid_pi_formula() {
    // Vcc 10 V, Rc 1k, Vbb 5 V through 100k, beta 100: gain = -gm Rc rpi / (Rb + rpi) for the base source Vbb.
    let c = Circuit { parts: vec![
        vs("VCC", 1, 10.0), res("RC", 1, 2, 1000.0), vs("VBB", 4, 5.0), res("RB", 4, 3, 100e3),
        part("Q1", Bjt { c: n(2), b: n(3), e: n(0), pnp: false, beta: 100.0, max_amps: 1.0 }),
    ]};
    let ic = dc_op(&c).unwrap().part_currents["Q1"];
    let gm = ic / VT;
    let rpi = 100.0 / gm;
    let want = gm * 1000.0 * rpi / (100e3 + rpi);
    let r = ac_sweep(&c, "VBB", 100.0, 1e3, 3).unwrap();
    assert!((r.magnitude[0][2] - want).abs() / want < 0.02, "|gain| = {}, want {want}", r.magnitude[0][2]);
    assert!((r.phase_deg[0][2].abs() - 180.0).abs() < 0.5, "phase = {}", r.phase_deg[0][2]); // inverting
}

#[test]
fn common_source_gain_is_gm_times_rd() {
    // Vdd 10 V, 1k drain load, gate at 4 V, vth 2, kp 0.002: saturation, gm = kp * (4 - 2) = 4 mS.
    let c = Circuit { parts: vec![
        vs("VDD", 1, 10.0), res("RD", 1, 2, 1000.0), vs("VG", 3, 4.0),
        part("M1", Mosfet { d: n(2), g: n(3), s: n(0), pmos: false, vth: 2.0, kp: 0.002, max_amps: 1.0 }),
    ]};
    let r = ac_sweep(&c, "VG", 10.0, 1e3, 3).unwrap();
    assert!((r.magnitude[0][2] - 4.0).abs() < 1e-3, "gain = {}", r.magnitude[0][2]);
}

#[test]
fn diode_small_signal_resistance_divides_the_input() {
    // 5 V through 1k into a diode: rd = Vt / Id, so |v_diode / v_in| = rd / (R + rd).
    let c = Circuit { parts: vec![vs("V1", 1, 5.0), res("R1", 1, 2, 1000.0), part("D1", Diode { anode: n(2), cathode: n(0) })] };
    let id = dc_op(&c).unwrap().part_currents["D1"];
    let rd = VT / id;
    let r = ac_sweep(&c, "V1", 10.0, 1e3, 3).unwrap();
    assert!((r.magnitude[0][2] - rd / (1000.0 + rd)).abs() < 1e-4, "{} vs {}", r.magnitude[0][2], rd / (1000.0 + rd));
}

#[test]
fn bad_input_is_invalid_input() {
    let c = Circuit { parts: vec![vs("V1", 1, 5.0), res("R1", 1, 0, 1000.0)] };
    let bad = |r: Result<_, SimError>| matches!(r, Err(SimError::InvalidInput(_)));
    assert!(bad(ac_sweep(&c, "R1", 1.0, 1e3, 10).map(|_| ())));
    assert!(bad(ac_sweep(&c, "nope", 1.0, 1e3, 10).map(|_| ())));
    assert!(bad(ac_sweep(&c, "V1", 0.0, 1e3, 10).map(|_| ())));
    assert!(bad(ac_sweep(&c, "V1", 1e3, 10.0, 10).map(|_| ())));
    assert!(bad(ac_sweep(&c, "V1", 1.0, 1e3, 0).map(|_| ())));
    assert!(bad(ac_sweep(&c, "V1", 1.0, 1e12, 100_000).map(|_| ())));
}

/// 1 V source on node 1, then R (1 kOhm) from node k to k+1 and C (1 uF) from k+1 to ground.
fn ladder(nodes: u32) -> Circuit {
    let mut parts = vec![vs("V1", 1, 5.0)];
    for k in 1..nodes {
        parts.push(res(&format!("R{k}"), k, k + 1, 1000.0));
        parts.push(part(&format!("C{k}"), Capacitor { a: n(k + 1), b: n(0), farads: 1e-6 }));
    }
    Circuit { parts }
}

#[test]
fn sparse_and_dense_ac_agree() {
    let c = ladder(40); // 40 unknowns: 80 in the real block form
    nodal_core::mna::set_dense_max(usize::MAX);
    let dense = ac_sweep(&c, "V1", 1.0, 1e4, 5).unwrap();
    nodal_core::mna::set_dense_max(0);
    let sparse = ac_sweep(&c, "V1", 1.0, 1e4, 5).unwrap();
    nodal_core::mna::set_dense_max(50);
    for (d, s) in dense.magnitude.iter().flatten().zip(sparse.magnitude.iter().flatten()) { assert!((d - s).abs() < 1e-9, "{d} vs {s}"); }
    for (d, s) in dense.phase_deg.iter().flatten().zip(sparse.phase_deg.iter().flatten()) { assert!((d - s).abs() < 1e-3, "{d} vs {s}"); }
}

#[test]
fn thousand_node_ladder_ac_runs_past_the_old_300_unknown_cap() {
    let r = ac_sweep(&ladder(1000), "V1", 0.1, 1e3, 2).unwrap();
    let last = r.magnitude[0].len() - 1;
    assert!(r.magnitude[0][2] > 0.9, "first node at low frequency {}", r.magnitude[0][2]);
    assert!(r.magnitude.last().unwrap()[last] < 1e-6, "high frequencies must die out along the ladder");
}

#[test]
fn zero_or_negative_reactive_values_are_rejected() {
    for farads in [0.0, -1e-6] {
        let c = Circuit { parts: vec![vs("V1", 1, 1.0), res("R1", 1, 2, 1000.0), part("C1", Capacitor { a: n(2), b: n(0), farads })] };
        assert!(matches!(ac_sweep(&c, "V1", 10.0, 1e3, 3), Err(SimError::InvalidInput(_))));
    }
}
