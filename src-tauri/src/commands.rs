use crate::auth;
use crate::backups;
use crate::db;
use crate::excel_export;
use crate::models::*;
use chrono::Utc;
use rusqlite::OptionalExtension;
use tauri::AppHandle;

mod presets;
mod query_builder;

const ANONYMIZED_STATUS: &str = "Anonymisé";
const ANONYMIZED_LABEL: &str = "Participant anonymisé";
const EDIT_LOCK_MINUTES: i64 = 10;

fn machine_label() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "Poste inconnu".to_string())
}

fn normalize_person_last_name(value: Option<String>) -> Option<String> {
    value
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
        .map(|raw| raw.to_uppercase())
}

fn rgpd_attention_predicate(alias: &str) -> String {
    format!(
        "COALESCE({alias}.Statut_Compte, '') != 'Anonymisé'
         AND (
            {alias}.Consentement_RGPD = 0
            OR {alias}.Statut_Compte = 'A supprimer'
            OR EXISTS (
                SELECT 1
                FROM T_Presences pr
                WHERE pr.Ref_Personne = {alias}.ID_Personne
                  AND pr.Souhaite_Rester_En_BDD = 0
            )
         )"
    )
}

fn permission_for_resource(resource_type: &str) -> Option<&'static str> {
    match resource_type {
        "personnes" => Some("personnes.update"),
        "structures" => Some("structures.update"),
        "affiliations" => Some("affiliations.update"),
        "reunions" => Some("reunions.update"),
        _ => None,
    }
}

fn ensure_resource_not_locked_by_other(
    conn: &rusqlite::Connection,
    resource_type: &str,
    resource_id: i64,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM T_EditLocks WHERE Expires_At <= datetime('now')",
        [],
    )
    .map_err(|e| e.to_string())?;

    let (user_id, _) = auth::current_actor_label(conn)?;
    let existing = conn
        .query_row(
            "SELECT Holder_User_Id, Holder_Label
             FROM T_EditLocks
             WHERE Resource_Type = ? AND Resource_Id = ?",
            rusqlite::params![resource_type, resource_id],
            |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?,
                    row.get::<_, String>(1)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if let Some((existing_user_id, existing_label)) = existing {
        if existing_user_id != user_id {
            return Err(format!(
                "Cette fiche est actuellement verrouillée par {}. Fermez ou laissez expirer l'édition avant de poursuivre.",
                existing_label
            ));
        }
    }

    Ok(())
}

fn ensure_affiliation_context_mutable(
    conn: &rusqlite::Connection,
    affiliation_id: Option<i64>,
    ref_personne: Option<i64>,
    ref_structure: Option<i64>,
) -> Result<(), String> {
    if let Some(id) = affiliation_id {
        ensure_resource_not_locked_by_other(conn, "affiliations", id)?;
        let existing = conn
            .query_row(
                "SELECT Ref_Personne, Ref_Structure FROM T_Affiliations WHERE ID_Affiliation = ?",
                rusqlite::params![id],
                |row| Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        if let Some((existing_personne, existing_structure)) = existing {
            if let Some(personne_id) = existing_personne {
                ensure_resource_not_locked_by_other(conn, "personnes", personne_id)?;
            }
            if let Some(structure_id) = existing_structure {
                ensure_resource_not_locked_by_other(conn, "structures", structure_id)?;
            }
        }
    }

    if let Some(personne_id) = ref_personne {
        ensure_resource_not_locked_by_other(conn, "personnes", personne_id)?;
    }
    if let Some(structure_id) = ref_structure {
        ensure_resource_not_locked_by_other(conn, "structures", structure_id)?;
    }

    Ok(())
}

fn ensure_presence_context_mutable(
    conn: &rusqlite::Connection,
    presence_id: Option<i64>,
    reunion_id: Option<i64>,
) -> Result<(), String> {
    if let Some(id) = presence_id {
        let existing_reunion = conn
            .query_row(
                "SELECT Ref_Reunion FROM T_Presences WHERE ID_Presence = ?",
                rusqlite::params![id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .flatten();

        if let Some(existing_reunion_id) = existing_reunion {
            ensure_resource_not_locked_by_other(conn, "reunions", existing_reunion_id)?;
        }
    }

    if let Some(target_reunion_id) = reunion_id {
        ensure_resource_not_locked_by_other(conn, "reunions", target_reunion_id)?;
    }

    Ok(())
}

#[tauri::command]
pub fn get_db_status(app: AppHandle) -> Result<serde_json::Value, String> {
    let (connected, path, error) = db::status(&app);
    Ok(serde_json::json!({ "connected": connected, "path": path, "error": error }))
}

#[tauri::command]
pub fn pick_db(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let file = app
        .dialog()
        .file()
        .add_filter("Base de données", &["sqlite", "db", "sqlite3"])
        .blocking_pick_file();

    match file {
        Some(f) => {
            let path = f.to_string();
            db::connect(&path)?;
            db::save_path(&app, &path);
            println!("[CRVI-GRC] DB connectée: {}", path);
            Ok(Some(path))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub fn reconnect_db(app: AppHandle) -> Result<(), String> {
    db::get_conn(&app)?;
    Ok(())
}

// ──────────────────────────────────────────────
// AUTH / SÉCURITÉ
// ──────────────────────────────────────────────

#[tauri::command]
pub fn get_current_session(app: AppHandle) -> Result<CurrentSession, String> {
    let conn = db::get_conn(&app)?;
    auth::get_current_session(&conn)
}

#[tauri::command]
pub fn login(app: AppHandle, credentials: LoginInput) -> Result<CurrentSession, String> {
    let conn = db::get_conn(&app)?;
    auth::login(
        &conn,
        credentials.username.trim(),
        credentials.password.trim(),
    )
}

#[tauri::command]
pub fn logout() -> Result<(), String> {
    auth::logout()
}

#[tauri::command]
pub fn lister_permissions(app: AppHandle) -> Result<Vec<Permission>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.roles")?;
    Ok(auth::list_permissions())
}

#[tauri::command]
pub fn lister_roles(app: AppHandle) -> Result<Vec<RoleDetails>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.roles")?;
    auth::list_roles(&conn)
}

#[tauri::command]
pub fn sauvegarder_role(app: AppHandle, role: RoleInput) -> Result<RoleDetails, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.roles")?;
    auth::save_role(&conn, role)
}

#[tauri::command]
pub fn supprimer_role(app: AppHandle, role_id: i64) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.roles")?;
    auth::delete_role(&conn, role_id)
}

#[tauri::command]
pub fn lister_users(app: AppHandle) -> Result<Vec<UserSummary>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.users")?;
    auth::list_users(&conn)
}

#[tauri::command]
pub fn sauvegarder_user(app: AppHandle, user: UserInput) -> Result<UserSummary, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.users")?;
    auth::save_user(&conn, user)
}

