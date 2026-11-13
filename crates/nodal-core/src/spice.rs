//! SPICE netlist text <-> Circuit. Covers R C L V I D Q M E and `.model`; other cards are ignored.
use crate::error::SimError;
use crate::model::{Circuit, NodeId, Part, PartKind, Wave};
use std::collections::HashMap;

fn bad(line: usize, msg: impl std::fmt::Display) -> SimError {
    SimError::BadFile(format!("line {line}: {msg}"))
}

/// A SPICE number: "4.7k", "1meg", "10uF", "1e-3". Trailing unit letters are ignored.
fn value(tok: &str) -> Option<f64> {
    let t = tok.to_ascii_lowercase();
    let end = (1..=t.len()).rev().find(|&i| t.is_char_boundary(i) && t[..i].parse::<f64>().is_ok())?;
    let mult = match &t[end..] {
        s if s.starts_with("meg") => 1e6,
        s if s.starts_with("mil") => 25.4e-6,
        s => match s.chars().next() {
            Some('f') => 1e-15, Some('p') => 1e-12, Some('n') => 1e-9, Some('u') => 1e-6,
            Some('m') => 1e-3, Some('k') => 1e3, Some('g') => 1e9, Some('t') => 1e12,
            _ => 1.0,
        },
    };
    Some(t[..end].parse::<f64>().ok()? * mult).filter(|v| v.is_finite())
}

pub fn to_spice(c: &Circuit) -> String {
    use PartKind::*;
    let mut out = String::from("* Nodal circuit\n");
    let mut models = String::new();
    // an id that already starts with its type letter keeps it (V1 stays V1); a clash falls back to prefixing
    let mut used = std::collections::HashSet::new();
    for p in &c.parts {
        let id = &p.id;
        let letter = match &p.kind {
            Resistor { .. } | Switch { .. } => 'R', Capacitor { .. } => 'C', Inductor { .. } => 'L', VSource { .. } => 'V', ISource { .. } => 'I',
            Diode { .. } | Led { .. } => 'D', Bjt { .. } => 'Q', Mosfet { .. } => 'M', Opamp { .. } => 'E',
        };
        let mut nm = if id.chars().next().is_some_and(|c| c.eq_ignore_ascii_case(&letter)) { id.clone() } else { format!("{letter}{id}") };
        if used.contains(&nm.to_ascii_lowercase()) { nm = format!("{letter}{id}") }
        used.insert(nm.to_ascii_lowercase());
        match &p.kind {
            Resistor { a, b, ohms, .. } => out += &format!("{nm} {} {} {ohms:e}\n", a.0, b.0),
            Capacitor { a, b, farads } => out += &format!("{nm} {} {} {farads:e}\n", a.0, b.0),
            Inductor { a, b, henries } => out += &format!("{nm} {} {} {henries:e}\n", a.0, b.0),
            VSource { pos, neg, volts, wave } => {
                // a phase advance is a PULSE delay of the rest of the period; SPICE holds the low level until then, so the first partial period differs from Nodal's
                let td = |phase: f64, freq: f64| match (phase / 360.0).rem_euclid(1.0) { f if f == 0.0 => 0.0, f => (1.0 - f) / freq };
                let w = match wave {
                    None => format!("DC {volts:e}"),
                    Some(Wave::Sine { amp, freq, phase }) if *phase == 0.0 => format!("SIN({volts:e} {amp:e} {freq:e})"),
                    Some(Wave::Sine { amp, freq, phase }) => format!("SIN({volts:e} {amp:e} {freq:e} 0 0 {phase:e})"),
                    Some(Wave::Square { amp, freq, duty, phase }) => format!("PULSE({:e} {:e} {:e} 0 0 {:e} {:e})", volts - amp, volts + amp, td(*phase, *freq), duty / freq, 1.0 / freq),
                    Some(Wave::Triangle { amp, freq, phase }) => format!("PULSE({:e} {:e} {:e} {:e} {:e} 0 {:e})", volts - amp, volts + amp, td(*phase, *freq), 0.5 / freq, 0.5 / freq, 1.0 / freq),
                    Some(Wave::Sawtooth { amp, freq, phase }) => format!("PULSE({:e} {:e} {:e} {:e} 0 0 {:e})", volts - amp, volts + amp, td(*phase, *freq), 1.0 / freq, 1.0 / freq),
                };
                out += &format!("{nm} {} {} {w}\n", pos.0, neg.0)
            }
            ISource { from, to, amps } => out += &format!("{nm} {} {} DC {amps:e}\n", from.0, to.0),
            Diode { anode, cathode } => out += &format!("{nm} {} {} DIODE\n", anode.0, cathode.0),
            Led { anode, cathode, .. } => out += &format!("{nm} {} {} LED\n", anode.0, cathode.0),
            // SPICE switches are voltage controlled, so a fixed switch becomes a resistor
            Switch { a, b, closed } => out += &format!("{nm} {} {} {}\n", a.0, b.0, if *closed { "1m" } else { "1G" }),
            Bjt { c, b, e, pnp, beta, .. } => {
                out += &format!("{nm} {} {} {} M{id}\n", c.0, b.0, e.0);
                models += &format!(".model M{id} {} (BF={beta:e})\n", if *pnp { "PNP" } else { "NPN" });
            }
            Mosfet { d, g, s, pmos, vth, kp, .. } => {
                out += &format!("{nm} {} {} {} {} M{id}\n", d.0, g.0, s.0, s.0);
                let sign = if *pmos { -1.0 } else { 1.0 };
                models += &format!(".model M{id} {} (VTO={:e} KP={kp:e})\n", if *pmos { "PMOS" } else { "NMOS" }, sign * vth);
            }
            Opamp { out: o, plus, minus, gain, .. } => out += &format!("{nm} {} 0 {} {} {gain:e}\n", o.0, plus.0, minus.0),
        }
    }
    out += ".model DIODE D (IS=1e-14)\n.model LED D (IS=1e-14)\n";
    out + &models + ".end\n"
}

