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