#[tauri::command]
pub fn supprimer_user(app: AppHandle, user_id: i64) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.users")?;
    auth::delete_user(&conn, user_id)
}

#[tauri::command]
pub fn changer_mot_de_passe_user(
    app: AppHandle,
    payload: PasswordChangeInput,
) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.users")?;
    auth::admin_set_password(&conn, payload.user_id, payload.new_password.trim())
}

#[tauri::command]
pub fn changer_mon_mot_de_passe(
    app: AppHandle,
    payload: OwnPasswordChangeInput,
) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::change_own_password(
        &conn,
        payload.current_password.trim(),
        payload.new_password.trim(),
    )
}

#[tauri::command]
pub fn get_security_settings(app: AppHandle) -> Result<SecuritySettings, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.settings")?;
    auth::get_security_settings(&conn)
}

#[tauri::command]
pub fn sauvegarder_security_settings(
    app: AppHandle,
    settings: SecuritySettings,
) -> Result<SecuritySettings, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.settings")?;
    auth::save_security_settings(&conn, settings)
}

#[tauri::command]
pub fn acquire_edit_lock(
    app: AppHandle,
    resource_type: String,
    resource_id: i64,
) -> Result<EditLockStatus, String> {
    let conn = db::get_conn(&app)?;
    let permission = permission_for_resource(&resource_type)
        .ok_or_else(|| "Type de ressource de verrou inconnu".to_string())?;
    auth::require_permission(&conn, permission)?;

    conn.execute(
        "DELETE FROM T_EditLocks WHERE Expires_At <= datetime('now')",
        [],
    )
    .map_err(|e| e.to_string())?;

    let (user_id, holder_label) = auth::current_actor_label(&conn)?;
    let machine = machine_label();

    let existing = conn
        .query_row(
            "SELECT Holder_User_Id, Holder_Label, Expires_At
             FROM T_EditLocks
             WHERE Resource_Type = ? AND Resource_Id = ?",
            rusqlite::params![resource_type, resource_id],
            |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if let Some((existing_user_id, existing_label, expires_at)) = existing {
        if existing_user_id == user_id {
            conn.execute(
                "UPDATE T_EditLocks
                 SET Holder_Label = ?, Machine_Label = ?, Acquired_At = datetime('now'),
                     Expires_At = datetime('now', ?)
                 WHERE Resource_Type = ? AND Resource_Id = ?",
                rusqlite::params![
                    holder_label,
                    machine,
                    format!("+{} minutes", EDIT_LOCK_MINUTES),
                    resource_type,
                    resource_id
                ],
            )
            .map_err(|e| e.to_string())?;
            return Ok(EditLockStatus {
                resource_type,
                resource_id,
                acquired: true,
                holder_label: Some(existing_label),
                expires_at: Some(expires_at),
            });
        }

        return Ok(EditLockStatus {
            resource_type,
            resource_id,
            acquired: false,
            holder_label: Some(existing_label),
            expires_at: Some(expires_at),
        });
    }

    let expires_at = conn
        .query_row(
            "SELECT datetime('now', ?)",
            rusqlite::params![format!("+{} minutes", EDIT_LOCK_MINUTES)],
            |row| row.get::<_, String>(0),
        )
        .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO T_EditLocks (Resource_Type, Resource_Id, Holder_User_Id, Holder_Label, Machine_Label, Expires_At)
         VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params![resource_type, resource_id, user_id, holder_label, machine, expires_at],
    )
    .map_err(|e| e.to_string())?;

    Ok(EditLockStatus {
        resource_type,
        resource_id,
        acquired: true,
        holder_label: None,
        expires_at: Some(expires_at),
    })
}

