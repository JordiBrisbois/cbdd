use super::*;

#[tauri::command]
pub fn lister_presences_reunion(
    app: AppHandle,
    reunion_id: i64,
) -> Result<Vec<PresenceAvecDetails>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "presences.read")?;
    let mut stmt = conn
        .prepare(
            "SELECT
                p.ID_Presence, p.Ref_Personne,
                CASE
                    WHEN pe.Statut_Compte = 'Anonymisé' OR (pe.Nom IS NULL AND pe.Prenom IS NULL)
                        THEN ?
                    ELSE pe.Nom
                END AS Nom_Affiche,
                CASE
                    WHEN pe.Statut_Compte = 'Anonymisé' OR (pe.Nom IS NULL AND pe.Prenom IS NULL)
                        THEN NULL
                    ELSE pe.Prenom
                END AS Prenom_Affiche,
                p.Statut_Presence, p.Souhaite_Rester_En_BDD,
                p.Notes_Commentaires
             FROM T_Presences p
             LEFT JOIN T_Personnes pe ON pe.ID_Personne = p.Ref_Personne
             WHERE p.Ref_Reunion = ?
             ORDER BY pe.Nom ASC, pe.Prenom ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![ANONYMIZED_LABEL, reunion_id], |row| {
            Ok(PresenceAvecDetails {
                id_presence: row.get(0)?,
                ref_personne: row.get(1)?,
                nom_personne: row.get(2)?,
                prenom_personne: row.get(3)?,
                statut_presence: row.get(4)?,
                souhaite_rester_en_bdd: row.get(5)?,
                notes_commentaires: row.get(6)?,
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
pub fn sauvegarder_presence(app: AppHandle, presence: PresenceInput) -> Result<Presence, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(
        &conn,
        if presence.id_presence.is_some() {
            "presences.update"
        } else {
            "presences.create"
        },
    )?;

    ensure_presence_context_mutable(&conn, presence.id_presence, presence.ref_reunion)?;

    if let Some(pid) = presence.id_presence {
        conn.execute(
            "UPDATE T_Presences SET
                Ref_Reunion = ?, Ref_Personne = ?, Statut_Presence = ?,
                Souhaite_Rester_En_BDD = ?, Notes_Commentaires = ?
             WHERE ID_Presence = ?",
            rusqlite::params![
                presence.ref_reunion,
                presence.ref_personne,
                presence.statut_presence,
                presence.souhaite_rester_en_bdd,
                presence.notes_commentaires,
                pid
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(Presence {
            id_presence: pid,
            ref_reunion: presence.ref_reunion,
            ref_personne: presence.ref_personne,
            statut_presence: presence.statut_presence,
            souhaite_rester_en_bdd: presence.souhaite_rester_en_bdd,
            notes_commentaires: presence.notes_commentaires,
        })
    } else {
        conn.execute(
            "INSERT INTO T_Presences
                (Ref_Reunion, Ref_Personne, Statut_Presence,
                 Souhaite_Rester_En_BDD, Notes_Commentaires)
             VALUES (?,?,?,?,?)",
            rusqlite::params![
                presence.ref_reunion,
                presence.ref_personne,
                presence.statut_presence,
                presence.souhaite_rester_en_bdd,
                presence.notes_commentaires
            ],
        )
        .map_err(|e| e.to_string())?;
        let new_id = conn.last_insert_rowid();
        Ok(Presence {
            id_presence: new_id,
            ref_reunion: presence.ref_reunion,
            ref_personne: presence.ref_personne,
            statut_presence: presence.statut_presence,
            souhaite_rester_en_bdd: presence.souhaite_rester_en_bdd,
            notes_commentaires: presence.notes_commentaires,
        })
    }
}

#[tauri::command]
pub fn supprimer_presence(app: AppHandle, id: i64) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "presences.delete")?;
    ensure_presence_context_mutable(&conn, Some(id), None)?;
    conn.execute(
        "DELETE FROM T_Presences WHERE ID_Presence = ?",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
