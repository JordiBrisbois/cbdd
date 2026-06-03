use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tauri::Manager;
use crate::db;

const BACKUP_EXTENSION: &str = "crvibak";
const LEGACY_SQLITE_EXTENSION: &str = "sqlite";

pub fn current_db_path(app: &AppHandle) -> Result<PathBuf, String> {
    if let Some(path) = db::current_path() {
        return Ok(PathBuf::from(path));
    }
    if let Some(path) = db::load_path(app) {
        return Ok(PathBuf::from(path));
    }
    Err("Aucune base connectée pour lancer un backup.".into())
}

pub fn backup_dir_for_db(app: &AppHandle, db_path: &Path) -> Result<PathBuf, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Impossible de déterminer le dossier applicatif: {}", e))?;
    let stem = db_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("crvi");
    let mut hasher = DefaultHasher::new();
    db_path.to_string_lossy().hash(&mut hasher);
    let suffix = hasher.finish();
    Ok(app_data
        .join("backups")
        .join(format!("{stem}_{suffix:016x}")))
}

pub fn is_supported_backup_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(BACKUP_EXTENSION | LEGACY_SQLITE_EXTENSION)
    )
}

pub fn ensure_backup_dir(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path)
        .map_err(|e| format!("Impossible de créer le dossier de backup: {}", e))?;
    super::recovery_files::ensure_recovery_support_files(path)
}
