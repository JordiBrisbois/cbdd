use super::*;

#[tauri::command]
pub fn get_personne_rgpd(app: AppHandle, id: i64) -> Result<Personne, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "rgpd.read")?;
    conn.query_row(
        "SELECT * FROM T_Personnes WHERE ID_Personne = ?",
        rusqlite::params![id],
        map_personne,
    )
    .map_err(|e| format!("Personne introuvable: {}", e))
}

#[tauri::command]
pub fn get_personnes_rgpd(app: AppHandle) -> Result<Vec<Personne>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "rgpd.read")?;
    let predicate = rgpd_attention_predicate("p");
    let mut stmt = conn
        .prepare(&format!(
            "SELECT p.* FROM T_Personnes p
             WHERE {}
             ORDER BY p.Nom ASC, p.Prenom ASC",
            predicate
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_personne)
        .map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[tauri::command]
pub fn maj_statut_rgpd(app: AppHandle, personne_id: i64, statut: String) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "rgpd.update")?;
    ensure_resource_not_locked_by_other(&conn, "personnes", personne_id)?;
    conn.execute(
        "UPDATE T_Personnes SET Statut_Compte = ?, Updated_At = datetime('now') WHERE ID_Personne = ?",
        rusqlite::params![statut, personne_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn anonymiser_personne(app: AppHandle, personne_id: i64) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "rgpd.anonymize")?;
    anonymize_person(&conn, personne_id)?;
    Ok(())
}

#[tauri::command]
pub fn lister_personnes_refus_bdd_presence(
    app: AppHandle,
) -> Result<Vec<PersonneRefusBDDPresence>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "rgpd.read")?;

    let mut sql = String::from(
        "SELECT DISTINCT p.ID_Personne, p.Nom, p.Prenom, p.Email_Prive, p.Telephone_Prive, \
         p.Consentement_RGPD, p.Statut_Compte, r.Titre_Reunion, r.Date_Reunion",
    );
    sql.push_str(" FROM T_Personnes p");
    sql.push_str(" INNER JOIN T_Presences pr ON pr.Ref_Personne = p.ID_Personne");
    sql.push_str(" INNER JOIN T_Reunions r ON r.ID_Reunion = pr.Ref_Reunion");
    sql.push_str(
        " WHERE pr.Souhaite_Rester_En_BDD = 0 AND COALESCE(p.Statut_Compte, '') != 'Anonymisé'",
    );
    sql.push_str(" ORDER BY p.Nom ASC, p.Prenom ASC");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(PersonneRefusBDDPresence {
                id_personne: row.get(0)?,
                nom: row.get(1)?,
                prenom: row.get(2)?,
                email_prive: row.get(3)?,
                telephone_prive: row.get(4)?,
                consentement_rgpd: row.get(5)?,
                statut_compte: row.get(6)?,
                reunion_titre: row.get(7)?,
                reunion_date: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[tauri::command]
pub fn anonymiser_personnes_en_masse(
    app: AppHandle,
    personne_ids: Vec<i64>,
) -> Result<i64, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "rgpd.anonymize.bulk")?;
    let mut count = 0i64;
    for id in personne_ids {
        if anonymize_person(&conn, id).is_ok() {
            count += 1;
        }
    }
    Ok(count)
}