#[tauri::command]
pub fn release_edit_lock(
    app: AppHandle,
    resource_type: String,
    resource_id: i64,
) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    let permission = permission_for_resource(&resource_type)
        .ok_or_else(|| "Type de ressource de verrou inconnu".to_string())?;
    auth::require_permission(&conn, permission)?;

    let (user_id, _) = auth::current_actor_label(&conn)?;
    conn.execute(
        "DELETE FROM T_EditLocks
         WHERE Resource_Type = ? AND Resource_Id = ? AND Holder_User_Id IS ?",
        rusqlite::params![resource_type, resource_id, user_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn run_auto_backup_check(app: AppHandle) -> Result<BackupRunResult, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.backups")?;
    let (_, holder_label) = auth::current_actor_label(&conn)?;
    backups::maybe_run_auto_backup(&app, &conn, &holder_label, &machine_label())
}

#[tauri::command]
pub fn create_manual_backup(
    app: AppHandle,
    request: ManualBackupRequest,
) -> Result<BackupRunResult, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.backups")?;
    backups::create_backup_now(&app, &conn, request)
}

#[tauri::command]
pub fn list_local_backups(app: AppHandle) -> Result<Vec<BackupInfo>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.backups")?;
    drop(conn);
    backups::list_local_backups(&app)
}

#[tauri::command]
pub fn get_backup_directory(app: AppHandle) -> Result<String, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.backups")?;
    drop(conn);
    backups::get_backup_directory(&app)
}

#[tauri::command]
pub fn open_backup_directory(app: AppHandle) -> Result<String, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.backups")?;
    drop(conn);
    backups::open_backup_directory(&app)
}

#[tauri::command]
pub fn restore_local_backup(
    app: AppHandle,
    request: RestoreBackupRequest,
) -> Result<BackupRunResult, String> {
    let conn = db::get_conn(&app)?;
    let session = auth::require_permission(&conn, "admin.backups")?;
    let preferred_admin_username = session.username.clone();
    drop(conn);
    backups::restore_backup(&app, preferred_admin_username.as_deref(), request)
}

#[tauri::command]
pub fn delete_local_backup(app: AppHandle, backup_path: String) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.backups")?;
    drop(conn);
    backups::delete_backup(&app, &backup_path)
}

#[tauri::command]
pub fn exporter_classeur_excel_admin(app: AppHandle) -> Result<Option<ExcelRebuildResult>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.exports")?;
    excel_export::export_reconstructed_excel(&app, &conn)
}

// ──────────────────────────────────────────────
// PERSONNES
// ──────────────────────────────────────────────

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

// ──────────────────────────────────────────────
// STRUCTURES
// ──────────────────────────────────────────────

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

// ──────────────────────────────────────────────
// CATEGORIES
// ──────────────────────────────────────────────

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

// ──────────────────────────────────────────────
// FONCTIONS
// ──────────────────────────────────────────────

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

// ──────────────────────────────────────────────
// AFFILIATIONS
// ──────────────────────────────────────────────

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

    ensure_affiliation_context_mutable(&conn, aff.id_affiliation, aff.ref_personne, aff.ref_structure)?;

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

// ──────────────────────────────────────────────
// REUNIONS
// ──────────────────────────────────────────────

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

// ──────────────────────────────────────────────
// PRESENCES
// ──────────────────────────────────────────────

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

// ──────────────────────────────────────────────
// STATISTIQUES
// ──────────────────────────────────────────────

