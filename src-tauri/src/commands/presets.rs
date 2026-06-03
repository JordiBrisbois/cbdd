use super::*;
use crate::auth;
use crate::db;
use tauri::AppHandle;

fn ensure_presets_table(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS T_Presets (
            ID_Preset INTEGER PRIMARY KEY AUTOINCREMENT,
            Nom_Preset TEXT NOT NULL,
            Table_Principale TEXT NOT NULL,
            Colonnes TEXT NOT NULL,
            Conditions TEXT,
            Date_Creation TEXT DEFAULT (datetime('now')),
            Date_Modification TEXT DEFAULT (datetime('now'))
        )",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn lister_presets_impl(app: AppHandle) -> Result<Vec<Preset>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "presets.read")?;
    ensure_presets_table(&conn)?;
    let mut stmt = conn
        .prepare(
            "SELECT ID_Preset, Nom_Preset, Table_Principale, Colonnes, Conditions, Date_Creation, Date_Modification FROM T_Presets ORDER BY Nom_Preset ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Preset {
                id_preset: row.get(0)?,
                nom_preset: row.get(1)?,
                table_principale: row.get(2)?,
                colonnes: row.get(3)?,
                conditions: row.get(4)?,
                date_creation: row.get(5)?,
                date_modification: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

pub fn sauvegarder_preset_impl(app: AppHandle, preset: PresetInput) -> Result<Preset, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(
        &conn,
        if preset.id_preset.is_some() {
            "presets.update"
        } else {
            "presets.create"
        },
    )?;
    ensure_presets_table(&conn)?;
    if let Some(id) = preset.id_preset {
        conn.execute(
            "UPDATE T_Presets SET Nom_Preset = ?, Table_Principale = ?, Colonnes = ?, Conditions = ?, Date_Modification = datetime('now') WHERE ID_Preset = ?",
            rusqlite::params![preset.nom_preset, preset.table_principale, preset.colonnes, preset.conditions, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(Preset {
            id_preset: id,
            nom_preset: preset.nom_preset,
            table_principale: preset.table_principale,
            colonnes: preset.colonnes,
            conditions: preset.conditions,
            date_creation: None,
            date_modification: None,
        })
    } else {
        conn.execute(
            "INSERT INTO T_Presets (Nom_Preset, Table_Principale, Colonnes, Conditions) VALUES (?, ?, ?, ?)",
            rusqlite::params![preset.nom_preset, preset.table_principale, preset.colonnes, preset.conditions],
        )
        .map_err(|e| e.to_string())?;
        let new_id = conn.last_insert_rowid();
        Ok(Preset {
            id_preset: new_id,
            nom_preset: preset.nom_preset,
            table_principale: preset.table_principale,
            colonnes: preset.colonnes,
            conditions: preset.conditions,
            date_creation: None,
            date_modification: None,
        })
    }
}

pub fn supprimer_preset_impl(app: AppHandle, id: i64) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "presets.delete")?;
    ensure_presets_table(&conn)?;
    conn.execute(
        "DELETE FROM T_Presets WHERE ID_Preset = ?",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn charger_preset_impl(app: AppHandle, id: i64) -> Result<Preset, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "presets.read")?;
    ensure_presets_table(&conn)?;
    conn.query_row(
        "SELECT ID_Preset, Nom_Preset, Table_Principale, Colonnes, Conditions, Date_Creation, Date_Modification FROM T_Presets WHERE ID_Preset = ?",
        rusqlite::params![id],
        |row| {
            Ok(Preset {
                id_preset: row.get(0)?,
                nom_preset: row.get(1)?,
                table_principale: row.get(2)?,
                colonnes: row.get(3)?,
                conditions: row.get(4)?,
                date_creation: row.get(5)?,
                date_modification: row.get(6)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}
