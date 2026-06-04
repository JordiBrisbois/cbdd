use super::*;

#[tauri::command]
pub fn lister_affiliations_personne(
    app: AppHandle,
    personne_id: i64,
) -> Result<Vec<AffiliationAvecDetails>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "affiliations.read")?;
    let mut stmt = conn
        .prepare(
            "SELECT
                a.ID_Affiliation,
                a.Ref_Personne, p.Nom, p.Prenom,
                a.Ref_Structure, s.Nom_Structure,
                a.Ref_Fonction, f.Libelle_Fonction,
                a.ID_Categorie, c.Nom_Categorie,
                a.Titre_Specifique, a.Email_Professionnel,
                a.Telephone_Direct, a.Gsm_Professionnel, a.Updated_At
             FROM T_Affiliations a
             LEFT JOIN T_Personnes p ON p.ID_Personne = a.Ref_Personne
             LEFT JOIN T_Structures s ON s.ID_Structure = a.Ref_Structure
             LEFT JOIN T_Fonctions f ON f.ID_Fonction = a.Ref_Fonction
             LEFT JOIN T_Categories c ON c.ID_Categorie = a.ID_Categorie
             WHERE a.Ref_Personne = ?
             ORDER BY s.Nom_Structure ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![personne_id], |row| {
            Ok(AffiliationAvecDetails {
                id_affiliation: row.get(0)?,
                ref_personne: row.get(1)?,
                nom_personne: row.get(2)?,
                prenom_personne: row.get(3)?,
                ref_structure: row.get(4)?,
                nom_structure: row.get(5)?,
                ref_fonction: row.get(6)?,
                libelle_fonction: row.get(7)?,
                id_categorie: row.get(8)?,
                nom_categorie: row.get(9)?,
                titre_specifique: row.get(10)?,
                email_professionnel: row.get(11)?,
                telephone_direct: row.get(12)?,
                gsm_professionnel: row.get(13)?,
                updated_at: row.get(14)?,
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
pub fn lister_affiliations_structure(
    app: AppHandle,
    structure_id: i64,
) -> Result<Vec<AffiliationAvecDetails>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "affiliations.read")?;
    let mut stmt = conn
        .prepare(
            "SELECT
                a.ID_Affiliation,
                a.Ref_Personne, p.Nom, p.Prenom,
                a.Ref_Structure, s.Nom_Structure,
                a.Ref_Fonction, f.Libelle_Fonction,
                a.ID_Categorie, c.Nom_Categorie,
                a.Titre_Specifique, a.Email_Professionnel,
                a.Telephone_Direct, a.Gsm_Professionnel, a.Updated_At
             FROM T_Affiliations a
             LEFT JOIN T_Personnes p ON p.ID_Personne = a.Ref_Personne
             LEFT JOIN T_Structures s ON s.ID_Structure = a.Ref_Structure
             LEFT JOIN T_Fonctions f ON f.ID_Fonction = a.Ref_Fonction
             LEFT JOIN T_Categories c ON c.ID_Categorie = a.ID_Categorie
             WHERE a.Ref_Structure = ? AND a.Ref_Personne IS NOT NULL
             ORDER BY p.Nom ASC, p.Prenom ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![structure_id], |row| {
            Ok(AffiliationAvecDetails {
                id_affiliation: row.get(0)?,
                ref_personne: row.get(1)?,
                nom_personne: row.get(2)?,
                prenom_personne: row.get(3)?,
                ref_structure: row.get(4)?,
                nom_structure: row.get(5)?,
                ref_fonction: row.get(6)?,
                libelle_fonction: row.get(7)?,
                id_categorie: row.get(8)?,
                nom_categorie: row.get(9)?,
                titre_specifique: row.get(10)?,
                email_professionnel: row.get(11)?,
                telephone_direct: row.get(12)?,
                gsm_professionnel: row.get(13)?,
                updated_at: row.get(14)?,
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
pub fn get_affiliation(app: AppHandle, id: i64) -> Result<AffiliationAvecDetails, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "affiliations.read")?;
    conn.query_row(
        "SELECT
            a.ID_Affiliation,
            a.Ref_Personne, p.Nom, p.Prenom,
            a.Ref_Structure, s.Nom_Structure,
            a.Ref_Fonction, f.Libelle_Fonction,
            a.ID_Categorie, c.Nom_Categorie,
            a.Titre_Specifique, a.Email_Professionnel,
            a.Telephone_Direct, a.Gsm_Professionnel, a.Updated_At
         FROM T_Affiliations a
         LEFT JOIN T_Personnes p ON p.ID_Personne = a.Ref_Personne
         LEFT JOIN T_Structures s ON s.ID_Structure = a.Ref_Structure
         LEFT JOIN T_Fonctions f ON f.ID_Fonction = a.Ref_Fonction
         LEFT JOIN T_Categories c ON c.ID_Categorie = a.ID_Categorie
         WHERE a.ID_Affiliation = ?",
        rusqlite::params![id],
        |row| {
            Ok(AffiliationAvecDetails {
                id_affiliation: row.get(0)?,
                ref_personne: row.get(1)?,
                nom_personne: row.get(2)?,
                prenom_personne: row.get(3)?,
                ref_structure: row.get(4)?,
                nom_structure: row.get(5)?,
                ref_fonction: row.get(6)?,
                libelle_fonction: row.get(7)?,
                id_categorie: row.get(8)?,
                nom_categorie: row.get(9)?,
                titre_specifique: row.get(10)?,
                email_professionnel: row.get(11)?,
                telephone_direct: row.get(12)?,
                gsm_professionnel: row.get(13)?,
                updated_at: row.get(14)?,
            })
        },
    )
    .map_err(|e| format!("Affiliation introuvable: {}", e))
}

#[tauri::command]
pub fn sauvegarder_affiliation(
    app: AppHandle,
    aff: AffiliationInput,
) -> Result<Affiliation, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(
        &conn,
        if aff.id_affiliation.is_some() {
            "affiliations.update"
        } else {
            "affiliations.create"
        },
    )?;

    ensure_affiliation_context_mutable(
        &conn,
        aff.id_affiliation,
        aff.ref_personne,
        aff.ref_structure,
    )?;

    if let Some(aid) = aff.id_affiliation {
        conn.execute(
            "UPDATE T_Affiliations SET
                Ref_Personne = ?, Ref_Structure = ?, Ref_Fonction = ?,
                Titre_Specifique = ?, Service_Specifique = ?,
                Email_Professionnel = ?, Telephone_Direct = ?,
                Gsm_Professionnel = ?, Date_Debut = ?, Date_Fin = ?,
                Notes_Commentaires = ?, ID_Categorie = ?, Updated_At = datetime('now')
             WHERE ID_Affiliation = ? AND COALESCE(Updated_At, '') = COALESCE(?, '')",
            rusqlite::params![
                aff.ref_personne,
                aff.ref_structure,
                aff.ref_fonction,
                aff.titre_specifique,
                aff.service_specifique,
                aff.email_professionnel,
                aff.telephone_direct,
                aff.gsm_professionnel,
                aff.date_debut,
                aff.date_fin,
                aff.notes_commentaires,
                aff.id_categorie,
                aid,
                aff.original_updated_at
            ],
        )
        .map_err(|e| e.to_string())?;
        if conn.changes() == 0 {
            return Err("Conflit: cette affiliation a été modifiée ailleurs. Rechargez la fiche avant de réessayer.".into());
        }
        Ok(Affiliation {
            id_affiliation: aid,
            ref_personne: aff.ref_personne,
            ref_structure: aff.ref_structure,
            ref_fonction: aff.ref_fonction,
            titre_specifique: aff.titre_specifique,
            service_specifique: aff.service_specifique,
            email_professionnel: aff.email_professionnel,
            telephone_direct: aff.telephone_direct,
            gsm_professionnel: aff.gsm_professionnel,
            date_debut: aff.date_debut,
            date_fin: aff.date_fin,
            notes_commentaires: aff.notes_commentaires,
            id_categorie: aff.id_categorie,
            updated_at: conn
                .query_row(
                    "SELECT Updated_At FROM T_Affiliations WHERE ID_Affiliation = ?",
                    rusqlite::params![aid],
                    |row| row.get(0),
                )
                .ok(),
        })
    } else {
        conn.execute(
            "INSERT INTO T_Affiliations
                (Ref_Personne, Ref_Structure, Ref_Fonction, Titre_Specifique,
                 Service_Specifique, Email_Professionnel, Telephone_Direct,
                 Gsm_Professionnel, Date_Debut, Date_Fin, Notes_Commentaires,
                 ID_Categorie, Updated_At)
             VALUES (?,?,?,?,?,?,?,?,?,?,?, ?, datetime('now'))",
            rusqlite::params![
                aff.ref_personne,
                aff.ref_structure,
                aff.ref_fonction,
                aff.titre_specifique,
                aff.service_specifique,
                aff.email_professionnel,
                aff.telephone_direct,
                aff.gsm_professionnel,
                aff.date_debut,
                aff.date_fin,
                aff.notes_commentaires,
                aff.id_categorie
            ],
        )
        .map_err(|e| e.to_string())?;
        let new_id = conn.last_insert_rowid();
        Ok(Affiliation {
            id_affiliation: new_id,
            ref_personne: aff.ref_personne,
            ref_structure: aff.ref_structure,
            ref_fonction: aff.ref_fonction,
            titre_specifique: aff.titre_specifique,
            service_specifique: aff.service_specifique,
            email_professionnel: aff.email_professionnel,
            telephone_direct: aff.telephone_direct,
            gsm_professionnel: aff.gsm_professionnel,
            date_debut: aff.date_debut,
            date_fin: aff.date_fin,
            notes_commentaires: aff.notes_commentaires,
            id_categorie: aff.id_categorie,
            updated_at: conn
                .query_row(
                    "SELECT Updated_At FROM T_Affiliations WHERE ID_Affiliation = ?",
                    rusqlite::params![new_id],
                    |row| row.get(0),
                )
                .ok(),
        })
    }
}

#[tauri::command]
pub fn supprimer_affiliation(app: AppHandle, id: i64) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "affiliations.delete")?;
    ensure_affiliation_context_mutable(&conn, Some(id), None, None)?;
    conn.execute(
        "DELETE FROM T_Affiliations WHERE ID_Affiliation = ?",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