pub fn from_spice(text: &str) -> Result<Circuit, SimError> {
    // join "+" continuation lines, drop the title line and comments
    let mut lines: Vec<(usize, String)> = Vec::new();
    for (i, raw) in text.lines().enumerate().skip(1) {
        let l = raw.trim();
        if l.is_empty() || l.starts_with('*') { continue }
        match (l.strip_prefix('+'), lines.last_mut()) {
            (Some(rest), Some((_, prev))) => { prev.push(' '); prev.push_str(rest) }
            _ => lines.push((i + 1, l.to_string())),
        }
    }
    let mut models: HashMap<String, (String, HashMap<String, f64>)> = HashMap::new();
    for (n, l) in &lines {
        if !l.to_ascii_lowercase().starts_with(".model") { continue }
        let t: Vec<String> = l.replace(['(', ')', '='], " ").split_whitespace().map(str::to_ascii_lowercase).collect();
        if t.len() < 3 { return Err(bad(*n, ".model needs a name and a type")) }
        let mut params = HashMap::new();
        for kv in t[3..].chunks(2) {
            if let [k, v] = kv { params.insert(k.clone(), value(v).ok_or_else(|| bad(*n, format!("bad value \"{v}\"")))?); }
        }
        models.insert(t[1].clone(), (t[2].clone(), params));
    }

    let mut nodes: HashMap<String, u32> = HashMap::new();
    let mut parts = Vec::new();
    for (n, l) in &lines {
        let t: Vec<&str> = l.split_whitespace().collect();
        let name = t[0];
        let first = name.chars().next().unwrap().to_ascii_uppercase();
        if first == '.' { continue } // .end, .op, .tran ...: not part of the circuit
        let mut node = |s: &str| {
            let k = s.to_ascii_lowercase();
            if k == "0" || k == "gnd" { return NodeId(0) }
            let next = nodes.len() as u32 + 1;
            NodeId(*nodes.entry(k).or_insert(next))
        };
        let need = |k: usize| if t.len() < k { Err(bad(*n, format!("{name} needs {} fields", k - 1))) } else { Ok(()) };
        let num = |s: &str| value(s).ok_or_else(|| bad(*n, format!("bad value \"{s}\"")));
        let model = |s: &str| models.get(&s.to_ascii_lowercase()).ok_or_else(|| bad(*n, format!("unknown model \"{s}\"")));
        let src = || -> Result<f64, SimError> { // "DC 5" or just "5"
            let v = if t.get(3).is_some_and(|s| s.eq_ignore_ascii_case("dc")) { t.get(4) } else { t.get(3) };
            num(v.ok_or_else(|| bad(*n, format!("{name} has no value")))?)
        };
        let kind = match first {
            'R' | 'C' | 'L' => {
                need(4)?;
                let (a, b, v) = (node(t[1]), node(t[2]), num(t[3])?);
                match first {
                    'R' => PartKind::Resistor { a, b, ohms: v, max_watts: 0.25 },
                    'C' => PartKind::Capacitor { a, b, farads: v },
                    _ => PartKind::Inductor { a, b, henries: v },
                }
            }
            'V' => {
                need(4)?;
                let u: Vec<&str> = l.split(['(', ')', ' ', '\t']).filter(|s| !s.is_empty()).collect();
                let arg = |k: usize| num(u.get(k).ok_or_else(|| bad(*n, format!("{name} has too few waveform values")))?);
                let (volts, wave) = match u.get(3).ok_or_else(|| bad(*n, format!("{name} has no value")))?.to_ascii_lowercase().as_str() {
                    "sin" => (arg(4)?, Some(Wave::Sine { amp: arg(5)?, freq: arg(6)?, phase: if u.get(9).is_some() { arg(9)? } else { 0.0 } })), // SIN(vo va freq td theta phase): td and theta are dropped
                    // shortcut: a PULSE with no flat top and a symmetric (triangle) or one-sided (sawtooth) edge pair is read as that wave; any other PULSE becomes an ideal square (duty = width / period) and a delay becomes a phase advance, rise/fall times are dropped, upgrade when they are modelled
                    "pulse" => {
                        let (v1, v2, td, tr, tf, pw, per) = (arg(4)?, arg(5)?, arg(6)?, arg(7)?, arg(8)?, arg(9)?, arg(10)?);
                        let (amp, freq, eq) = ((v2 - v1) / 2.0, 1.0 / per, |a: f64, b: f64| (a - b).abs() < 1e-6 * per);
                        let f = (td / per).rem_euclid(1.0); // the delay inside one period is the phase advance left over
                        let phase = if f < 1e-9 || 1.0 - f < 1e-9 { 0.0 } else { 360.0 * (1.0 - f) };
                        let wave = if eq(pw, 0.0) && eq(tr, per / 2.0) && eq(tf, per / 2.0) { Wave::Triangle { amp, freq, phase } }
                            else if eq(pw, 0.0) && eq(tr, per) && eq(tf, 0.0) { Wave::Sawtooth { amp, freq, phase } }
                            else { Wave::Square { amp, freq, duty: pw / per, phase } };
                        ((v1 + v2) / 2.0, Some(wave))
                    }
                    _ => (src()?, None),
                };
                PartKind::VSource { pos: node(t[1]), neg: node(t[2]), volts, wave }
            }
            'I' => { need(4)?; PartKind::ISource { from: node(t[1]), to: node(t[2]), amps: src()? } }
            'D' => {
                need(4)?;
                let (anode, cathode) = (node(t[1]), node(t[2]));
                if t[3].to_ascii_lowercase().starts_with("led") { PartKind::Led { anode, cathode, max_amps: 0.02 } } else { PartKind::Diode { anode, cathode } }
            }
            'Q' => {
                need(5)?;
                let (ty, p) = model(t[4])?;
                PartKind::Bjt { c: node(t[1]), b: node(t[2]), e: node(t[3]), pnp: ty == "pnp", beta: p.get("bf").copied().unwrap_or(100.0), max_amps: 0.1 }
            }
            'M' => {
                need(6)?;
                let (ty, p) = model(t[5])?;
                PartKind::Mosfet { d: node(t[1]), g: node(t[2]), s: node(t[3]), pmos: ty == "pmos", vth: p.get("vto").copied().unwrap_or(0.0).abs(), kp: p.get("kp").copied().unwrap_or(2e-5), max_amps: 0.5 }
            }
            'E' => {
                need(6)?;
                let (out, plus, minus) = (node(t[1]), node(t[3]), node(t[4]));
                if node(t[2]).0 != 0 { return Err(bad(*n, "E source: only a ground-referenced output (negative output node 0) is supported")) }
                PartKind::Opamp { out, plus, minus, gain: num(t[5])?, vsat: 12.0, max_amps: 0.025 }
            }
            _ => return Err(bad(*n, format!("unsupported element \"{name}\""))),
        };
        if parts.iter().any(|q: &Part| q.id == name) { return Err(bad(*n, format!("duplicate part \"{name}\""))) }
        parts.push(Part { id: name.to_string(), kind });
    }
    if parts.is_empty() { return Err(SimError::BadFile("no circuit elements found".into())) }
    Ok(Circuit { parts })
}