#[tauri::command]
pub fn get_dashboard_stats(
    app: AppHandle,
    filters: Option<StatsFilters>,
) -> Result<DashboardStats, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "stats.read")?;
    let filters = filters.unwrap_or(StatsFilters {
        start_date: None,
        end_date: None,
    });

    let (available_start_date, available_end_date): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT MIN(Date_Reunion), MAX(Date_Reunion) FROM T_Reunions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    let total_contacts = query_i64(&conn, "SELECT COUNT(*) FROM T_Personnes", Vec::new())?;
    let active_contacts = query_i64(
        &conn,
        "SELECT COUNT(*) FROM T_Personnes WHERE COALESCE(Statut_Compte, '') != ?",
        vec![ANONYMIZED_STATUS.to_string()],
    )?;
    let anonymized_contacts = query_i64(
        &conn,
        "SELECT COUNT(*) FROM T_Personnes WHERE Statut_Compte = ?",
        vec![ANONYMIZED_STATUS.to_string()],
    )?;
    let contacts_to_delete = query_i64(
        &conn,
        &format!(
            "SELECT COUNT(*) FROM T_Personnes p WHERE {}",
            rgpd_attention_predicate("p")
        ),
        Vec::new(),
    )?;
    let total_structures = query_i64(&conn, "SELECT COUNT(*) FROM T_Structures", Vec::new())?;
    let total_categories = query_i64(&conn, "SELECT COUNT(*) FROM T_Categories", Vec::new())?;
    let total_affiliations =
        query_i64(&conn, "SELECT COUNT(*) FROM T_Affiliations", Vec::new())?;
    let total_reunions = query_i64(&conn, "SELECT COUNT(*) FROM T_Reunions", Vec::new())?;
    let partner_direct_structures = query_i64(
        &conn,
        "SELECT COUNT(*) FROM T_Structures WHERE Partenaire_Direct = 1",
        Vec::new(),
    )?;
    let rgpd_consent_rate = query_f64(
        &conn,
        "SELECT COALESCE(
            AVG(
                CASE
                    WHEN p.Consentement_RGPD = 1
                      AND COALESCE(p.Statut_Compte, '') != 'A supprimer'
                      AND NOT EXISTS (
                          SELECT 1
                          FROM T_Presences pr
                          WHERE pr.Ref_Personne = p.ID_Personne
                            AND pr.Souhaite_Rester_En_BDD = 0
                      )
                    THEN 100.0
                    ELSE 0.0
                END
            ),
            0
         )
         FROM T_Personnes p
         WHERE COALESCE(p.Statut_Compte, '') != ?",
        vec![ANONYMIZED_STATUS.to_string()],
    )?;

    let mut period_clauses = Vec::new();
    let mut period_params = Vec::new();
    append_date_filters(&mut period_clauses, &mut period_params, "r.Date_Reunion", &filters);
    let period_where = build_where_sql(&period_clauses);

    let period_reunions = query_i64(
        &conn,
        &format!("SELECT COUNT(*) FROM T_Reunions r{}", period_where),
        period_params.clone(),
    )?;
    let period_presences = query_i64(
        &conn,
        &format!(
            "SELECT COUNT(*)
             FROM T_Presences p
             INNER JOIN T_Reunions r ON r.ID_Reunion = p.Ref_Reunion{}",
            period_where
        ),
        period_params.clone(),
    )?;
    let average_presences_per_reunion = query_f64(
        &conn,
        &format!(
            "SELECT COALESCE(AVG(attendee_count), 0)
             FROM (
               SELECT COUNT(p.ID_Presence) AS attendee_count
               FROM T_Reunions r
               LEFT JOIN T_Presences p ON p.Ref_Reunion = r.ID_Reunion{}
               GROUP BY r.ID_Reunion
             )",
            period_where
        ),
        period_params.clone(),
    )?;

    let meetings_by_month = {
        let mut clauses = vec!["r.Date_Reunion IS NOT NULL".to_string()];
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, "r.Date_Reunion", &filters);
        load_stats_buckets(
            &conn,
            &format!(
                "SELECT substr(r.Date_Reunion, 1, 7), substr(r.Date_Reunion, 1, 7), COUNT(*)
                 FROM T_Reunions r{}
                 GROUP BY substr(r.Date_Reunion, 1, 7)
                 ORDER BY substr(r.Date_Reunion, 1, 7) ASC",
                build_where_sql(&clauses)
            ),
            params,
        )?
    };

    let contacts_created_by_month = {
        let creation_date_expr = sqlite_date_expr("p.Date_Creation");
        let creation_month_expr = sqlite_month_expr("p.Date_Creation");
        let mut clauses = vec![format!("{} IS NOT NULL", creation_date_expr)];
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, &creation_date_expr, &filters);
        load_stats_buckets(
            &conn,
            &format!(
                "SELECT {month_expr}, {month_expr}, COUNT(*)
                 FROM T_Personnes p{}
                 GROUP BY {month_expr}
                 ORDER BY {month_expr} ASC",
                build_where_sql(&clauses),
                month_expr = creation_month_expr,
            ),
            params,
        )?
    };

    let attendance_by_status = {
        let mut clauses = vec!["COALESCE(p.Statut_Presence, '') != ''".to_string()];
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, "r.Date_Reunion", &filters);
        load_stats_buckets(
            &conn,
            &format!(
                "SELECT p.Statut_Presence, p.Statut_Presence, COUNT(*)
                 FROM T_Presences p
                 INNER JOIN T_Reunions r ON r.ID_Reunion = p.Ref_Reunion{}
                 GROUP BY p.Statut_Presence
                 ORDER BY COUNT(*) DESC, p.Statut_Presence ASC",
                build_where_sql(&clauses)
            ),
            params,
        )?
    };

    let structures_by_category = load_stats_buckets(
        &conn,
        "SELECT
            COALESCE(CAST(s.ID_Categorie AS TEXT), 'none'),
            COALESCE(c.Nom_Categorie, 'Sans catégorie'),
            COUNT(*)
         FROM T_Structures s
         LEFT JOIN T_Categories c ON c.ID_Categorie = s.ID_Categorie
         GROUP BY COALESCE(c.Nom_Categorie, 'Sans catégorie'), COALESCE(CAST(s.ID_Categorie AS TEXT), 'none')
         ORDER BY COUNT(*) DESC, COALESCE(c.Nom_Categorie, 'Sans catégorie') ASC",
        Vec::new(),
    )?;

    let contacts_by_commune = merge_casefolded_buckets(load_stats_buckets(
        &conn,
        "SELECT
            COALESCE(NULLIF(TRIM(Commune_Privee), ''), 'Non renseignée'),
            COALESCE(NULLIF(TRIM(Commune_Privee), ''), 'Non renseignée'),
            COUNT(*)
         FROM T_Personnes
         WHERE COALESCE(Statut_Compte, '') != 'Anonymisé'
         GROUP BY COALESCE(NULLIF(TRIM(Commune_Privee), ''), 'Non renseignée')
         ORDER BY COUNT(*) DESC, COALESCE(NULLIF(TRIM(Commune_Privee), ''), 'Non renseignée') ASC
         LIMIT 8",
        Vec::new(),
    )?);

    let structures_by_commune = merge_casefolded_buckets(load_stats_buckets(
        &conn,
        "SELECT
            COALESCE(NULLIF(TRIM(Commune_Structure), ''), 'Non renseignée'),
            COALESCE(NULLIF(TRIM(Commune_Structure), ''), 'Non renseignée'),
            COUNT(*)
         FROM T_Structures
         GROUP BY COALESCE(NULLIF(TRIM(Commune_Structure), ''), 'Non renseignée')
         ORDER BY COUNT(*) DESC, COALESCE(NULLIF(TRIM(Commune_Structure), ''), 'Non renseignée') ASC
         LIMIT 8",
        Vec::new(),
    )?);

    let meetings_by_organisme = {
        let mut clauses = Vec::new();
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, "r.Date_Reunion", &filters);
        load_stats_buckets(
            &conn,
            &format!(
                "SELECT
                    COALESCE(CAST(r.Ref_Structure AS TEXT), 'none'),
                    COALESCE(s.Nom_Structure, 'Sans organisme'),
                    COUNT(*)
                 FROM T_Reunions r
                 LEFT JOIN T_Structures s ON s.ID_Structure = r.Ref_Structure{}
                 GROUP BY COALESCE(s.Nom_Structure, 'Sans organisme'), COALESCE(CAST(r.Ref_Structure AS TEXT), 'none')
                 ORDER BY COUNT(*) DESC, COALESCE(s.Nom_Structure, 'Sans organisme') ASC
                 LIMIT 8",
                build_where_sql(&clauses)
            ),
            params,
        )?
    };

    let account_statuses = load_stats_buckets(
        &conn,
        "SELECT
            COALESCE(NULLIF(TRIM(Statut_Compte), ''), 'Inconnu'),
            COALESCE(NULLIF(TRIM(Statut_Compte), ''), 'Inconnu'),
            COUNT(*)
         FROM T_Personnes
         GROUP BY COALESCE(NULLIF(TRIM(Statut_Compte), ''), 'Inconnu')
         ORDER BY COUNT(*) DESC, COALESCE(NULLIF(TRIM(Statut_Compte), ''), 'Inconnu') ASC",
        Vec::new(),
    )?;

    let quality_checks = vec![
        StatsBucket {
            key: "contacts_without_email".to_string(),
            label: "Contacts sans email".to_string(),
            value: query_i64(
                &conn,
                "SELECT COUNT(*) FROM T_Personnes
                 WHERE COALESCE(Statut_Compte, '') != 'Anonymisé'
                   AND COALESCE(NULLIF(TRIM(Email_Prive), ''), '') = ''",
                Vec::new(),
            )?,
        },
        StatsBucket {
            key: "contacts_without_phone".to_string(),
            label: "Contacts sans téléphone".to_string(),
            value: query_i64(
                &conn,
                "SELECT COUNT(*) FROM T_Personnes
                 WHERE COALESCE(Statut_Compte, '') != 'Anonymisé'
                   AND COALESCE(NULLIF(TRIM(Telephone_Prive), ''), '') = ''",
                Vec::new(),
            )?,
        },
        StatsBucket {
            key: "structures_without_category".to_string(),
            label: "Structures sans catégorie".to_string(),
            value: query_i64(
                &conn,
                "SELECT COUNT(*) FROM T_Structures WHERE ID_Categorie IS NULL",
                Vec::new(),
            )?,
        },
        StatsBucket {
            key: "structures_without_email".to_string(),
            label: "Structures sans email".to_string(),
            value: query_i64(
                &conn,
                "SELECT COUNT(*) FROM T_Structures
                 WHERE COALESCE(NULLIF(TRIM(Email_General), ''), '') = ''",
                Vec::new(),
            )?,
        },
        StatsBucket {
            key: "meetings_without_structure".to_string(),
            label: "Réunions sans organisme".to_string(),
            value: query_i64(
                &conn,
                "SELECT COUNT(*) FROM T_Reunions WHERE Ref_Structure IS NULL",
                Vec::new(),
            )?,
        },
    ];

    let top_meetings = {
        let mut clauses = Vec::new();
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, "r.Date_Reunion", &filters);
        load_top_meetings(
            &conn,
            &format!(
                "SELECT
                    COALESCE(r.Titre_Reunion, 'Réunion sans titre'),
                    COUNT(p.ID_Presence),
                    r.Date_Reunion,
                    COALESCE(s.Nom_Structure, 'Sans organisme')
                 FROM T_Reunions r
                 LEFT JOIN T_Presences p ON p.Ref_Reunion = r.ID_Reunion
                 LEFT JOIN T_Structures s ON s.ID_Structure = r.Ref_Structure{}
                 GROUP BY r.ID_Reunion, r.Titre_Reunion, r.Date_Reunion, s.Nom_Structure
                 ORDER BY COUNT(p.ID_Presence) DESC, r.Date_Reunion DESC
                 LIMIT 8",
                build_where_sql(&clauses)
            ),
            params,
        )?
    };

    let top_structures_presence_rate = {
        let mut clauses = Vec::new();
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, "r.Date_Reunion", &filters);
        let where_sql = build_where_sql(&clauses);
        load_stats_participation(
            &conn,
            &format!(
                "WITH presence_structure AS (
                    SELECT DISTINCT
                        p.ID_Presence AS presence_id,
                        s.ID_Structure AS structure_id,
                        COALESCE(s.Nom_Structure, 'Structure sans nom') AS structure_name,
                        COALESCE(LOWER(TRIM(p.Statut_Presence)), '') AS status_key
                    FROM T_Presences p
                    INNER JOIN T_Reunions r ON r.ID_Reunion = p.Ref_Reunion
                    INNER JOIN T_Affiliations a ON a.Ref_Personne = p.Ref_Personne
                    INNER JOIN T_Structures s ON s.ID_Structure = a.Ref_Structure
                    {where_sql}
                )
                SELECT
                    CAST(structure_id AS TEXT),
                    structure_name,
                    COUNT(*) AS invitations,
                    SUM(CASE WHEN status_key IN ('present', 'présent') THEN 1 ELSE 0 END) AS presents,
                    COALESCE(ROUND(
                        SUM(CASE WHEN status_key IN ('present', 'présent') THEN 100.0 ELSE 0.0 END) / NULLIF(COUNT(*), 0),
                        1
                    ), 0) AS presence_rate
                FROM presence_structure
                GROUP BY structure_id, structure_name
                HAVING COUNT(*) >= 3
                ORDER BY presence_rate DESC, presents DESC, structure_name ASC
                LIMIT 8"
            ),
            params,
        )?
    };

    let top_structures_presence_volume = {
        let mut clauses = Vec::new();
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, "r.Date_Reunion", &filters);
        let where_sql = build_where_sql(&clauses);
        load_stats_participation(
            &conn,
            &format!(
                "WITH presence_structure AS (
                    SELECT DISTINCT
                        p.ID_Presence AS presence_id,
                        s.ID_Structure AS structure_id,
                        COALESCE(s.Nom_Structure, 'Structure sans nom') AS structure_name,
                        COALESCE(LOWER(TRIM(p.Statut_Presence)), '') AS status_key
                    FROM T_Presences p
                    INNER JOIN T_Reunions r ON r.ID_Reunion = p.Ref_Reunion
                    INNER JOIN T_Affiliations a ON a.Ref_Personne = p.Ref_Personne
                    INNER JOIN T_Structures s ON s.ID_Structure = a.Ref_Structure
                    {where_sql}
                )
                SELECT
                    CAST(structure_id AS TEXT),
                    structure_name,
                    COUNT(*) AS invitations,
                    SUM(CASE WHEN status_key IN ('present', 'présent') THEN 1 ELSE 0 END) AS presents,
                    COALESCE(ROUND(
                        SUM(CASE WHEN status_key IN ('present', 'présent') THEN 100.0 ELSE 0.0 END) / NULLIF(COUNT(*), 0),
                        1
                    ), 0) AS presence_rate
                FROM presence_structure
                GROUP BY structure_id, structure_name
                ORDER BY presents DESC, presence_rate DESC, structure_name ASC
                LIMIT 8"
            ),
            params,
        )?
    };

    let top_people_presence = {
        let mut clauses = vec!["COALESCE(pe.Statut_Compte, '') != 'Anonymisé'".to_string()];
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, "r.Date_Reunion", &filters);
        load_stats_participation(
            &conn,
            &format!(
                "SELECT
                    CAST(pe.ID_Personne AS TEXT),
                    TRIM(COALESCE(pe.Nom, '') || ' ' || COALESCE(pe.Prenom, '')),
                    COUNT(*) AS invitations,
                    SUM(CASE WHEN COALESCE(LOWER(TRIM(p.Statut_Presence)), '') IN ('present', 'présent') THEN 1 ELSE 0 END) AS presents,
                    COALESCE(ROUND(
                        SUM(CASE WHEN COALESCE(LOWER(TRIM(p.Statut_Presence)), '') IN ('present', 'présent') THEN 100.0 ELSE 0.0 END) / NULLIF(COUNT(*), 0),
                        1
                    ), 0) AS presence_rate
                 FROM T_Presences p
                 INNER JOIN T_Reunions r ON r.ID_Reunion = p.Ref_Reunion
                 INNER JOIN T_Personnes pe ON pe.ID_Personne = p.Ref_Personne
                 {}
                 GROUP BY pe.ID_Personne, pe.Nom, pe.Prenom
                 HAVING COUNT(*) >= 2
                 ORDER BY presents DESC, presence_rate DESC, pe.Nom ASC, pe.Prenom ASC
                 LIMIT 8",
                build_where_sql(&clauses)
            ),
            params,
        )?
    };

    Ok(DashboardStats {
        generated_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        available_start_date,
        available_end_date,
        overview: DashboardOverview {
            total_contacts,
            active_contacts,
            anonymized_contacts,
            contacts_to_delete,
            total_structures,
            total_categories,
            total_affiliations,
            total_reunions,
            period_reunions,
            period_presences,
            average_presences_per_reunion,
            rgpd_consent_rate,
            partner_direct_structures,
        },
        meetings_by_month,
        attendance_by_status,
        structures_by_category,
        contacts_by_commune,
        structures_by_commune,
        meetings_by_organisme,
        account_statuses,
        contacts_created_by_month,
        quality_checks,
        top_meetings,
        top_structures_presence_rate,
        top_structures_presence_volume,
        top_people_presence,
    })
}

