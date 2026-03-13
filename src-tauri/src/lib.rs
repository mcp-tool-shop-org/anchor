pub mod commands;
pub mod domain;
pub mod drift_rules;
pub mod export_compiler;
pub mod readiness_gate;
pub mod stale_propagation;
pub mod state_machine;
pub mod store;
pub mod traceability;

/// Tauri app entry point. Called from main.rs.
pub fn run() {
    use std::sync::Mutex;

    tauri::Builder::default()
        .manage(Mutex::new(store::ProjectStore::demo()))
        .invoke_handler(tauri::generate_handler![
            commands::get_project_snapshot,
            commands::get_artifact_detail,
            commands::get_readiness_gate,
            commands::get_export_preview,
            commands::transition_artifact,
            commands::approve_artifact,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Anchor");
}
