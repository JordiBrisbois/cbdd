use super::*;

#[tauri::command]
pub fn lister_personnes(
    app: AppHandle,
    recherche: Option<String>,
    categorie_id: Option<i64>,
) -> Result<Vec<Personne>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "personnes.read")?;

    let mut sql = String::from("SELECT DISTINCT p.* FROM T_Personnes p");
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if categorie_id.is_some() {
        sql.push_str(" INNER JOIN T_Affiliations a ON a.Ref_Personne = p.ID_Personne");
    }

    let mut conditions: Vec<String> =
        vec!["COALESCE(p.Statut_Compte, '') != 'Anonymisé'".to_string()];

    if let Some(cat_id) = categorie_id {
        conditions.push("a.ID_Categorie = ?".to_string());
        params.push(Box::new(cat_id));
    }

    if let Some(ref search) = recherche {
        if !search.is_empty() {
            conditions
                .push("(p.Nom LIKE ? OR p.Prenom LIKE ? OR p.Email_Prive LIKE ?)".to_string());
            let like = format!("%{}%", search);
            let like2 = like.clone();
            let like3 = like.clone();
            params.push(Box::new(like));
            params.push(Box::new(like2));
            params.push(Box::new(like3));
        }
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    sql.push_str(" ORDER BY p.Nom ASC, p.Prenom ASC");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), map_personne)
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[tauri::command]
pub fn get_personne(app: AppHandle, id: i64) -> Result<Personne, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "personnes.read")?;
    conn.query_row(
        "SELECT * FROM T_Personnes WHERE ID_Personne = ?",
        rusqlite::params![id],
        map_personne,
    )
    .map_err(|e| format!("Personne introuvable: {}", e))
}

#[tauri::command]
pub fn sauvegarder_personne(app: AppHandle, personne: PersonneInput) -> Result<Personne, String> {
    let mut personne = personne;
    personne.nom = normalize_person_last_name(personne.nom);

    let conn = db::get_conn(&app)?;
    auth::require_permission(
        &conn,
        if personne.id_personne.is_some() {
            "personnes.update"
        } else {
            "personnes.create"
        },
    )?;

    if let Some(id) = personne.id_personne {
        ensure_resource_not_locked_by_other(&conn, "personnes", id)?;
        conn.execute(
            "UPDATE T_Personnes SET
                Civilite = ?, Nom = ?, Prenom = ?, Email_Prive = ?,
                Telephone_Prive = ?, Adresse_Privee = ?, Code_Postal_Prive = ?,
                Commune_Privee = ?, Pays = ?, Consentement_RGPD = ?,
                Date_Consentement = ?, Statut_Compte = ?, Notes_Commentaires = ?,
                Updated_At = datetime('now')
             WHERE ID_Personne = ? AND COALESCE(Updated_At, '') = COALESCE(?, '')",
            rusqlite::params![
                personne.civilite,
                personne.nom,
                personne.prenom,
                personne.email_prive,
                personne.telephone_prive,
                personne.adresse_privee,
                personne.code_postal_prive,
                personne.commune_privee,
                personne.pays,
                personne.consentement_rgpd,
                personne.date_consentement,
                personne.statut_compte,
                personne.notes_commentaires,
                id,
                personne.original_updated_at
            ],
        )
        .map_err(|e| e.to_string())?;
        if conn.changes() == 0 {
            return Err("Conflit: ce contact a été modifié ailleurs. Rechargez la fiche avant de réessayer.".into());
        }
        get_personne(app, id)
    } else {
        conn.execute(
            "INSERT INTO T_Personnes
                (Civilite, Nom, Prenom, Email_Prive, Telephone_Prive,
                 Adresse_Privee, Code_Postal_Prive, Commune_Privee, Pays,
                 Consentement_RGPD, Date_Consentement, Statut_Compte,
                 Notes_Commentaires, Date_Creation, Updated_At)
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?, date('now'), datetime('now'))",
            rusqlite::params![
                personne.civilite,
                personne.nom,
                personne.prenom,
                personne.email_prive,
                personne.telephone_prive,
                personne.adresse_privee,
                personne.code_postal_prive,
                personne.commune_privee,
                personne.pays,
                personne.consentement_rgpd,
                personne.date_consentement,
                personne.statut_compte,
                personne.notes_commentaires
            ],
        )
        .map_err(|e| e.to_string())?;
        let new_id = conn.last_insert_rowid();
        get_personne(app, new_id)
    }
}

#[tauri::command]
pub fn supprimer_personne(app: AppHandle, id: i64) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "personnes.delete")?;
    ensure_resource_not_locked_by_other(&conn, "personnes", id)?;
    conn.execute(
        "DELETE FROM T_Affiliations WHERE Ref_Personne = ?",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE T_Presences SET Ref_Personne = NULL WHERE Ref_Personne = ?",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM T_Personnes WHERE ID_Personne = ?",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