// ──────────────────────────────────────────────
// RGPD
// ──────────────────────────────────────────────

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

// ──────────────────────────────────────────────
// RGPD: PERSONNES QUI ONT REFUSE LA BDD VIA UNE PRESENCE
// ──────────────────────────────────────────────

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

// ──────────────────────────────────────────────
// RGPD: ANONYMISER EN MASSE
// ──────────────────────────────────────────────

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

fn anonymize_person(conn: &rusqlite::Connection, personne_id: i64) -> Result<(), String> {
    ensure_resource_not_locked_by_other(conn, "personnes", personne_id)?;
    conn.execute(
        "UPDATE T_Personnes SET
            Civilite = NULL,
            Nom = ?,
            Prenom = NULL,
            Email_Prive = NULL,
            Telephone_Prive = NULL,
            Adresse_Privee = NULL,
            Code_Postal_Prive = NULL,
            Commune_Privee = NULL,
            Pays = NULL,
            Consentement_RGPD = 0,
            Date_Consentement = NULL,
            Statut_Compte = ?,
            Notes_Commentaires = NULL,
            Date_Creation = NULL,
            Updated_At = datetime('now')
         WHERE ID_Personne = ?",
        rusqlite::params![ANONYMIZED_LABEL, ANONYMIZED_STATUS, personne_id],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "DELETE FROM T_Affiliations WHERE Ref_Personne = ?",
        rusqlite::params![personne_id],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE T_Presences
         SET Notes_Commentaires = NULL,
             Souhaite_Rester_En_BDD = 0
         WHERE Ref_Personne = ?",
        rusqlite::params![personne_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

// ──────────────────────────────────────────────
// ROW MAPPERS
// ──────────────────────────────────────────────

fn map_personne(row: &rusqlite::Row) -> rusqlite::Result<Personne> {
    Ok(Personne {
        id_personne: row.get("ID_Personne")?,
        civilite: row.get("Civilite")?,
        nom: row.get("Nom")?,
        prenom: row.get("Prenom")?,
        email_prive: row.get("Email_Prive")?,
        telephone_prive: row.get("Telephone_Prive")?,
        adresse_privee: row.get("Adresse_Privee")?,
        code_postal_prive: row.get("Code_Postal_Prive")?,
        commune_privee: row.get("Commune_Privee")?,
        pays: row.get("Pays")?,
        consentement_rgpd: row.get("Consentement_RGPD")?,
        date_consentement: row.get("Date_Consentement")?,
        statut_compte: row.get("Statut_Compte")?,
        notes_commentaires: row.get("Notes_Commentaires")?,
        date_creation: row.get("Date_Creation")?,
        updated_at: row.get("Updated_At")?,
    })
}

fn map_structure(row: &rusqlite::Row) -> rusqlite::Result<Structure> {
    Ok(Structure {
        id_structure: row.get("ID_Structure")?,
        nom_structure: row.get("Nom_Structure")?,
        service_specifique: row.get("Service_Specifique")?,
        reseau_subvention: row.get("Reseau_Subvention")?,
        partenaire_direct: row.get("Partenaire_Direct")?,
        adresse_structure: row.get("Adresse_Structure")?,
        code_postal_structure: row.get("Code_Postal_Structure")?,
        commune_structure: row.get("Commune_Structure")?,
        pays: row.get("Pays")?,
        telephone_general: row.get("Telephone_General")?,
        email_general: row.get("Email_General")?,
        site_web: row.get("Site_Web")?,
        notes_commentaires: row.get("Notes_Commentaires")?,
        date_creation: row.get("Date_Creation")?,
        id_categorie: row.get("ID_Categorie")?,
        updated_at: row.get("Updated_At")?,
    })
}

fn map_categorie(row: &rusqlite::Row) -> rusqlite::Result<Categorie> {
    Ok(Categorie {
        id_categorie: row.get("ID_Categorie")?,
        nom_categorie: row.get("Nom_Categorie")?,
    })
}

fn map_fonction(row: &rusqlite::Row) -> rusqlite::Result<Fonction> {
    Ok(Fonction {
        id_fonction: row.get("ID_Fonction")?,
        libelle_fonction: row.get("Libelle_Fonction")?,
    })
}

fn map_reunion(row: &rusqlite::Row) -> rusqlite::Result<Reunion> {
    Ok(Reunion {
        id_reunion: row.get("ID_Reunion")?,
        titre_reunion: row.get("Titre_Reunion")?,
        date_reunion: row.get("Date_Reunion")?,
        heure_reunion: row.get("Heure_Reunion")?,
        lieu_reunion: row.get("Lieu_Reunion")?,
        ref_structure: row.get("Ref_Structure")?,
        nom_structure: row.get("Nom_Structure")?,
        notes_commentaires: row.get("Notes_Commentaires")?,
        updated_at: row.get("Updated_At")?,
    })
}

fn append_date_filters(
    clauses: &mut Vec<String>,
    params: &mut Vec<String>,
    column: &str,
    filters: &StatsFilters,
) {
    if let Some(start) = filters.start_date.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
        clauses.push(format!("date({}) >= date(?)", column));
        params.push(start.to_string());
    }

    if let Some(end) = filters.end_date.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
        clauses.push(format!("date({}) <= date(?)", column));
        params.push(end.to_string());
    }
}

fn sqlite_date_expr(column: &str) -> String {
    format!(
        "CASE
            WHEN instr({column}, '/') > 0 THEN date('20' || substr({column}, 7, 2) || '-' || substr({column}, 1, 2) || '-' || substr({column}, 4, 2))
            ELSE date({column})
         END"
    )
}

fn sqlite_month_expr(column: &str) -> String {
    format!(
        "CASE
            WHEN instr({column}, '/') > 0 THEN '20' || substr({column}, 7, 2) || '-' || substr({column}, 1, 2)
            ELSE substr({column}, 1, 7)
         END"
    )
}

fn build_where_sql(clauses: &[String]) -> String {
    if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    }
}

