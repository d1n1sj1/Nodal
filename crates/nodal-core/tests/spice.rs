use nodal_core::spice::{from_spice, to_spice};
use nodal_core::*;

fn n(i: u32) -> NodeId { NodeId(i) }
fn part(id: &str, kind: PartKind) -> Part { Part { id: id.into(), kind } }

#[test]
fn every_part_roundtrips_through_spice() {
    let c = Circuit { parts: vec![
        part("1", PartKind::Resistor { a: n(1), b: n(2), ohms: 4700.0, max_watts: 0.25 }),
        part("2", PartKind::Capacitor { a: n(2), b: n(0), farads: 1e-6 }),
        part("3", PartKind::Inductor { a: n(2), b: n(3), henries: 1e-3 }),
        part("4", PartKind::VSource { pos: n(1), neg: n(0), volts: 5.0 }),
        part("5", PartKind::ISource { from: n(3), to: n(0), amps: 0.01 }),
        part("6", PartKind::Diode { anode: n(3), cathode: n(0) }),
        part("7", PartKind::Led { anode: n(3), cathode: n(0), max_amps: 0.02 }),
        part("8", PartKind::Bjt { c: n(1), b: n(2), e: n(0), pnp: true, beta: 150.0, max_amps: 0.1 }),
        part("9", PartKind::Mosfet { d: n(1), g: n(2), s: n(0), pmos: true, vth: 1.5, kp: 0.02, max_amps: 0.5 }),
        part("10", PartKind::Opamp { out: n(3), plus: n(1), minus: n(2), gain: 1e5, vsat: 12.0, max_amps: 0.025 }),
    ] };
    let back = from_spice(&to_spice(&c)).unwrap();
    assert_eq!(back.parts.len(), 10);
    for (a, b) in c.parts.iter().zip(&back.parts) {
        assert_eq!(b.id, format!("{}{}", match a.kind {
            PartKind::Resistor { .. } => "R", PartKind::Capacitor { .. } => "C", PartKind::Inductor { .. } => "L",
            PartKind::VSource { .. } => "V", PartKind::ISource { .. } => "I", PartKind::Diode { .. } | PartKind::Led { .. } => "D",
            PartKind::Bjt { .. } => "Q", PartKind::Mosfet { .. } => "M", _ => "E",
        }, a.id));
        assert_eq!(a.kind, b.kind);
    }
}

#[test]
fn reads_hand_written_netlist_with_named_nodes() {
    let c = from_spice("divider
V1 in 0 10
R1 in out 1k
R2 out gnd 2.2K
.op
.end
").unwrap();
    assert_eq!(c.parts[0].kind, PartKind::VSource { pos: n(1), neg: n(0), volts: 10.0 });
    assert_eq!(c.parts[2].kind, PartKind::Resistor { a: n(2), b: n(0), ohms: 2200.0, max_watts: 0.25 });
}

#[test]
fn rejects_unsupported_and_malformed_lines_with_a_line_number() {
    let e = from_spice("t\nR1 1 0 1k\nX1 1 0 sub\n").unwrap_err().to_string();
    assert!(e.contains("line 3") && e.contains("unsupported"), "{e}");
    assert!(from_spice("t\nR1 1 0 abc\n").is_err());
    assert!(from_spice("t\nQ1 1 2 0 nomodel\n").is_err());
    assert!(from_spice("t\nE1 1 5 2 3 1e5\n").is_err());
    assert!(from_spice("t\nR1 1 0 1k\nR1 1 0 2k\n").is_err());
    assert!(from_spice("t\n* nothing\n").is_err());
}

#[test]
fn continuation_models_and_suffixes() {
    let c = from_spice("t\nV1 a 0 DC 5\nR1 a b 1meg\nC1 b 0 10uF\nQ1 b a 0 qn\n.model qn NPN\n+ (BF=50)\n").unwrap();
    assert_eq!(c.parts.len(), 4);
    assert_eq!(c.parts[1].kind, PartKind::Resistor { a: n(1), b: n(2), ohms: 1e6, max_watts: 0.25 });
    assert!(matches!(c.parts[2].kind, PartKind::Capacitor { farads, .. } if (farads - 1e-5).abs() < 1e-18));
    assert!(matches!(c.parts[3].kind, PartKind::Bjt { beta, pnp: false, .. } if beta == 50.0));
}
