use nodal_core::dc::dc_op;
use nodal_core::transient::transient;
use nodal_core::{check_damage, Circuit, DamageKind, NodeId, Part, PartKind, PartKind::*, SimError};

fn n(i: u32) -> NodeId { NodeId(i) }
fn part(id: &str, kind: PartKind) -> Part { Part { id: id.into(), kind } }

/// Common source: VDD 10 V -> 1k -> drain (node 2); gate (node 3) driven by `vg`; source at ground.
fn nmos(vg: f64, kp: f64, max_amps: f64) -> Circuit {
    Circuit { parts: vec![
        part("VDD", VSource { pos: n(1), neg: n(0), volts: 10.0, wave: None }),
        part("RD", Resistor { a: n(1), b: n(2), ohms: 1000.0, max_watts: 10.0 }),
        part("VG", VSource { pos: n(3), neg: n(0), volts: vg, wave: None }),
        part("M1", Mosfet { d: n(2), g: n(3), s: n(0), pmos: false, vth: 2.0, kp, max_amps }),
    ]}
}

#[test]
fn nmos_saturation_follows_square_law() {
    let r = dc_op(&nmos(4.0, 0.002, 1.0)).unwrap();
    let id = r.part_currents["M1"];
    assert!((id - 0.004).abs() < 1e-6, "id = {id}"); // kp/2 * (4-2)^2
    assert!((r.node_voltages[2] - 6.0).abs() < 1e-3);
}

#[test]
fn nmos_triode_and_cutoff() {
    let t = dc_op(&nmos(10.0, 0.01, 1.0)).unwrap();
    let vd = t.node_voltages[2];
    assert!((vd - 0.1245).abs() < 1e-3, "vd = {vd}");
    assert!((t.part_currents["M1"] - (10.0 - vd) / 1000.0).abs() < 1e-6);
    let off = dc_op(&nmos(1.0, 0.01, 1.0)).unwrap();
    assert!(off.part_currents["M1"].abs() < 1e-9 && (off.node_voltages[2] - 10.0).abs() < 1e-3);
}

#[test]
fn pmos_mirrors_nmos() {
    // Source at 10 V, gate at 6 V (Vsg = 4), drain through 1k to ground.
    let c = Circuit { parts: vec![
        part("VDD", VSource { pos: n(1), neg: n(0), volts: 10.0, wave: None }),
        part("VG", VSource { pos: n(3), neg: n(0), volts: 6.0, wave: None }),
        part("RD", Resistor { a: n(2), b: n(0), ohms: 1000.0, max_watts: 10.0 }),
        part("M1", Mosfet { d: n(2), g: n(3), s: n(1), pmos: true, vth: 2.0, kp: 0.002, max_amps: 1.0 }),
    ]};
    let r = dc_op(&c).unwrap();
    assert!((r.part_currents["M1"] + 0.004).abs() < 1e-6, "{}", r.part_currents["M1"]); // flows out of a PMOS drain
    assert!((r.node_voltages[2] - 4.0).abs() < 1e-3);
}

#[test]
fn mosfet_transient_damage_and_validation() {
    let c = nmos(4.0, 0.002, 2e-3); // 4 mA > 2 mA limit
    let t = transient(&c, 1e-3, 1e-4).unwrap();
    assert!((t.part_currents["M1"].last().unwrap() - 0.004).abs() < 1e-6);
    let w = check_damage(&c, &t);
    assert_eq!((w.len(), w[0].part_id.as_str(), w[0].kind), (1, "M1", DamageKind::OverCurrent));
    assert!(matches!(dc_op(&nmos(4.0, 0.0, 1.0)), Err(SimError::InvalidInput(_))));
}

#[test]
fn mosfet_json_roundtrip() {
    let c = nmos(4.0, 0.002, 0.5);
    assert_eq!(Circuit::from_json(&c.to_json()).unwrap(), c);
}

/// 1 ns of load capacitance at a 1 us step: trapezoidal alone overshoots both rails by 0.3 V.
#[test]
fn cmos_inverter_with_a_coarse_step_stays_inside_the_rails() {
    use nodal_core::Wave;
    let c = Circuit { parts: vec![
        part("VDD", VSource { pos: n(1), neg: n(0), volts: 5.0, wave: None }),
        part("VIN", VSource { pos: n(2), neg: n(0), volts: 2.5, wave: Some(Wave::Square { amp: 2.5, freq: 100e3, duty: 0.5, phase: 0.0 }) }),
        part("MP", Mosfet { d: n(3), g: n(2), s: n(1), pmos: true, vth: 1.0, kp: 2e-3, max_amps: 1.0 }),
        part("MN", Mosfet { d: n(3), g: n(2), s: n(0), pmos: false, vth: 1.0, kp: 2e-3, max_amps: 1.0 }),
        part("CL", Capacitor { a: n(3), b: n(0), farads: 1e-9 }),
    ]};
    let r = transient(&c, 30e-6, 1e-6).unwrap();
    assert!(r.node_voltages.iter().all(|v| v[3] > -0.05 && v[3] < 5.05));
}
