use nodal_core::dc::dc_op;
use nodal_core::{Circuit, NodeId, Part, PartKind::*, SimError};

fn p(id: &str, kind: nodal_core::PartKind) -> Part {
    Part { id: id.into(), kind }
}
fn n(i: u32) -> NodeId {
    NodeId(i)
}
fn r(id: &str, a: u32, b: u32) -> Part {
    p(id, Resistor { a: n(a), b: n(b), ohms: 1000.0, max_watts: 0.25 })
}
fn v(id: &str, pos: u32, volts: f64) -> Part {
    p(id, VSource { pos: n(pos), neg: n(0), volts })
}

#[test]
fn voltage_divider() {
    let c = Circuit { parts: vec![v("V1", 1, 10.0), r("R1", 1, 2), r("R2", 2, 0)] };
    let d = dc_op(&c).unwrap();
    assert!((d.node_voltages[2] - 5.0).abs() < 1e-9);
    assert!((d.part_currents["R1"] - 0.005).abs() < 1e-9);
    assert!((d.part_currents["R2"] - 0.005).abs() < 1e-9);
}

#[test]
fn no_ground_is_error() {
    let c = Circuit { parts: vec![r("R1", 1, 2), r("R2", 1, 2)] };
    assert_eq!(dc_op(&c).unwrap_err(), SimError::NoGround);
}

#[test]
fn floating_node_is_error() {
    let c = Circuit { parts: vec![v("V1", 1, 5.0), r("R1", 1, 2)] };
    assert_eq!(dc_op(&c).unwrap_err(), SimError::FloatingNode(2));
}

#[test]
fn voltage_source_loop_is_singular() {
    let c = Circuit { parts: vec![v("V1", 1, 5.0), v("V2", 1, 3.0)] };
    assert!(matches!(dc_op(&c), Err(SimError::Singular(_))));
}

#[test]
fn empty_circuit_is_error() {
    assert_eq!(dc_op(&Circuit::default()).unwrap_err(), SimError::EmptyCircuit);
}

#[test]
fn zero_ohm_and_nan_are_errors() {
    let z = p("R0", Resistor { a: n(1), b: n(0), ohms: 0.0, max_watts: 1.0 });
    let c = Circuit { parts: vec![v("V1", 1, 5.0), z] };
    assert!(matches!(dc_op(&c), Err(SimError::InvalidInput(m)) if m.contains("R0")));
    let c = Circuit { parts: vec![v("V1", 1, f64::NAN), r("R1", 1, 0)] };
    assert!(dc_op(&c).is_err());
}

#[test]
fn ill_scaled_circuit_solves() {
    let rr = |id, a, b, ohms| p(id, Resistor { a: n(a), b: n(b), ohms, max_watts: 1.0 });
    let c = Circuit { parts: vec![v("V1", 1, 10.0), rr("A", 1, 2, 1e-3), rr("B", 2, 3, 1e9), rr("C", 3, 0, 1e12)] };
    let d = dc_op(&c).unwrap();
    let want = 10.0 * 1e12 / (1e-3 + 1e9 + 1e12);
    assert!((d.node_voltages[3] - want).abs() < 1e-6, "{}", d.node_voltages[3]);
}

#[test]
fn singular_message_names_cause() {
    let c = Circuit { parts: vec![v("V1", 1, 5.0), v("V2", 1, 3.0)] };
    assert!(matches!(dc_op(&c), Err(SimError::Singular(m)) if m.contains("voltage sources in a loop")));
}

#[test]
fn series_capacitor_node_is_floating() {
    let cap = p("C1", Capacitor { a: n(1), b: n(2), farads: 1e-6 });
    let c = Circuit { parts: vec![v("V1", 1, 5.0), cap, r("R1", 2, 3), r("R2", 3, 2)] };
    assert_eq!(dc_op(&c).unwrap_err(), SimError::FloatingNode(2));
}

#[test]
fn isolated_resistor_island_is_floating() {
    let c = Circuit { parts: vec![v("V1", 1, 5.0), r("R1", 1, 0), r("R2", 2, 3), r("R3", 3, 2)] };
    assert_eq!(dc_op(&c).unwrap_err(), SimError::FloatingNode(2));
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn closed_switch_is_a_short() {
    let s = p("S1", Switch { a: n(2), b: n(0), closed: true });
    let d = dc_op(&Circuit { parts: vec![v("V1", 1, 10.0), r("R1", 1, 2), s] }).unwrap();
    assert!(near(d.node_voltages[2], 0.0));
    assert!(near(d.part_currents["S1"], 0.01));
}

#[test]
fn open_switch_blocks_current() {
    let s = p("S1", Switch { a: n(2), b: n(0), closed: false });
    let d = dc_op(&Circuit { parts: vec![v("V1", 1, 10.0), r("R1", 1, 2), r("R2", 2, 0), s] }).unwrap();
    assert!(near(d.node_voltages[2], 5.0));
    assert!(d.part_currents["S1"].abs() < 1e-9);
}

#[test]
fn inductor_is_a_short_at_dc() {
    let l = p("L1", Inductor { a: n(1), b: n(2), henries: 1e-3 });
    let d = dc_op(&Circuit { parts: vec![v("V1", 1, 10.0), l, r("R1", 2, 0)] }).unwrap();
    assert!(near(d.node_voltages[2], 10.0));
    assert!(near(d.part_currents["L1"], 0.01));
}

#[test]
fn capacitor_is_open_at_dc() {
    let cap = p("C1", Capacitor { a: n(1), b: n(2), farads: 1e-6 });
    let d = dc_op(&Circuit { parts: vec![v("V1", 1, 10.0), cap, r("R1", 2, 0)] }).unwrap();
    assert!(near(d.node_voltages[2], 0.0));
    assert!(d.part_currents["C1"].abs() < 1e-9);
}

#[test]
fn current_source_drives_a_resistor() {
    let i = p("I1", ISource { from: n(0), to: n(1), amps: 0.01 });
    let d = dc_op(&Circuit { parts: vec![i, r("R1", 1, 0)] }).unwrap();
    assert!(near(d.node_voltages[1], 10.0)); // from -> to injects into node 1
    assert!(near(d.part_currents["I1"], 0.01));
}
