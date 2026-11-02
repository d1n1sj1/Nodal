use crate::{transient::TransientResult, Circuit, PartKind};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DamageKind { OverPower, OverCurrent }

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DamageWarning { pub part_id: String, pub kind: DamageKind, pub peak: f64, pub limit: f64 }

/// Peak of `f` over samples; a non-finite sample counts as an infinite peak (still warns).
fn peak(s: &[f64], f: impl Fn(f64) -> f64) -> f64 {
    s.iter().map(|&i| f(i)).map(|v| if v.is_finite() { v } else { f64::INFINITY }).fold(0.0, f64::max)
}

/// One warning per over-limit part, in `c.parts` order. Parts without a current series are skipped.
pub fn check_damage(c: &Circuit, r: &TransientResult) -> Vec<DamageWarning> {
    c.parts.iter().filter_map(|p| {
        let s = r.part_currents.get(&p.id)?;
        let (kind, peak, limit) = match p.kind {
            PartKind::Resistor { ohms, max_watts, .. } => (DamageKind::OverPower, peak(s, |i| i * i * ohms), max_watts),
            PartKind::Led { max_amps, .. } => (DamageKind::OverCurrent, peak(s, f64::abs), max_amps),
            PartKind::Bjt { max_amps, .. } => (DamageKind::OverCurrent, peak(s, f64::abs), max_amps),
            PartKind::Mosfet { max_amps, .. } => (DamageKind::OverCurrent, peak(s, f64::abs), max_amps),
            PartKind::Opamp { max_amps, .. } => (DamageKind::OverCurrent, peak(s, f64::abs), max_amps),
            _ => return None,
        };
        (peak > limit).then(|| DamageWarning { part_id: p.id.clone(), kind, peak, limit })
    }).collect()
}
