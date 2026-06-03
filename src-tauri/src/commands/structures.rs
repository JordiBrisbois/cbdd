use super::*;

#[tauri::command]
pub fn lister_structures(
    app: AppHandle,
    recherche: Option<String>,
) -> Result<Vec<Structure>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "structures.read")?;
    let mut sql = String::from("SELECT s.* FROM T_Structures s");
    let mut params: Vec<String> = Vec::new();

    if let Some(ref search) = recherche {
        if !search.is_empty() {
            sql.push_str(" WHERE s.Nom_Structure LIKE ? OR s.Commune_Structure LIKE ?");
            let like = format!("%{}%", search);
            params.push(like.clone());
            params.push(like);
        }
    }
    sql.push_str(" ORDER BY s.Nom_Structure ASC");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
        .iter()
        .map(|p| p as &dyn rusqlite::types::ToSql)
        .collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), map_structure)
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[tauri::command]
pub fn get_structure(app: AppHandle, id: i64) -> Result<Structure, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "structures.read")?;
    conn.query_row(
        "SELECT * FROM T_Structures WHERE ID_Structure = ?",
        rusqlite::params![id],
        map_structure,
    )
    .map_err(|e| format!("Structure introuvable: {}", e))
}

#[tauri::command]
pub fn sauvegarder_structure(
    app: AppHandle,
    structure: StructureInput,
) -> Result<Structure, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(
        &conn,
        if structure.id_structure.is_some() {
            "structures.update"
        } else {
            "structures.create"
        },
    )?;

    if let Some(id) = structure.id_structure {
        ensure_resource_not_locked_by_other(&conn, "structures", id)?;
        conn.execute(
            "UPDATE T_Structures SET
                Nom_Structure = ?, Service_Specifique = ?,
                Reseau_Subvention = ?, Partenaire_Direct = ?,
                Adresse_Structure = ?, Code_Postal_Structure = ?,
                Commune_Structure = ?, Pays = ?, Telephone_General = ?,
                Email_General = ?, Site_Web = ?, Notes_Commentaires = ?,
                ID_Categorie = ?, Updated_At = datetime('now')
             WHERE ID_Structure = ? AND COALESCE(Updated_At, '') = COALESCE(?, '')",
            rusqlite::params![
                structure.nom_structure,
                structure.service_specifique,
                structure.reseau_subvention,
                structure.partenaire_direct,
                structure.adresse_structure,
                structure.code_postal_structure,
                structure.commune_structure,
                structure.pays,
                structure.telephone_general,
                structure.email_general,
                structure.site_web,
                structure.notes_commentaires,
                structure.id_categorie,
                id,
                structure.original_updated_at
            ],
        )
        .map_err(|e| e.to_string())?;
        if conn.changes() == 0 {
            return Err("Conflit: cette structure a été modifiée ailleurs. Rechargez la fiche avant de réessayer.".into());
        }
        get_structure(app, id)
    } else {
        conn.execute(
            "INSERT INTO T_Structures
                (Nom_Structure, Service_Specifique,
                 Reseau_Subvention, Partenaire_Direct, Adresse_Structure,
                 Code_Postal_Structure, Commune_Structure, Pays,
                 Telephone_General, Email_General, Site_Web,
                 Notes_Commentaires, ID_Categorie, Date_Creation, Updated_At)
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?, date('now'), datetime('now'))",
            rusqlite::params![
                structure.nom_structure,
                structure.service_specifique,
                structure.reseau_subvention,
                structure.partenaire_direct,
                structure.adresse_structure,
                structure.code_postal_structure,
                structure.commune_structure,
                structure.pays,
                structure.telephone_general,
                structure.email_general,
                structure.site_web,
                structure.notes_commentaires,
                structure.id_categorie
            ],
        )
        .map_err(|e| e.to_string())?;
        let new_id = conn.last_insert_rowid();
        get_structure(app, new_id)
    }
}

#[tauri::command]
pub fn supprimer_structure(app: AppHandle, id: i64) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "structures.delete")?;
    ensure_resource_not_locked_by_other(&conn, "structures", id)?;
    conn.execute(
        "DELETE FROM T_Affiliations WHERE Ref_Structure = ?",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE T_Reunions SET Ref_Structure = NULL WHERE Ref_Structure = ?",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM T_Structures WHERE ID_Structure = ?",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
