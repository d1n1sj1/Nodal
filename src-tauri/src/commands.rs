//! Thin wrappers over nodal-core; no solver logic here.
use nodal_core::transient::transient;
use nodal_core::{check_damage, Circuit, DamageWarning};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimOutput {
    pub times: Vec<f64>,
    pub node_voltages: Vec<Vec<f64>>,
    pub part_currents: HashMap<String, Vec<f64>>,
    pub warnings: Vec<DamageWarning>,
}

/// serde_json writes non-finite floats as null; clamp so the UI always gets a number.
pub fn clamp_peak(mut w: DamageWarning) -> DamageWarning {
    if !w.peak.is_finite() {
        w.peak = f64::MAX;
    }
    w
}

pub fn run_simulation_impl(circuit_json: &str, stop: f64, step: f64) -> Result<SimOutput, String> {
    let c = Circuit::from_json(circuit_json).map_err(|e| e.to_string())?;
    let r = transient(&c, stop, step).map_err(|e| e.to_string())?;
    let warnings = check_damage(&c, &r).into_iter().map(clamp_peak).collect();
    Ok(SimOutput { times: r.times, node_voltages: r.node_voltages, part_currents: r.part_currents, warnings })
}

pub fn save_circuit_impl(path: &str, circuit_json: &str) -> Result<(), String> {
    Circuit::from_json(circuit_json).map_err(|e| e.to_string())?;
    // Write a sibling temp file then rename, so a failed write cannot truncate an existing save.
    let tmp = format!("{path}.tmp");
    let err = |e: std::io::Error| format!("could not write {path}: {e}");
    std::fs::write(&tmp, circuit_json).map_err(err)?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        err(e)
    })
}

pub fn load_circuit_impl(path: &str) -> Result<String, String> {
    let s = std::fs::read_to_string(path).map_err(|e| format!("could not read {path}: {e}"))?;
    Circuit::from_json(&s).map_err(|e| e.to_string())?;
    Ok(s)
}

/// `async` runs commands on the threadpool: heavy runs stay off the main thread and a panic cannot abort the app.
#[tauri::command(async)]
pub fn run_simulation(circuit_json: String, stop: f64, step: f64) -> Result<SimOutput, String> {
    run_simulation_impl(&circuit_json, stop, step)
}

/// `path` always comes from the native dialog plugin (the UI never builds paths itself).
#[tauri::command(async)]
pub fn save_circuit(path: String, circuit_json: String) -> Result<(), String> {
    save_circuit_impl(&path, &circuit_json)
}

#[tauri::command(async)]
pub fn load_circuit(path: String) -> Result<String, String> {
    load_circuit_impl(&path)
}
