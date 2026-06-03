use super::*;

#[tauri::command]
pub fn lister_categories(app: AppHandle) -> Result<Vec<Categorie>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "categories.read")?;
    let mut stmt = conn
        .prepare("SELECT * FROM T_Categories ORDER BY Nom_Categorie ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_categorie)
        .map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[tauri::command]
pub fn sauvegarder_categorie(
    app: AppHandle,
    id: Option<i64>,
    nom: String,
) -> Result<Categorie, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(
        &conn,
        if id.is_some() {
            "categories.update"
        } else {
            "categories.create"
        },
    )?;
    if let Some(cat_id) = id {
        conn.execute(
            "UPDATE T_Categories SET Nom_Categorie = ? WHERE ID_Categorie = ?",
            rusqlite::params![nom, cat_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(Categorie {
            id_categorie: cat_id,
            nom_categorie: Some(nom),
        })
    } else {
        conn.execute(
            "INSERT INTO T_Categories (Nom_Categorie) VALUES (?)",
            rusqlite::params![nom],
        )
        .map_err(|e| e.to_string())?;
        let new_id = conn.last_insert_rowid();
        Ok(Categorie {
            id_categorie: new_id,
            nom_categorie: Some(nom),
        })
    }
}

#[tauri::command]
pub fn supprimer_categorie(app: AppHandle, id: i64) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "categories.delete")?;
    let count_aff: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM T_Affiliations WHERE ID_Categorie = ?",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let count_struct: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM T_Structures WHERE ID_Categorie = ?",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if count_aff > 0 || count_struct > 0 {
        return Err("Cette catégorie est rattachée à des affiliations ou structures. Supprimez d'abord les liens concernés.".into());
    }
    conn.execute(
        "DELETE FROM T_Categories WHERE ID_Categorie = ?",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