#[tauri::command]
pub fn lister_personnes_categorie_detaillee(
    app: AppHandle,
    categorie_id: Option<i64>,
    recherche: Option<String>,
) -> Result<Vec<PersonneCategorieDetaillee>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "personnes.read")?;

    let mut sql = String::from(
        "SELECT DISTINCT p.ID_Personne, p.Civilite, p.Nom, p.Prenom, p.Email_Prive, \
         p.Telephone_Prive, p.Adresse_Privee, p.Code_Postal_Prive, p.Commune_Privee, \
         p.Pays, p.Consentement_RGPD, p.Date_Consentement, p.Statut_Compte, \
         p.Notes_Commentaires, p.Date_Creation, \
         GROUP_CONCAT(DISTINCT s.Nom_Structure) AS structures, \
         GROUP_CONCAT(DISTINCT f.Libelle_Fonction) AS fonctions, \
         GROUP_CONCAT(DISTINCT c.Nom_Categorie) AS categories, \
         GROUP_CONCAT(DISTINCT a.Email_Professionnel) AS emails_pro, \
         GROUP_CONCAT(DISTINCT a.Titre_Specifique) AS titres, \
         GROUP_CONCAT(DISTINCT a.Telephone_Direct) AS tels_directs, \
         GROUP_CONCAT(DISTINCT a.Gsm_Professionnel) AS gsms_pro, \
         GROUP_CONCAT(DISTINCT a.Date_Debut) AS dates_debut, \
         GROUP_CONCAT(DISTINCT a.Date_Fin) AS dates_fin, \
         'personne' AS type_entree",
    );
    sql.push_str(" FROM T_Personnes p");
    sql.push_str(" LEFT JOIN T_Affiliations a ON a.Ref_Personne = p.ID_Personne");
    sql.push_str(" LEFT JOIN T_Structures s ON s.ID_Structure = a.Ref_Structure");
    sql.push_str(" LEFT JOIN T_Fonctions f ON f.ID_Fonction = a.Ref_Fonction");
    sql.push_str(" LEFT JOIN T_Categories c ON c.ID_Categorie = a.ID_Categorie");

    let mut conditions: Vec<String> =
        vec!["COALESCE(p.Statut_Compte, '') != 'Anonymisé'".to_string()];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(cat_id) = categorie_id {
        conditions.push("a.ID_Categorie = ?".to_string());
        params.push(Box::new(cat_id));
    }

    if let Some(ref search) = recherche {
        if !search.is_empty() {
            conditions
                .push("(p.Nom LIKE ? OR p.Prenom LIKE ? OR p.Email_Prive LIKE ?)".to_string());
            let like = format!("%{}%", search);
            params.push(Box::new(like.clone()));
            params.push(Box::new(like.clone()));
            params.push(Box::new(like));
        }
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    sql.push_str(" GROUP BY p.ID_Personne");

    sql.push_str(
        " UNION ALL SELECT \
         -s.ID_Structure, NULL, NULL, NULL, \
         s.Email_General, s.Telephone_General, \
         s.Adresse_Structure, s.Code_Postal_Structure, s.Commune_Structure, s.Pays, \
         0, NULL, 'Structure', s.Notes_Commentaires, NULL, \
         s.Nom_Structure, \
         c.Nom_Categorie, \
         c.Nom_Categorie, \
         NULL, s.Service_Specifique, NULL, NULL, NULL, NULL, \
         'structure'",
    );
    sql.push_str(" FROM T_Structures s");
    sql.push_str(" INNER JOIN T_Categories c ON c.ID_Categorie = s.ID_Categorie");
    sql.push_str(" WHERE COALESCE(s.Nom_Structure, '') != ''");
    sql.push_str(" AND NOT EXISTS (SELECT 1 FROM T_Affiliations a WHERE a.Ref_Structure = s.ID_Structure AND a.Ref_Personne IS NOT NULL)");

    let mut struct_conditions: Vec<String> = Vec::new();

    if let Some(cat_id) = categorie_id {
        struct_conditions.push("s.ID_Categorie = ?".to_string());
        params.push(Box::new(cat_id));
    }

    if let Some(ref search) = recherche {
        if !search.is_empty() {
            struct_conditions.push("s.Nom_Structure LIKE ?".to_string());
            let like = format!("%{}%", search);
            params.push(Box::new(like));
        }
    }

    if !struct_conditions.is_empty() {
        sql.push_str(" AND ");
        sql.push_str(&struct_conditions.join(" AND "));
    }

    sql.push_str(" ORDER BY nom ASC, prenom ASC");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(PersonneCategorieDetaillee {
                id_personne: row.get(0)?,
                civilite: row.get(1)?,
                nom: row.get(2)?,
                prenom: row.get(3)?,
                email_prive: row.get(4)?,
                telephone_prive: row.get(5)?,
                adresse_privee: row.get(6)?,
                code_postal_prive: row.get(7)?,
                commune_privee: row.get(8)?,
                pays: row.get(9)?,
                consentement_rgpd: row.get(10)?,
                date_consentement: row.get(11)?,
                statut_compte: row.get(12)?,
                notes_commentaires: row.get(13)?,
                date_creation: row.get(14)?,
                structures: row.get(15)?,
                fonctions: row.get(16)?,
                categories: row.get(17)?,
                emails_pro: row.get(18)?,
                titres: row.get(19)?,
                tels_directs: row.get(20)?,
                gsms_pro: row.get(21)?,
                dates_debut: row.get(22)?,
                dates_fin: row.get(23)?,
                type_entree: row.get(24)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}