fn get_query_meta_select_parts(main_alias: &str) -> Vec<String> {
    match main_alias {
        "p" => vec!["p.ID_Personne AS _id".to_string()],
        "s" => vec!["s.ID_Structure AS _id".to_string()],
        "r" => vec!["r.ID_Reunion AS _id".to_string()],
        "a" => vec![
            "a.ID_Affiliation AS _id".to_string(),
            "a.Ref_Personne AS _personne_id".to_string(),
            "a.Ref_Structure AS _structure_id".to_string(),
        ],
        "pr" => vec![
            "pr.ID_Presence AS _id".to_string(),
            "pr.Ref_Reunion AS _reunion_id".to_string(),
        ],
        _ => Vec::new(),
    }
}

fn query_i64(
    conn: &rusqlite::Connection,
    sql: &str,
    params: Vec<String>,
) -> Result<i64, String> {
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
        .iter()
        .map(|p| p as &dyn rusqlite::types::ToSql)
        .collect();
    conn.query_row(sql, param_refs.as_slice(), |row| row.get::<_, i64>(0))
        .map_err(|e| e.to_string())
}

fn query_f64(
    conn: &rusqlite::Connection,
    sql: &str,
    params: Vec<String>,
) -> Result<f64, String> {
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
        .iter()
        .map(|p| p as &dyn rusqlite::types::ToSql)
        .collect();
    conn.query_row(sql, param_refs.as_slice(), |row| row.get::<_, f64>(0))
        .map_err(|e| e.to_string())
}

