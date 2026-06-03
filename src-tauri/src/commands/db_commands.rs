use super::*;

#[tauri::command]
pub fn get_db_status(app: AppHandle) -> Result<serde_json::Value, String> {
    let (connected, path, error) = db::status(&app);
    Ok(serde_json::json!({ "connected": connected, "path": path, "error": error }))
}

#[tauri::command]
pub fn pick_db(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let file = app
        .dialog()
        .file()
        .add_filter("Base de données", &["sqlite", "db", "sqlite3"])
        .blocking_pick_file();

    match file {
        Some(f) => {
            let path = f.to_string();
            db::connect(&path)?;
            db::save_path(&app, &path);
            println!("[CRVI-GRC] DB connectée: {}", path);
            Ok(Some(path))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub fn reconnect_db(app: AppHandle) -> Result<(), String> {
    db::get_conn(&app)?;
    Ok(())
}
