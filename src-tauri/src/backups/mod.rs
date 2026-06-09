mod crypto;
mod dpapi;
mod envelope;
mod paths;
mod recovery_files;
mod restore;

use crate::auth;
use crate::db;
use crate::models::{
    BackupInfo, BackupRunResult, BackupRunStatus, ManualBackupRequest, RestoreBackupRequest,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Local;
use rand_core::{OsRng, RngCore};
use rusqlite::{Connection, OptionalExtension};
use std::fs;
use std::path::Path;
use std::process::Command;
use tauri::AppHandle;

use crypto::*;
use envelope::*;
use paths::*;
use restore::*;

const AUTO_BACKUP_KEY: &str = "AUTO_HOURLY";
const AUTO_BACKUP_INTERVAL_MINUTES: i64 = 60;
const AUTO_BACKUP_LEASE_MINUTES: i64 = 15;
const MAX_LOCAL_BACKUPS: usize = 10;

pub fn maybe_run_auto_backup(
    app: &AppHandle,
    conn: &Connection,
    holder_label: &str,
    machine_label: &str,
) -> Result<BackupRunResult, String> {
    let db_path = current_db_path(app)?;
    let backup_dir = backup_dir_for_db(app, &db_path)?;
    ensure_backup_dir(&backup_dir)?;

    let state = conn
        .query_row(
            "SELECT Holder_Label, Machine_Label, Expires_At, Last_Success_At
             FROM T_BackupState
             WHERE Lock_Key = ?",
            rusqlite::params![AUTO_BACKUP_KEY],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if let Some((_, _, _, Some(last_success_at))) = &state {
        let is_recent: i64 = conn
            .query_row(
                "SELECT CASE WHEN datetime(?) >= datetime('now', ?) THEN 1 ELSE 0 END",
                rusqlite::params![
                    last_success_at,
                    format!("-{} minutes", AUTO_BACKUP_INTERVAL_MINUTES)
                ],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if is_recent == 1 {
            return Ok(BackupRunResult {
                status: BackupRunStatus::Skipped,
                message: "Un backup automatique récent existe déjà.".into(),
                backup: None,
            });
        }
    }

    if let Some((existing_holder, existing_machine, Some(expires_at), _)) = &state {
        let lease_active: i64 = conn
            .query_row(
                "SELECT CASE WHEN datetime(?) > datetime('now') THEN 1 ELSE 0 END",
                rusqlite::params![expires_at],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if lease_active == 1 {
            return Ok(BackupRunResult {
                status: BackupRunStatus::Skipped,
                message: format!(
                    "Backup automatique déjà pris en charge par {}{}.",
                    existing_holder.as_deref().unwrap_or("un autre poste"),
                    existing_machine
                        .as_deref()
                        .map(|m| format!(" ({m})"))
                        .unwrap_or_default()
                ),
                backup: None,
            });
        }
    }

    conn.execute(
        "INSERT INTO T_BackupState (Lock_Key, Holder_Label, Machine_Label, Expires_At, Last_Success_At)
         VALUES (?, ?, ?, datetime('now', ?), COALESCE((SELECT Last_Success_At FROM T_BackupState WHERE Lock_Key = ?), NULL))
         ON CONFLICT(Lock_Key) DO UPDATE SET
            Holder_Label = excluded.Holder_Label,
            Machine_Label = excluded.Machine_Label,
            Expires_At = excluded.Expires_At",
        rusqlite::params![
            AUTO_BACKUP_KEY,
            holder_label,
            machine_label,
            format!("+{} minutes", AUTO_BACKUP_LEASE_MINUTES),
            AUTO_BACKUP_KEY
        ],
    )
    .map_err(|e| e.to_string())?;

    match create_backup_impl(app, conn, true, BackupMode::WindowsLocal, "auto_local") {
        Ok(backup) => {
            conn.execute(
                "UPDATE T_BackupState
                 SET Last_Success_At = datetime('now'), Expires_At = NULL
                 WHERE Lock_Key = ?",
                rusqlite::params![AUTO_BACKUP_KEY],
            )
            .map_err(|e| e.to_string())?;
            Ok(BackupRunResult {
                status: BackupRunStatus::Created,
                message: "Backup automatique local chiffré créé.".into(),
                backup: Some(backup),
            })
        }
        Err(e) => {
            conn.execute(
                "UPDATE T_BackupState SET Expires_At = NULL WHERE Lock_Key = ?",
                rusqlite::params![AUTO_BACKUP_KEY],
            )
            .ok();
            Err(e)
        }
    }
}

pub fn create_backup_now(
    app: &AppHandle,
    conn: &Connection,
    request: ManualBackupRequest,
) -> Result<BackupRunResult, String> {
    let (mode, suffix, message) = if request.portable {
        let passphrase = request
            .passphrase
            .filter(|value| value.chars().count() >= 12)
            .ok_or_else(|| {
                "Le mot de passe du backup portable doit contenir au moins 12 caractères."
                    .to_string()
            })?;
        (
            BackupMode::PortablePassphrase(passphrase),
            "manual_portable",
            "Backup portable chiffré créé.",
        )
    } else {
        (
            BackupMode::WindowsLocal,
            "manual_local",
            "Backup local chiffré créé.",
        )
    };

    let backup = create_backup_impl(app, conn, false, mode, suffix)?;
    Ok(BackupRunResult {
        status: BackupRunStatus::Created,
        message: message.into(),
        backup: Some(backup),
    })
}

pub fn list_local_backups(app: &AppHandle) -> Result<Vec<BackupInfo>, String> {
    let db_path = current_db_path(app)?;
    let backup_dir = backup_dir_for_db(app, &db_path)?;
    ensure_backup_dir(&backup_dir)?;

    let mut entries = Vec::new();
    for entry in fs::read_dir(&backup_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !is_supported_backup_path(&path) {
            continue;
        }

        let metadata = entry.metadata().map_err(|e| e.to_string())?;
        let descriptor = describe_backup_file(&path)?;
        entries.push(backup_info_from_descriptor(&path, &metadata, descriptor));
    }

    entries.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| b.name.cmp(&a.name))
    });
    Ok(entries)
}

pub fn get_backup_directory(app: &AppHandle) -> Result<String, String> {
    let db_path = current_db_path(app)?;
    let backup_dir = backup_dir_for_db(app, &db_path)?;
    ensure_backup_dir(&backup_dir)?;
    Ok(backup_dir.to_string_lossy().into_owned())
}

pub fn open_backup_directory(app: &AppHandle) -> Result<String, String> {
    let directory = get_backup_directory(app)?;
    Command::new("explorer")
        .arg(&directory)
        .spawn()
        .map_err(|e| format!("Impossible d'ouvrir le dossier de sauvegarde: {}", e))?;
    Ok(directory)
}

pub fn restore_backup(
    app: &AppHandle,
    preferred_admin_username: Option<&str>,
    request: RestoreBackupRequest,
) -> Result<BackupRunResult, String> {
    let db_path = current_db_path(app)?;
    let backup_dir = backup_dir_for_db(app, &db_path)?;
    ensure_backup_dir(&backup_dir)?;

    let backup = std::path::PathBuf::from(&request.backup_path);
    let backup = backup
        .canonicalize()
        .map_err(|e| format!("Backup introuvable: {}", e))?;
    let allowed_root = backup_dir
        .canonicalize()
        .map_err(|e| format!("Dossier de backup introuvable: {}", e))?;

    if !backup.starts_with(&allowed_root) {
        return Err(
            "Le backup sélectionné n'appartient pas au dossier local de sauvegarde.".into(),
        );
    }

    let descriptor = describe_backup_file(&backup)?;
    let decrypted_bytes = match &descriptor {
        BackupDescriptor::LegacyPlain { .. } => None,
        BackupDescriptor::Encrypted { header } => Some(decrypt_backup_payload(
            &backup,
            header,
            request.passphrase.as_deref(),
        )?),
    };

    let emergency_copy = create_emergency_backup_from_current_db(app, &db_path, &backup_dir)?;

    let temp_target = db_path.with_extension("restore_tmp.sqlite");
    if temp_target.exists() {
        let _ = fs::remove_file(&temp_target);
    }

    match decrypted_bytes {
        Some(bytes) => {
            fs::write(&temp_target, &bytes).map_err(|e| {
                format!("Impossible de préparer le backup pour restauration: {}", e)
            })?;
            if !verify_backup_file_internal(&temp_target)? {
                let _ = fs::remove_file(&temp_target);
                return Err(
                    "Le backup chiffré déchiffré a échoué au contrôle d'intégrité SQLite.".into(),
                );
            }
        }
        None => {
            if !verify_backup_file_internal(&backup)? {
                return Err(
                    "Le backup sélectionné a échoué au contrôle d'intégrité SQLite.".into(),
                );
            }
            fs::copy(&backup, &temp_target).map_err(|e| {
                format!("Impossible de préparer le backup pour restauration: {}", e)
            })?;
        }
    }

    let rollback_path = db_path.with_extension("restore_rollback.sqlite");
    if rollback_path.exists() {
        fs::remove_file(&rollback_path)
            .map_err(|e| format!("Impossible de nettoyer l'ancien rollback: {e}"))?;
    }
    if db_path.exists() {
        fs::rename(&db_path, &rollback_path).map_err(|e| {
            format!("Impossible de préparer le remplacement de la base actuelle: {e}")
        })?;
    }
    if let Err(error) = fs::rename(&temp_target, &db_path) {
        if rollback_path.exists() {
            let _ = fs::rename(&rollback_path, &db_path);
        }
        return Err(format!("Impossible de finaliser la restauration: {error}"));
    }

    // À partir d'ici, toute erreur doit déclencher un rollback complet.
    // Le fichier rollback ne sera supprimé qu'après validation complète.
    if let Err(error) = db::connect(&db_path.to_string_lossy()) {
        return Err(rollback_error(
            &db_path,
            &rollback_path,
            format!("Base restaurée mais reconnexion impossible: {error}"),
        ));
    }
    let conn = match db::get_conn(app) {
        Ok(conn) => conn,
        Err(error) => {
            return Err(rollback_error(
                &db_path,
                &rollback_path,
                format!("Base restaurée mais validation d'accès impossible: {error}"),
            ));
        }
    };
    let recovered_admin = match auth::recover_admin_access(&conn, preferred_admin_username) {
        Ok(admin) => admin,
        Err(error) => {
            drop(conn);
            return Err(rollback_error(
                &db_path,
                &rollback_path,
                format!(
                    "Base restaurée mais récupération de l'accès administrateur impossible: {error}"
                ),
            ));
        }
    };
    drop(conn);
    // Tout a réussi : on peut supprimer le rollback.
    if rollback_path.exists() {
        fs::remove_file(&rollback_path)
            .map_err(|e| format!("Base restaurée, mais nettoyage du rollback impossible: {e}"))?;
    }

    let backup_metadata = fs::metadata(&backup).map_err(|e| e.to_string())?;
    let admin_message = recovered_admin
        .map(|username| format!(" Accès administrateur sécurisé via le compte {}.", username))
        .unwrap_or_default();

    Ok(BackupRunResult {
        status: BackupRunStatus::Restored,
        message: format!(
            "Backup restauré. Une copie locale chiffrée pré-restauration a été conservée dans {}.{}",
            emergency_copy,
            admin_message
        ),
        backup: Some(backup_info_from_descriptor(&backup, &backup_metadata, descriptor)),
    })
}

pub fn delete_backup(app: &AppHandle, backup_path: &str) -> Result<(), String> {
    let db_path = current_db_path(app)?;
    let backup_dir = backup_dir_for_db(app, &db_path)?;
    ensure_backup_dir(&backup_dir)?;

    let backup = std::path::PathBuf::from(backup_path);
    let backup = backup
        .canonicalize()
        .map_err(|e| format!("Backup introuvable: {}", e))?;
    let allowed_root = backup_dir
        .canonicalize()
        .map_err(|e| format!("Dossier de backup introuvable: {}", e))?;

    if !backup.starts_with(&allowed_root) {
        return Err(
            "Le backup sélectionné n'appartient pas au dossier local de sauvegarde.".into(),
        );
    }
    if !is_supported_backup_path(&backup) {
        return Err("Seuls les fichiers de backup CRVI peuvent être supprimés.".into());
    }

    fs::remove_file(&backup).map_err(|e| format!("Impossible de supprimer le backup: {}", e))?;
    Ok(())
}

fn rollback_error(db_path: &Path, rollback_path: &Path, original_error: String) -> String {
    match rollback_restore(db_path, rollback_path)
        .and_then(|_| db::connect(&db_path.to_string_lossy()))
    {
        Ok(()) => format!("{original_error}. L'ancienne base a été restaurée."),
        Err(rollback_error) => format!("{original_error}. {rollback_error}"),
    }
}

/// Restaure le fichier rollback par dessus la base active.
fn rollback_restore(db_path: &Path, rollback_path: &Path) -> Result<(), String> {
    if !rollback_path.exists() {
        return Err(format!(
            "Échec critique: fichier rollback introuvable à {}",
            rollback_path.display()
        ));
    }
    if db_path.exists() {
        fs::remove_file(db_path).map_err(|e| {
            format!("Échec critique: impossible de retirer la base restaurée avant rollback ({e})")
        })?;
    }
    fs::rename(rollback_path, db_path).map_err(|e| {
        format!(
            "Échec critique: impossible de restaurer l'ancienne base ({e}). \
             Le fichier rollback est conservé à {}",
            rollback_path.display()
        )
    })?;
    Ok(())
}

fn create_backup_impl(
    app: &AppHandle,
    conn: &Connection,
    automatic: bool,
    mode: BackupMode,
    name_suffix: &str,
) -> Result<BackupInfo, String> {
    let db_path = current_db_path(app)?;
    let backup_dir = backup_dir_for_db(app, &db_path)?;
    ensure_backup_dir(&backup_dir)?;

    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let temp_sqlite = backup_dir.join(format!("crvi_backup_{timestamp}_{name_suffix}.tmp.sqlite"));
    if temp_sqlite.exists() {
        let _ = fs::remove_file(&temp_sqlite);
    }

    conn.execute(
        "VACUUM INTO ?1",
        rusqlite::params![temp_sqlite.to_string_lossy().to_string()],
    )
    .map_err(|e| format!("Échec du backup SQLite: {}", e))?;

    let finalize_result = finalize_snapshot_as_backup_impl(
        &temp_sqlite,
        &backup_dir,
        automatic,
        &timestamp,
        mode,
        name_suffix,
    );
    let _ = fs::remove_file(&temp_sqlite);
    let backup = finalize_result?;

    prune_old_backups(&backup_dir)?;
    Ok(backup)
}

pub(crate) fn finalize_snapshot_as_backup_impl(
    snapshot_path: &Path,
    backup_dir: &Path,
    automatic: bool,
    timestamp: &str,
    mode: BackupMode,
    name_suffix: &str,
) -> Result<BackupInfo, String> {
    if !verify_backup_file_internal(snapshot_path)? {
        return Err("Le backup créé n'a pas passé le contrôle d'intégrité.".into());
    }

    let snapshot_bytes = fs::read(snapshot_path)
        .map_err(|e| format!("Impossible de lire le snapshot SQLite: {}", e))?;
    let created_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let backup_path = backup_dir.join(format!("crvi_backup_{timestamp}_{name_suffix}.crvibak"));

    let header = match &mode {
        BackupMode::WindowsLocal => BackupEnvelopeHeader {
            version: 1,
            scheme: crypto::WINDOWS_DPAPI_SCHEME.to_string(),
            created_at: created_at.clone(),
            automatic,
            salt_b64: None,
            nonce_b64: None,
            pbkdf2_iterations: None,
        },
        BackupMode::PortablePassphrase(_) => {
            let mut salt = [0u8; 16];
            let mut nonce = [0u8; 12];
            OsRng.fill_bytes(&mut salt);
            OsRng.fill_bytes(&mut nonce);
            BackupEnvelopeHeader {
                version: 1,
                scheme: crypto::PORTABLE_PASSPHRASE_SCHEME.to_string(),
                created_at: created_at.clone(),
                automatic,
                salt_b64: Some(BASE64.encode(salt)),
                nonce_b64: Some(BASE64.encode(nonce)),
                pbkdf2_iterations: Some(crypto::PBKDF2_ITERATIONS),
            }
        }
    };

    let payload = encrypt_snapshot_bytes(&snapshot_bytes, &header, &mode)?;
    let roundtrip = decrypt_snapshot_bytes(
        &payload,
        &header,
        match &mode {
            BackupMode::WindowsLocal => None,
            BackupMode::PortablePassphrase(passphrase) => Some(passphrase.as_str()),
        },
    )?;
    if roundtrip != snapshot_bytes {
        return Err("Le backup chiffré n'a pas pu être relu correctement après écriture.".into());
    }

    write_backup_envelope(&backup_path, &header, &payload)?;
    let metadata = fs::metadata(&backup_path).map_err(|e| e.to_string())?;

    Ok(BackupInfo {
        name: backup_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("backup.crvibak")
            .to_string(),
        path: backup_path.to_string_lossy().to_string(),
        created_at: Some(created_at),
        size_bytes: metadata.len(),
        automatic,
        encryption_kind: header.scheme.clone(),
        encryption_label: encryption_label(&header.scheme).to_string(),
        requires_passphrase: header.scheme == crypto::PORTABLE_PASSPHRASE_SCHEME,
        legacy_unencrypted: false,
    })
}

fn prune_old_backups(dir: &Path) -> Result<(), String> {
    let mut entries: Vec<(std::path::PathBuf, String)> = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if !is_supported_backup_path(&path) {
                return None;
            }
            let descriptor = describe_backup_file(&path).ok()?;
            let automatic = match descriptor {
                BackupDescriptor::LegacyPlain { automatic } => automatic,
                BackupDescriptor::Encrypted { header } => header.automatic,
            };
            if automatic {
                Some((
                    path,
                    entry.file_name().to_str().unwrap_or_default().to_string(),
                ))
            } else {
                None
            }
        })
        .collect();

    entries.sort_by(|a, b| a.1.cmp(&b.1));
    if entries.len() <= MAX_LOCAL_BACKUPS {
        return Ok(());
    }

    let to_remove = entries.len() - MAX_LOCAL_BACKUPS;
    for (path, _) in entries.into_iter().take(to_remove) {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;

    /// Crée un répertoire temporaire avec une base SQLite source.
    fn setup_temp_db() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("test.sqlite");
        let rollback_path = dir.path().join("test.restore_rollback.sqlite");

        // Créer une base source avec un token connu
        let conn = Connection::open(&db_path).expect("open db");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT);
             INSERT INTO meta (key, value) VALUES ('version', 'original');",
        )
        .expect("schema");
        drop(conn);

        // Créer la base rollback (l'ancienne base sauvegardée avant restauration)
        let conn_rb = Connection::open(&rollback_path).expect("open rollback");
        conn_rb
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT);
                 INSERT INTO meta (key, value) VALUES ('version', 'rollback');",
            )
            .expect("rollback schema");
        drop(conn_rb);

        (dir, db_path, rollback_path)
    }

    /// Vérifie que le chemin principal contient la base d'origine (version = 'rollback').
    fn assert_original_db_restored(db_path: &Path) {
        let conn = Connection::open(db_path).expect("open restored db");
        let version: String = conn
            .query_row("SELECT value FROM meta WHERE key = 'version'", [], |row| {
                row.get(0)
            })
            .expect("read version");
        assert_eq!(version, "rollback", "la base d'origine doit être restaurée");
    }

    /// Vérifie qu'aucun fichier rollback orphelin ne subsiste.
    fn assert_no_rollback_orphan(rollback_path: &Path) {
        assert!(
            !rollback_path.exists(),
            "le fichier rollback ne doit pas subsister après échec: {}",
            rollback_path.display()
        );
    }

    #[test]
    fn rollback_restaure_fichier_source() {
        let (_dir, db_path, rollback_path) = setup_temp_db();
        // Simuler le remplacement : la nouvelle base écrase l'originale
        let conn_new = Connection::open(&db_path).expect("open new db");
        conn_new
            .execute(
                "INSERT OR REPLACE INTO meta (key, value) VALUES ('version', 'new')",
                [],
            )
            .expect("write new");
        drop(conn_new);

        // Exécuter rollback_restore comme si une étape avait échoué
        rollback_restore(&db_path, &rollback_path).expect("rollback");
        // Vérifier que le contenu est celui du rollback
        assert_original_db_restored(&db_path);
        assert_no_rollback_orphan(&rollback_path);
    }

    #[test]
    fn rollback_introuvable_preserve_la_base_active() {
        let (_dir, db_path, rollback_path) = setup_temp_db();
        fs::remove_file(&rollback_path).expect("remove rollback");

        let error = rollback_restore(&db_path, &rollback_path).expect_err("rollback doit échouer");
        assert!(error.contains("rollback introuvable"));

        let conn = Connection::open(&db_path).expect("open active db");
        let version: String = conn
            .query_row("SELECT value FROM meta WHERE key = 'version'", [], |row| {
                row.get(0)
            })
            .expect("read active version");
        assert_eq!(version, "original");
    }

    #[test]
    fn rollback_apres_admin_recovery_echoue() {
        // Simule : remplacement réussi, puis recover_admin_access échoue.
        // On vérifie que rollback_restore rend le fichier source original.
        let (_dir, db_path, rollback_path) = setup_temp_db();

        let conn_new = Connection::open(&db_path).expect("open new db");
        conn_new
            .execute(
                "INSERT OR REPLACE INTO meta (key, value) VALUES ('version', 'new')",
                [],
            )
            .expect("write new");
        drop(conn_new);

        // Simuler l'échec de recover_admin_access → rollback
        rollback_restore(&db_path, &rollback_path).expect("rollback");
        assert_original_db_restored(&db_path);
        assert_no_rollback_orphan(&rollback_path);
    }
}
