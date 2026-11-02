use nodal_core::dc::dc_op;
use nodal_core::transient::transient;
use nodal_core::{check_damage, Circuit, DamageKind, NodeId, Part, PartKind, PartKind::*};

fn n(i: u32) -> NodeId { NodeId(i) }
fn part(id: &str, kind: PartKind) -> Part { Part { id: id.into(), kind } }

/// Common emitter: Vcc 10 V -> 1k -> collector (node 2); Vbb -> rb -> base (node 3); emitter at ground.
fn npn(vbb: f64, rb: f64, beta: f64, max_amps: f64) -> Circuit {
    Circuit { parts: vec![
        part("VCC", VSource { pos: n(1), neg: n(0), volts: 10.0 }),
        part("RC", Resistor { a: n(1), b: n(2), ohms: 1000.0, max_watts: 10.0 }),
        part("VBB", VSource { pos: n(4), neg: n(0), volts: vbb }),
        part("RB", Resistor { a: n(4), b: n(3), ohms: rb, max_watts: 10.0 }),
        part("Q1", Bjt { c: n(2), b: n(3), e: n(0), pnp: false, beta, max_amps }),
    ]}
}

#[test]
fn npn_active_region_gain() {
    let r = dc_op(&npn(5.0, 100e3, 100.0, 1.0)).unwrap();
    let (vb, vc) = (r.node_voltages[3], r.node_voltages[2]);
    assert!(vb > 0.55 && vb < 0.8, "vb = {vb}");
    let ib = (5.0 - vb) / 100e3;
    let ic = r.part_currents["Q1"];
    assert!((ic / ib - 100.0).abs() < 2.0, "beta = {}", ic / ib);
    assert!((vc - (10.0 - ic * 1000.0)).abs() < 1e-6, "vc = {vc}");
    assert!(vc > 1.0, "should stay active, vc = {vc}");
}

#[test]
fn npn_cutoff_and_saturation() {
    let off = dc_op(&npn(0.0, 100e3, 100.0, 1.0)).unwrap();
    assert!(off.part_currents["Q1"].abs() < 1e-9 && (off.node_voltages[2] - 10.0).abs() < 1e-3);
    let sat = dc_op(&npn(10.0, 10e3, 100.0, 1.0)).unwrap();
    assert!(sat.node_voltages[2] < 0.3, "vce(sat) = {}", sat.node_voltages[2]);
    assert!(sat.part_currents["Q1"] < 10.0e-3 && sat.part_currents["Q1"] > 8.0e-3);
}

#[test]
fn pnp_mirrors_npn() {
    // Emitter at +10 V, collector through 1k to ground, base pulled to (10 - 5) V through 100k.
    let c = Circuit { parts: vec![
        part("VCC", VSource { pos: n(1), neg: n(0), volts: 10.0 }),
        part("VBB", VSource { pos: n(4), neg: n(0), volts: 5.0 }),
        part("RB", Resistor { a: n(3), b: n(4), ohms: 100e3, max_watts: 10.0 }),
        part("RC", Resistor { a: n(2), b: n(0), ohms: 1000.0, max_watts: 10.0 }),
        part("Q1", Bjt { c: n(2), b: n(3), e: n(1), pnp: true, beta: 100.0, max_amps: 1.0 }),
    ]};
    let r = dc_op(&c).unwrap();
    let ic = r.part_currents["Q1"];
    // Collector current flows out of a PNP collector, so the into-collector value is negative.
    assert!(ic < -3e-3 && ic > -6e-3, "ic = {ic}");
    assert!((r.node_voltages[2] + ic * 1000.0).abs() < 1e-6);
    assert!(r.node_voltages[1] - r.node_voltages[3] > 0.55);
}

#[test]
fn bjt_transient_matches_dc_and_damage_warns() {
    let c = npn(5.0, 100e3, 100.0, 1e-3); // collector current ~4 mA > 1 mA limit
    let dc = dc_op(&c).unwrap();
    let t = transient(&c, 1e-3, 1e-4).unwrap();
    let last = *t.part_currents["Q1"].last().unwrap();
    assert!((last - dc.part_currents["Q1"]).abs() < 1e-6);
    let w = check_damage(&c, &t);
    assert_eq!(w.len(), 1);
    assert_eq!((w[0].part_id.as_str(), w[0].kind), ("Q1", DamageKind::OverCurrent));
}

#[test]
fn bjt_with_bad_beta_is_invalid_input() {
    assert!(matches!(dc_op(&npn(5.0, 100e3, 0.0, 1.0)), Err(nodal_core::SimError::InvalidInput(_))));
}

#[test]
fn bjt_json_roundtrip() {
    let c = npn(5.0, 100e3, 100.0, 0.1);
    assert_eq!(Circuit::from_json(&c.to_json()).unwrap(), c);
}
