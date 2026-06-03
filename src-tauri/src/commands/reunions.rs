use super::*;

#[tauri::command]
pub fn lister_reunions(app: AppHandle, recherche: Option<String>) -> Result<Vec<Reunion>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "reunions.read")?;
    let mut sql = String::from(
        "SELECT r.*, s.Nom_Structure
         FROM T_Reunions r
         LEFT JOIN T_Structures s ON s.ID_Structure = r.Ref_Structure",
    );
    let mut params: Vec<String> = Vec::new();

    if let Some(ref search) = recherche {
        if !search.is_empty() {
            sql.push_str(
                " WHERE r.Titre_Reunion LIKE ?
                   OR r.Date_Reunion LIKE ?
                   OR r.Heure_Reunion LIKE ?
                   OR r.Lieu_Reunion LIKE ?
                   OR r.Notes_Commentaires LIKE ?
                   OR s.Nom_Structure LIKE ?",
            );
            let like = format!("%{}%", search);
            params.push(like.clone());
            params.push(like.clone());
            params.push(like.clone());
            params.push(like.clone());
            params.push(like.clone());
            params.push(like);
        }
    }
    sql.push_str(" ORDER BY r.Date_Reunion DESC, r.Heure_Reunion DESC");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
        .iter()
        .map(|p| p as &dyn rusqlite::types::ToSql)
        .collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), map_reunion)
        .map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[tauri::command]
pub fn get_reunion(app: AppHandle, id: i64) -> Result<Reunion, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "reunions.read")?;
    conn.query_row(
        "SELECT r.*, s.Nom_Structure
         FROM T_Reunions r
         LEFT JOIN T_Structures s ON s.ID_Structure = r.Ref_Structure
         WHERE r.ID_Reunion = ?",
        rusqlite::params![id],
        map_reunion,
    )
    .map_err(|e| format!("Réunion introuvable: {}", e))
}

#[tauri::command]
pub fn sauvegarder_reunion(app: AppHandle, reunion: ReunionInput) -> Result<Reunion, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(
        &conn,
        if reunion.id_reunion.is_some() {
            "reunions.update"
        } else {
            "reunions.create"
        },
    )?;

    if let Some(id) = reunion.id_reunion {
        ensure_resource_not_locked_by_other(&conn, "reunions", id)?;
        conn.execute(
            "UPDATE T_Reunions SET
                Titre_Reunion = ?, Date_Reunion = ?, Heure_Reunion = ?,
                Lieu_Reunion = ?, Ref_Structure = ?, Notes_Commentaires = ?,
                Updated_At = datetime('now')
             WHERE ID_Reunion = ? AND COALESCE(Updated_At, '') = COALESCE(?, '')",
            rusqlite::params![
                reunion.titre_reunion,
                reunion.date_reunion,
                reunion.heure_reunion,
                reunion.lieu_reunion,
                reunion.ref_structure,
                reunion.notes_commentaires,
                id,
                reunion.original_updated_at
            ],
        )
        .map_err(|e| e.to_string())?;
        if conn.changes() == 0 {
            return Err("Conflit: cette réunion a été modifiée ailleurs. Rechargez la fiche avant de réessayer.".into());
        }
        get_reunion(app, id)
    } else {
        conn.execute(
            "INSERT INTO T_Reunions
                (Titre_Reunion, Date_Reunion, Heure_Reunion, Lieu_Reunion,
                 Ref_Structure, Notes_Commentaires, Updated_At)
             VALUES (?,?,?,?,?,?, datetime('now'))",
            rusqlite::params![
                reunion.titre_reunion,
                reunion.date_reunion,
                reunion.heure_reunion,
                reunion.lieu_reunion,
                reunion.ref_structure,
                reunion.notes_commentaires
            ],
        )
        .map_err(|e| e.to_string())?;
        let new_id = conn.last_insert_rowid();
        get_reunion(app, new_id)
    }
}

#[tauri::command]
pub fn supprimer_reunion(app: AppHandle, id: i64) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "reunions.delete")?;
    ensure_resource_not_locked_by_other(&conn, "reunions", id)?;
    conn.execute(
        "DELETE FROM T_Presences WHERE Ref_Reunion = ?",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM T_Reunions WHERE ID_Reunion = ?",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
