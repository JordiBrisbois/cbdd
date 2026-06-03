use super::*;

#[tauri::command]
pub fn rechercher_personnes_par_email(
    app: AppHandle,
    email: String,
) -> Result<Vec<Personne>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "personnes.read")?;
    let mut stmt = conn
        .prepare(
            "SELECT * FROM T_Personnes \
             WHERE LOWER(Email_Prive) = LOWER(?) \
             AND COALESCE(Statut_Compte, '') != 'Anonymisé' \
             ORDER BY Nom ASC, Prenom ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![email], map_personne)
        .map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[tauri::command]
pub fn rechercher_structures_par_email(
    app: AppHandle,
    email: String,
) -> Result<Vec<Structure>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "structures.read")?;
    let mut stmt = conn
        .prepare(
            "SELECT * FROM T_Structures \
             WHERE LOWER(Email_General) = LOWER(?) \
             ORDER BY Nom_Structure ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![email], map_structure)
        .map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}
