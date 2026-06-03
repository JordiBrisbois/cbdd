use super::*;

#[tauri::command]
pub fn lister_fonctions(app: AppHandle) -> Result<Vec<Fonction>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "affiliations.read")?;
    let mut stmt = conn
        .prepare("SELECT * FROM T_Fonctions ORDER BY Libelle_Fonction ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_fonction)
        .map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[tauri::command]
pub fn sauvegarder_fonction(
    app: AppHandle,
    id: Option<i64>,
    libelle: String,
) -> Result<Fonction, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(
        &conn,
        if id.is_some() {
            "affiliations.update"
        } else {
            "affiliations.create"
        },
    )?;
    if let Some(f_id) = id {
        conn.execute(
            "UPDATE T_Fonctions SET Libelle_Fonction = ? WHERE ID_Fonction = ?",
            rusqlite::params![libelle, f_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(Fonction {
            id_fonction: f_id,
            libelle_fonction: Some(libelle),
        })
    } else {
        conn.execute(
            "INSERT INTO T_Fonctions (Libelle_Fonction) VALUES (?)",
            rusqlite::params![libelle],
        )
        .map_err(|e| e.to_string())?;
        let new_id = conn.last_insert_rowid();
        Ok(Fonction {
            id_fonction: new_id,
            libelle_fonction: Some(libelle),
        })
    }
}

#[tauri::command]
pub fn supprimer_fonction(app: AppHandle, id: i64) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "affiliations.delete")?;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM T_Affiliations WHERE Ref_Fonction = ?",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if count > 0 {
        return Err("Cette fonction est rattachée à des affiliations. Supprimez d'abord les affiliations concernées.".into());
    }
    conn.execute(
        "DELETE FROM T_Fonctions WHERE ID_Fonction = ?",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
