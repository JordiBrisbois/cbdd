use super::finalize_snapshot_as_backup_impl;
use super::BackupMode;
use chrono::Local;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

pub fn verify_backup_file_internal(path: &Path) -> Result<bool, String> {
    let conn =
        Connection::open(path).map_err(|e| format!("Impossible d'ouvrir le backup: {}", e))?;
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| format!("Impossible de vérifier le backup: {}", e))?;
    Ok(integrity.eq_ignore_ascii_case("ok"))
}

pub fn create_emergency_backup_from_current_db(
    app: &tauri::AppHandle,
    db_path: &Path,
    backup_dir: &Path,
) -> Result<String, String> {
    let temp_snapshot = backup_dir.join(format!(
        "pre_restore_{}.tmp.sqlite",
        Local::now().format("%Y-%m-%d_%H-%M-%S")
    ));
    if temp_snapshot.exists() {
        let _ = fs::remove_file(&temp_snapshot);
    }
    fs::copy(db_path, &temp_snapshot).map_err(|e| {
        format!(
            "Impossible de copier la base actuelle avant restauration: {}",
            e
        )
    })?;

    let finalize_result = finalize_snapshot_as_backup_impl(
        &temp_snapshot,
        backup_dir,
        false,
        &Local::now().format("%Y-%m-%d_%H-%M-%S").to_string(),
        BackupMode::WindowsLocal,
        "pre_restore_local",
    );
    let _ = fs::remove_file(&temp_snapshot);
    let backup = finalize_result?;
    let _ = app;
    Ok(backup.path)
}