fn load_stats_buckets(
    conn: &rusqlite::Connection,
    sql: &str,
    params: Vec<String>,
) -> Result<Vec<StatsBucket>, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
        .iter()
        .map(|p| p as &dyn rusqlite::types::ToSql)
        .collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(StatsBucket {
                key: row.get(0)?,
                label: row.get(1)?,
                value: row.get(2)?,
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

fn load_top_meetings(
    conn: &rusqlite::Connection,
    sql: &str,
    params: Vec<String>,
) -> Result<Vec<StatsTopMeeting>, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
        .iter()
        .map(|p| p as &dyn rusqlite::types::ToSql)
        .collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(StatsTopMeeting {
                label: row.get(0)?,
                value: row.get(1)?,
                date_reunion: row.get(2)?,
                organisme: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

fn load_stats_participation(
    conn: &rusqlite::Connection,
    sql: &str,
    params: Vec<String>,
) -> Result<Vec<StatsParticipation>, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
        .iter()
        .map(|p| p as &dyn rusqlite::types::ToSql)
        .collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(StatsParticipation {
                key: row.get(0)?,
                label: row.get(1)?,
                invitations: row.get(2)?,
                presents: row.get(3)?,
                presence_rate: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

fn merge_casefolded_buckets(items: Vec<StatsBucket>) -> Vec<StatsBucket> {
    use std::collections::BTreeMap;

    let mut merged: BTreeMap<String, StatsBucket> = BTreeMap::new();

    for item in items {
        let normalized_key = item.label.trim().to_lowercase();
        match merged.get_mut(&normalized_key) {
            Some(existing) => {
                existing.value += item.value;
                if is_better_bucket_label(&item.label, &existing.label) {
                    existing.label = item.label.clone();
                    existing.key = item.key.clone();
                }
            }
            None => {
                merged.insert(normalized_key, item);
            }
        }
    }

    let mut result: Vec<StatsBucket> = merged.into_values().collect();
    result.sort_by(|a, b| b.value.cmp(&a.value).then_with(|| a.label.cmp(&b.label)));
    result.truncate(8);
    result
}

fn is_better_bucket_label(candidate: &str, current: &str) -> bool {
    let candidate_all_caps = candidate == candidate.to_uppercase();
    let current_all_caps = current == current.to_uppercase();

    if current_all_caps && !candidate_all_caps {
        return true;
    }

    if candidate_all_caps == current_all_caps {
        return candidate.len() > current.len();
    }

    false
}

// ──────────────────────────────────────────────
// CATEGORIES: PERSONNES AVEC DONNÉES DÉTAILLÉES (JOINS)
// ──────────────────────────────────────────────

#[tauri::command]
pub fn lister_personnes_categorie_detaillee(
    app: AppHandle,
    categorie_id: Option<i64>,
    recherche: Option<String>,
) -> Result<Vec<PersonneCategorieDetaillee>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "personnes.read")?;

    // Partie 1 : les personnes avec leurs affiliations
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

    // Partie 2 : les structures sans personne référente mais avec catégorie directe
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

    // Mêmes filtres catégorie et recherche pour la partie structure
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

// ──────────────────────────────────────────────
// VÉRIFICATION DOUBLONS (emails)
// ──────────────────────────────────────────────

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

// ──────────────────────────────────────────────
// QUERY BUILDER
// ──────────────────────────────────────────────

#[tauri::command]
pub fn executer_requete(
    app: AppHandle,
    table_principale: String,
    colonnes: Vec<String>,
    conditions: Vec<Condition>,
) -> Result<Vec<serde_json::Value>, String> {
    query_builder::executer_requete(app, table_principale, colonnes, conditions)
}

// ──────────────────────────────────────────────
// PRESETS (Query Builder)
// ──────────────────────────────────────────────

#[tauri::command]
pub fn lister_presets(app: AppHandle) -> Result<Vec<Preset>, String> {
    presets::lister_presets(app)
}

#[tauri::command]
pub fn sauvegarder_preset(app: AppHandle, preset: PresetInput) -> Result<Preset, String> {
    presets::sauvegarder_preset(app, preset)
}

#[tauri::command]
pub fn supprimer_preset(app: AppHandle, id: i64) -> Result<(), String> {
    presets::supprimer_preset(app, id)
}

#[tauri::command]
pub fn charger_preset(app: AppHandle, id: i64) -> Result<Preset, String> {
    presets::charger_preset(app, id)
}

