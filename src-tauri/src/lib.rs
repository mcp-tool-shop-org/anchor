pub mod amendments;
pub mod audit_log;
pub mod commands;
pub mod domain;
pub mod drift_rules;
pub mod editing;
pub mod export_compiler;
pub mod persistence;
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
            commands::edit_artifact_content,
            commands::propose_amendment,
            commands::assess_amendment_impact,
            commands::apply_amendment,
            commands::abandon_amendment,
            commands::get_audit_timeline,
            commands::get_artifact_history,
            commands::save_project,
            commands::load_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Anchor");
}
