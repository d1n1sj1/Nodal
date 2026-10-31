use nodal_core::dc::dc_op;
use nodal_core::transient::{transient, transient_from};
use nodal_core::{Circuit, NodeId, Part, PartKind, PartKind::*, SimError};
use std::time::Instant;

fn n(i: u32) -> NodeId { NodeId(i) }
fn part(id: &str, kind: PartKind) -> Part { Part { id: id.into(), kind } }

fn series(volts: f64, reversed: bool) -> Circuit {
    let (anode, cathode) = if reversed { (n(0), n(2)) } else { (n(2), n(0)) };
    Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts }),
        part("R1", Resistor { a: n(1), b: n(2), ohms: 1000.0, max_watts: 1.0 }),
        part("D1", Diode { anode, cathode }),
    ]}
}

#[test]
fn diode_series_resistor_dc() {
    let r = dc_op(&series(5.0, false)).unwrap();
    let vd = r.node_voltages[2];
    assert!(vd > 0.55 && vd < 0.75, "vd = {vd}");
    let want = (5.0 - vd) / 1000.0;
    assert!((r.part_currents["D1"] - want).abs() < 1e-6, "{} vs {want}", r.part_currents["D1"]);
}

#[test]
fn reverse_diode_blocks() {
    let r = dc_op(&series(5.0, true)).unwrap();
    // D1 anode->cathode current; reversed, so it flows backwards but must be tiny.
    assert!(r.part_currents["D1"].abs() < 1e-9, "{}", r.part_currents["D1"]);
}

#[test]
fn led_drops_more_than_diode() {
    let mut c = series(5.0, false);
    c.parts[2].kind = Led { anode: n(2), cathode: n(0), max_amps: 0.02 };
    let r = dc_op(&c).unwrap();
    assert!(r.node_voltages[2] > 1.0, "{}", r.node_voltages[2]); // n = 2 roughly doubles the drop
}

#[test]
fn nonconvergence_returns_error() {
    // Pathological: a 1e9 A current source forced through a diode. At that current the companion
    // model's Norton term (i - g*v, each ~1e10 A) loses ~1e-6 A to rounding, so the 1e-9 A residual
    // test can never pass and Newton must stop at the 100-iteration cap instead of looping forever.
    let c = Circuit { parts: vec![
        part("I1", ISource { from: n(0), to: n(1), amps: 1e9 }),
        part("D1", Diode { anode: n(1), cathode: n(0) }),
    ]};
    let t = Instant::now();
    let r = dc_op(&c);
    assert!(t.elapsed().as_secs_f64() < 1.0);
    assert_eq!(r.unwrap_err(), SimError::NoConvergence { iterations: 100 });
}

#[test]
fn diode_blocks_reverse_step_in_transient() {
    // Source at node 1, diode 1->2, 1 kΩ load 2->0.
    let circuit = |volts| Circuit { parts: vec![
        part("V1", VSource { pos: n(1), neg: n(0), volts }),
        part("D1", Diode { anode: n(1), cathode: n(2) }),
        part("R1", Resistor { a: n(2), b: n(0), ohms: 1000.0, max_watts: 1.0 }),
    ]};
    let first = transient(&circuit(5.0), 1e-3, 1e-5).unwrap();
    let on = first.node_voltages.last().unwrap()[2];
    assert!(on > 4.0, "forward load voltage {on}");
    assert!(first.part_currents["D1"].last().unwrap() > &4e-3);

    let second = transient_from(&circuit(-5.0), &first.final_state(), 1e-3, 1e-5).unwrap();
    for v in &second.node_voltages[1..] {
        assert!(v[2] >= -0.01, "load went to {}", v[2]);
    }
}

#[test]
fn diode_reached_only_through_diode_is_not_floating() {
    // Node 2 connects to ground only via the diode; must not be FloatingNode.
    let c = Circuit { parts: vec![
        part("I1", ISource { from: n(0), to: n(1), amps: 1e-3 }),
        part("R1", Resistor { a: n(1), b: n(2), ohms: 100.0, max_watts: 1.0 }),
        part("D1", Diode { anode: n(2), cathode: n(0) }),
    ]};
    assert!(dc_op(&c).is_ok());
}
