pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::run_simulation,
            commands::save_circuit,
            commands::load_circuit
        ])
        .run(tauri::generate_context!())
        .expect("error while running Nodal");
}
