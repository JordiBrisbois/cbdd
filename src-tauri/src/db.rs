use crate::auth;
use once_cell::sync::Lazy;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

const DEFAULT_DB_NAME: &str = "CRVI_GRC_be.sqlite";

pub static DB_STATE: Lazy<Mutex<DbState>> = Lazy::new(|| Mutex::new(DbState { path: None }));

pub struct DbState {
    pub path: Option<String>,
}

fn open_existing(path: &str) -> Result<Connection, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!(
            "Base introuvable: {}. Utilisez 'Choisir une base' pour sélectionner un fichier existant.",
            path
        ));
    }
    if !p.is_file() {
        return Err(format!("Chemin BDD invalide (pas un fichier): {}", path));
    }

    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)
        .map_err(|e| format!("Erreur ouverture BDD: {}", e))
}

fn candidate_paths(app: &tauri::AppHandle) -> Vec<String> {
    let mut candidates = Vec::new();

    if let Some(path) = current_path() {
        candidates.push(path);
    }
    if let Some(path) = load_path(app) {
        candidates.push(path);
    }
    if let Some(path) = discover_db(app) {
        candidates.push(path);
    }

    let mut unique = Vec::new();
    for candidate in candidates {
        if !unique.iter().any(|existing| existing == &candidate) {
            unique.push(candidate);
        }
    }
    unique
}

fn apply_connection_pragmas(conn: &Connection) {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
        PRAGMA journal_mode = DELETE;
        PRAGMA synchronous = NORMAL;
        ",
    )
    .ok();
}

pub fn connect(path: &str) -> Result<(), String> {
    let conn = open_existing(path)?;
    apply_connection_pragmas(&conn);
    auth::ensure_security_schema(&conn)?;
    let mut state = DB_STATE
        .lock()
        .map_err(|e| format!("Erreur mutex: {}", e))?;
    drop(conn);
    auth::logout()?;
    state.path = Some(path.to_string());
    Ok(())
}

pub fn current_path() -> Option<String> {
    DB_STATE.lock().ok()?.path.clone()
}

pub fn get_conn(app: &tauri::AppHandle) -> Result<Connection, String> {
    let candidates = candidate_paths(app);
    if candidates.is_empty() {
        return Err(
            "Aucune base de données. Utilisez 'Choisir une base' pour sélectionner un fichier."
                .into(),
        );
    }

    let mut last_error = None;

    for path in candidates {
        match open_existing(&path) {
            Ok(conn) => {
                apply_connection_pragmas(&conn);
                auth::ensure_security_schema(&conn)?;
                {
                    let mut state = DB_STATE
                        .lock()
                        .map_err(|e| format!("Erreur mutex: {}", e))?;
                    if state.path.as_deref() != Some(path.as_str()) {
                        auth::logout()?;
                    }
                    state.path = Some(path.clone());
                }
                save_path(app, &path);
                return Ok(conn);
            }
            Err(err) => {
                last_error = Some(err);
            }
        }
    }

    {
        let mut state = DB_STATE
            .lock()
            .map_err(|e| format!("Erreur mutex: {}", e))?;
        state.path = None;
    }

    Err(last_error.unwrap_or_else(|| {
        "Aucune base de données valide. Utilisez 'Choisir une base' pour sélectionner un fichier.".into()
    }))
}

pub fn status(app: &tauri::AppHandle) -> (bool, Option<String>, Option<String>) {
    let candidates = candidate_paths(app);
    let path = candidates.first().cloned();
    let mut last_error = None;

    for candidate in candidates {
        match open_existing(&candidate) {
            Ok(conn) => {
                drop(conn);
                return (true, Some(candidate), None);
            }
            Err(err) => {
                last_error = Some(err);
            }
        }
    }

    (false, path, last_error)
}

pub fn discover_db(app: &tauri::AppHandle) -> Option<String> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()));
    let data_dir = app.path().app_data_dir().ok();

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(DEFAULT_DB_NAME));
    }
    if let Some(ref dir) = exe_dir {
        candidates.push(dir.join(DEFAULT_DB_NAME));
    }
    if let Some(ref dir) = data_dir {
        candidates.push(dir.join(DEFAULT_DB_NAME));
    }

    for p in candidates {
        if p.exists() {
            return Some(p.to_string_lossy().to_string());
        }
    }
    None
}

pub fn config_file(app: &tauri::AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    std::fs::create_dir_all(&dir).ok();
    dir.join("db_path.txt")
}

pub fn save_path(app: &tauri::AppHandle, path: &str) {
    if let Ok(cf) = std::fs::canonicalize(path) {
        let _ = std::fs::write(config_file(app), cf.to_string_lossy().as_ref());
    }
}

pub fn load_path(app: &tauri::AppHandle) -> Option<String> {
    let cf = config_file(app);
    if cf.exists() {
        std::fs::read_to_string(&cf).ok().filter(|s| !s.is_empty())
    } else {
        None
    }
}
