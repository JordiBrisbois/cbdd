use super::*;

#[tauri::command]
pub fn run_auto_backup_check(app: AppHandle) -> Result<BackupRunResult, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.backups")?;
    let (_, holder_label) = auth::current_actor_label(&conn)?;
    backups::maybe_run_auto_backup(&app, &conn, &holder_label, &machine_label())
}

#[tauri::command]
pub fn create_manual_backup(
    app: AppHandle,
    request: ManualBackupRequest,
) -> Result<BackupRunResult, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.backups")?;
    backups::create_backup_now(&app, &conn, request)
}

#[tauri::command]
pub fn list_local_backups(app: AppHandle) -> Result<Vec<BackupInfo>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.backups")?;
    drop(conn);
    backups::list_local_backups(&app)
}

#[tauri::command]
pub fn get_backup_directory(app: AppHandle) -> Result<String, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.backups")?;
    drop(conn);
    backups::get_backup_directory(&app)
}

#[tauri::command]
pub fn open_backup_directory(app: AppHandle) -> Result<String, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.backups")?;
    drop(conn);
    backups::open_backup_directory(&app)
}

#[tauri::command]
pub fn restore_local_backup(
    app: AppHandle,
    request: RestoreBackupRequest,
) -> Result<BackupRunResult, String> {
    let conn = db::get_conn(&app)?;
    let session = auth::require_permission(&conn, "admin.backups")?;
    let preferred_admin_username = session.username.clone();
    drop(conn);
    backups::restore_backup(&app, preferred_admin_username.as_deref(), request)
}

#[tauri::command]
pub fn delete_local_backup(app: AppHandle, backup_path: String) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.backups")?;
    drop(conn);
    backups::delete_backup(&app, &backup_path)
}

#[tauri::command]
pub fn exporter_classeur_excel_admin(app: AppHandle) -> Result<Option<ExcelRebuildResult>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.exports")?;
    excel_export::export_reconstructed_excel(&app, &conn)
}
