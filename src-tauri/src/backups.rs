use crate::auth;
use crate::db;
use crate::models::{
    BackupInfo, BackupRunResult, BackupRunStatus, ManualBackupRequest, RestoreBackupRequest,
};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Local;
use pbkdf2::pbkdf2_hmac_array;
use rand_core::{OsRng, RngCore};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Manager};

const AUTO_BACKUP_KEY: &str = "AUTO_HOURLY";
const AUTO_BACKUP_INTERVAL_MINUTES: i64 = 60;
const AUTO_BACKUP_LEASE_MINUTES: i64 = 15;
const MAX_LOCAL_BACKUPS: usize = 10;

const BACKUP_MAGIC: &[u8; 8] = b"CRVIBAK1";
const BACKUP_EXTENSION: &str = "crvibak";
const LEGACY_SQLITE_EXTENSION: &str = "sqlite";
const WINDOWS_DPAPI_SCHEME: &str = "windows-dpapi";
const PORTABLE_PASSPHRASE_SCHEME: &str = "portable-passphrase";
const PBKDF2_ITERATIONS: u32 = 600_000;
const RECOVERY_SCRIPT_NAME: &str = "decrypt-crvi-backup.ps1";
const RECOVERY_README_NAME: &str = "README_RECOVERY.txt";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupEnvelopeHeader {
    version: u8,
    scheme: String,
    created_at: String,
    automatic: bool,
    salt_b64: Option<String>,
    nonce_b64: Option<String>,
    pbkdf2_iterations: Option<u32>,
}

#[derive(Debug, Clone)]
enum BackupMode {
    WindowsLocal,
    PortablePassphrase(String),
}

#[derive(Debug, Clone)]
enum BackupDescriptor {
    LegacyPlain {
        automatic: bool,
    },
    Encrypted {
        header: BackupEnvelopeHeader,
    },
}

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

    match create_backup(app, conn, true, BackupMode::WindowsLocal, "auto_local") {
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

    let backup = create_backup(app, conn, false, mode, suffix)?;
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

    let backup = PathBuf::from(&request.backup_path);
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
            fs::write(&temp_target, &bytes)
                .map_err(|e| format!("Impossible de préparer le backup pour restauration: {}", e))?;
            if !verify_backup_file(&temp_target)? {
                let _ = fs::remove_file(&temp_target);
                return Err(
                    "Le backup chiffré déchiffré a échoué au contrôle d'intégrité SQLite.".into(),
                );
            }
        }
        None => {
            if !verify_backup_file(&backup)? {
                return Err(
                    "Le backup sélectionné a échoué au contrôle d'intégrité SQLite.".into(),
                );
            }
            fs::copy(&backup, &temp_target)
                .map_err(|e| format!("Impossible de préparer le backup pour restauration: {}", e))?;
        }
    }

    if db_path.exists() {
        fs::remove_file(&db_path).map_err(|e| {
            format!(
                "Impossible de remplacer la base actuelle. Vérifiez que les autres postes ne l'utilisent pas: {}",
                e
            )
        })?;
    }
    fs::rename(&temp_target, &db_path)
        .map_err(|e| format!("Impossible de finaliser la restauration: {}", e))?;

    db::connect(&db_path.to_string_lossy())
        .map_err(|e| format!("Base restaurée mais reconnexion impossible: {}", e))?;

    let conn = db::get_conn(app)
        .map_err(|e| format!("Base restaurée mais validation d'accès impossible: {}", e))?;
    let recovered_admin = auth::recover_admin_access(&conn, preferred_admin_username)?;

    let backup_metadata = fs::metadata(&backup).map_err(|e| e.to_string())?;
    let admin_message = recovered_admin.map(|username| {
        format!(
            " Accès administrateur sécurisé via le compte {}.",
            username
        )
    }).unwrap_or_default();

    Ok(BackupRunResult {
        status: BackupRunStatus::Restored,
        message: format!(
            "Backup restauré. Une copie locale chiffrée pré-restauration a été conservée dans {}.{}",
            emergency_copy,
            admin_message
        ),
        backup: Some(backup_info_from_descriptor(
            &backup,
            &backup_metadata,
            descriptor,
        )),
    })
}

pub fn delete_backup(app: &AppHandle, backup_path: &str) -> Result<(), String> {
    let db_path = current_db_path(app)?;
    let backup_dir = backup_dir_for_db(app, &db_path)?;
    ensure_backup_dir(&backup_dir)?;

    let backup = PathBuf::from(backup_path);
    let backup = backup
        .canonicalize()
        .map_err(|e| format!("Backup introuvable: {}", e))?;
    let allowed_root = backup_dir
        .canonicalize()
        .map_err(|e| format!("Dossier de backup introuvable: {}", e))?;

    if !backup.starts_with(&allowed_root) {
        return Err("Le backup sélectionné n'appartient pas au dossier local de sauvegarde.".into());
    }
    if !is_supported_backup_path(&backup) {
        return Err("Seuls les fichiers de backup CRVI peuvent être supprimés.".into());
    }

    fs::remove_file(&backup).map_err(|e| format!("Impossible de supprimer le backup: {}", e))?;
    Ok(())
}

fn create_backup(
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

    let finalize_result = finalize_snapshot_as_backup(
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

fn finalize_snapshot_as_backup(
    snapshot_path: &Path,
    backup_dir: &Path,
    automatic: bool,
    timestamp: &str,
    mode: BackupMode,
    name_suffix: &str,
) -> Result<BackupInfo, String> {
    if !verify_backup_file(snapshot_path)? {
        return Err("Le backup créé n'a pas passé le contrôle d'intégrité.".into());
    }

    let snapshot_bytes =
        fs::read(snapshot_path).map_err(|e| format!("Impossible de lire le snapshot SQLite: {}", e))?;
    let created_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let backup_path = backup_dir.join(format!(
        "crvi_backup_{timestamp}_{name_suffix}.{BACKUP_EXTENSION}"
    ));

    let header = match &mode {
        BackupMode::WindowsLocal => BackupEnvelopeHeader {
            version: 1,
            scheme: WINDOWS_DPAPI_SCHEME.to_string(),
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
                scheme: PORTABLE_PASSPHRASE_SCHEME.to_string(),
                created_at: created_at.clone(),
                automatic,
                salt_b64: Some(BASE64.encode(salt)),
                nonce_b64: Some(BASE64.encode(nonce)),
                pbkdf2_iterations: Some(PBKDF2_ITERATIONS),
            }
        }
    };

    let payload = encrypt_snapshot_bytes(&snapshot_bytes, &header, &mode)?;
    let roundtrip = decrypt_snapshot_bytes(&payload, &header, match &mode {
        BackupMode::WindowsLocal => None,
        BackupMode::PortablePassphrase(passphrase) => Some(passphrase.as_str()),
    })?;
    if roundtrip != snapshot_bytes {
        return Err(
            "Le backup chiffré n'a pas pu être relu correctement après écriture.".into(),
        );
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
        requires_passphrase: header.scheme == PORTABLE_PASSPHRASE_SCHEME,
        legacy_unencrypted: false,
    })
}

fn create_emergency_backup_from_current_db(
    app: &AppHandle,
    db_path: &Path,
    backup_dir: &Path,
) -> Result<String, String> {
    let temp_snapshot = backup_dir.join(format!(
        "pre_restore_{}.tmp.sqlite",
        Local::now().format("%Y-%m-%d_%H-%M-%S")
    ));
    if temp_snapshot.exists() {
        let _ = fs::remove_file(&temp_snapshot);
    }
    fs::copy(db_path, &temp_snapshot).map_err(|e| {
        format!(
            "Impossible de copier la base actuelle avant restauration: {}",
            e
        )
    })?;

    let finalize_result = finalize_snapshot_as_backup(
        &temp_snapshot,
        backup_dir,
        false,
        &Local::now().format("%Y-%m-%d_%H-%M-%S").to_string(),
        BackupMode::WindowsLocal,
        "pre_restore_local",
    );
    let _ = fs::remove_file(&temp_snapshot);
    let backup = finalize_result?;
    let _ = app;
    Ok(backup.path)
}

fn write_backup_envelope(
    path: &Path,
    header: &BackupEnvelopeHeader,
    payload: &[u8],
) -> Result<(), String> {
    if path.exists() {
        let _ = fs::remove_file(path);
    }

    let header_bytes = serde_json::to_vec(header).map_err(|e| e.to_string())?;
    let header_len = u32::try_from(header_bytes.len())
        .map_err(|_| "En-tête de backup trop volumineux.".to_string())?;

    let mut output = Vec::with_capacity(BACKUP_MAGIC.len() + 4 + header_bytes.len() + payload.len());
    output.extend_from_slice(BACKUP_MAGIC);
    output.extend_from_slice(&header_len.to_le_bytes());
    output.extend_from_slice(&header_bytes);
    output.extend_from_slice(payload);
    fs::write(path, output).map_err(|e| format!("Impossible d'écrire le backup chiffré: {}", e))
}

fn describe_backup_file(path: &Path) -> Result<BackupDescriptor, String> {
    if path.extension().and_then(|ext| ext.to_str()) == Some(LEGACY_SQLITE_EXTENSION) {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_lowercase();
        return Ok(BackupDescriptor::LegacyPlain {
            automatic: file_name.contains("_auto"),
        });
    }

    let bytes = fs::read(path).map_err(|e| format!("Impossible de lire le backup: {}", e))?;
    let (_, header, _) = parse_backup_envelope(&bytes)?;
    Ok(BackupDescriptor::Encrypted { header })
}

fn parse_backup_envelope(
    bytes: &[u8],
) -> Result<(&[u8], BackupEnvelopeHeader, &[u8]), String> {
    if bytes.len() < BACKUP_MAGIC.len() + 4 || &bytes[..BACKUP_MAGIC.len()] != BACKUP_MAGIC {
        return Err("Format de backup CRVI inconnu.".into());
    }

    let header_len_offset = BACKUP_MAGIC.len();
    let header_len = u32::from_le_bytes([
        bytes[header_len_offset],
        bytes[header_len_offset + 1],
        bytes[header_len_offset + 2],
        bytes[header_len_offset + 3],
    ]) as usize;
    let header_start = BACKUP_MAGIC.len() + 4;
    let header_end = header_start + header_len;
    if bytes.len() < header_end {
        return Err("En-tête de backup tronqué.".into());
    }

    let header: BackupEnvelopeHeader = serde_json::from_slice(&bytes[header_start..header_end])
        .map_err(|e| format!("En-tête de backup invalide: {}", e))?;
    Ok((&bytes[..header_end], header, &bytes[header_end..]))
}

fn decrypt_backup_payload(
    path: &Path,
    header: &BackupEnvelopeHeader,
    passphrase: Option<&str>,
) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|e| format!("Impossible de lire le backup: {}", e))?;
    let (_, _, payload) = parse_backup_envelope(&bytes)?;
    decrypt_snapshot_bytes(payload, header, passphrase)
}

fn encrypt_snapshot_bytes(
    snapshot_bytes: &[u8],
    header: &BackupEnvelopeHeader,
    mode: &BackupMode,
) -> Result<Vec<u8>, String> {
    match mode {
        BackupMode::WindowsLocal => dpapi_protect(snapshot_bytes),
        BackupMode::PortablePassphrase(passphrase) => {
            let salt = decode_b64_required(header.salt_b64.as_deref(), "salt")?;
            let nonce = decode_b64_required(header.nonce_b64.as_deref(), "nonce")?;
            let key = derive_portable_key(passphrase, &salt, header.pbkdf2_iterations)?;
            let cipher = Aes256Gcm::new_from_slice(&key)
                .map_err(|_| "Impossible d'initialiser le chiffrement AES-GCM.".to_string())?;
            cipher
                .encrypt(Nonce::from_slice(&nonce), snapshot_bytes)
                .map_err(|_| "Impossible de chiffrer le backup portable.".to_string())
        }
    }
}

fn decrypt_snapshot_bytes(
    payload: &[u8],
    header: &BackupEnvelopeHeader,
    passphrase: Option<&str>,
) -> Result<Vec<u8>, String> {
    match header.scheme.as_str() {
        WINDOWS_DPAPI_SCHEME => dpapi_unprotect(payload).map_err(|_| {
            "Impossible de déchiffrer ce backup local. Il faut le restaurer avec le même compte Windows sur le poste d'origine."
                .to_string()
        }),
        PORTABLE_PASSPHRASE_SCHEME => {
            let passphrase = passphrase.ok_or_else(|| {
                "Ce backup portable est chiffré par mot de passe. Fournissez la passphrase pour le restaurer."
                    .to_string()
            })?;
            let salt = decode_b64_required(header.salt_b64.as_deref(), "salt")?;
            let nonce = decode_b64_required(header.nonce_b64.as_deref(), "nonce")?;
            let key = derive_portable_key(passphrase, &salt, header.pbkdf2_iterations)?;
            let cipher = Aes256Gcm::new_from_slice(&key)
                .map_err(|_| "Impossible d'initialiser le déchiffrement AES-GCM.".to_string())?;
            cipher
                .decrypt(Nonce::from_slice(&nonce), payload)
                .map_err(|_| {
                    "Impossible de déchiffrer ce backup portable. Vérifiez le mot de passe."
                        .to_string()
                })
        }
        _ => Err("Schéma de chiffrement de backup inconnu.".into()),
    }
}

fn derive_portable_key(
    passphrase: &str,
    salt: &[u8],
    iterations: Option<u32>,
) -> Result<[u8; 32], String> {
    let iterations = iterations.unwrap_or(PBKDF2_ITERATIONS);
    if passphrase.chars().count() < 12 {
        return Err(
            "Le mot de passe du backup portable doit contenir au moins 12 caractères."
                .into(),
        );
    }
    Ok(pbkdf2_hmac_array::<Sha256, 32>(
        passphrase.as_bytes(),
        salt,
        iterations,
    ))
}

fn decode_b64_required(value: Option<&str>, field_name: &str) -> Result<Vec<u8>, String> {
    let value = value.ok_or_else(|| format!("Champ de backup manquant: {field_name}"))?;
    BASE64
        .decode(value)
        .map_err(|e| format!("Champ de backup invalide ({field_name}): {e}"))
}

fn backup_info_from_descriptor(
    path: &Path,
    metadata: &fs::Metadata,
    descriptor: BackupDescriptor,
) -> BackupInfo {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("backup")
        .to_string();
    let modified = metadata.modified().ok().map(|time| {
        chrono::DateTime::<Local>::from(time)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    });

    match descriptor {
        BackupDescriptor::LegacyPlain { automatic } => BackupInfo {
            name,
            path: path.to_string_lossy().to_string(),
            created_at: modified,
            size_bytes: metadata.len(),
            automatic,
            encryption_kind: "legacy-plain".to_string(),
            encryption_label: "Ancien format SQLite non chiffré".to_string(),
            requires_passphrase: false,
            legacy_unencrypted: true,
        },
        BackupDescriptor::Encrypted { header } => BackupInfo {
            name,
            path: path.to_string_lossy().to_string(),
            created_at: Some(header.created_at),
            size_bytes: metadata.len(),
            automatic: header.automatic,
            encryption_kind: header.scheme.clone(),
            encryption_label: encryption_label(&header.scheme).to_string(),
            requires_passphrase: header.scheme == PORTABLE_PASSPHRASE_SCHEME,
            legacy_unencrypted: false,
        },
    }
}

fn encryption_label(scheme: &str) -> &'static str {
    match scheme {
        WINDOWS_DPAPI_SCHEME => "Protégé Windows (DPAPI)",
        PORTABLE_PASSPHRASE_SCHEME => "Portable chiffré (passphrase)",
        _ => "Chiffrement inconnu",
    }
}

fn verify_backup_file(path: &Path) -> Result<bool, String> {
    let conn =
        Connection::open(path).map_err(|e| format!("Impossible d'ouvrir le backup: {}", e))?;
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| format!("Impossible de vérifier le backup: {}", e))?;
    Ok(integrity.eq_ignore_ascii_case("ok"))
}

fn prune_old_backups(dir: &Path) -> Result<(), String> {
    let mut entries: Vec<(PathBuf, String)> = fs::read_dir(dir)
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
                    entry
                        .file_name()
                        .to_str()
                        .unwrap_or_default()
                        .to_string(),
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

fn ensure_backup_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("Impossible de créer le dossier de backup: {}", e))?;
    ensure_recovery_support_files(path)
}

fn ensure_recovery_support_files(path: &Path) -> Result<(), String> {
    let readme_path = path.join(RECOVERY_README_NAME);
    let script_path = path.join(RECOVERY_SCRIPT_NAME);

    fs::write(&readme_path, recovery_readme_contents())
        .map_err(|e| format!("Impossible d'écrire le guide de recovery backup: {}", e))?;
    fs::write(&script_path, recovery_script_contents())
        .map_err(|e| format!("Impossible d'écrire le script de recovery backup: {}", e))?;
    Ok(())
}

fn recovery_readme_contents() -> &'static str {
    "Backups CRVI\n\
\n\
- `*.crvibak` : nouveau format chiffré CRVI.\n\
  - `Protégé Windows (DPAPI)` : restaurable avec le meme compte Windows sur le poste d'origine.\n\
  - `Portable chiffré (passphrase)` : restaurable avec le mot de passe saisi lors de la création.\n\
- `*.sqlite` : ancien format non chiffré conserve pour compatibilite.\n\
\n\
Recovery sans l'application:\n\
1. Ouvrir PowerShell.\n\
2. Aller dans ce dossier.\n\
3. Lancer `./decrypt-crvi-backup.ps1` pour ouvrir le mode guide.\n\
4. Le script liste les backups du dossier et propose un choix numerote.\n\
5. Pour un backup portable, le mot de passe est demande seulement si necessaire.\n\
6. Le script produit un `.sqlite` exploitable ou peut aussi restaurer vers un chemin cible.\n\
\n\
Exemples rapides:\n\
- `./decrypt-crvi-backup.ps1`\n\
- `./decrypt-crvi-backup.ps1 -ListOnly`\n\
- `./decrypt-crvi-backup.ps1 -InputPath .\\nom-du-backup.crvibak`\n\
- `./decrypt-crvi-backup.ps1 -InputPath .\\nom-du-backup.crvibak -RestoreTo C:\\donnees\\crvi.sqlite`\n\
- `./decrypt-crvi-backup.ps1 -Mode Restore`\n\
\n\
Conseils:\n\
- Conserver les backups portables et leur passphrase separement.\n\
- Nettoyer ou migrer les anciens `*.sqlite` non chiffres.\n"
}

fn recovery_script_contents() -> &'static str {
    r#"param(
  [string]$InputPath,
  [string]$OutputPath,
  [string]$RestoreTo,
  [string]$Passphrase,
  [ValidateSet("Export", "Restore", "List")]
  [string]$Mode,
  [switch]$ListOnly
)

$ErrorActionPreference = "Stop"

function Get-ScriptDirectory {
  if ($PSScriptRoot) {
    return $PSScriptRoot
  }
  if ($PSCommandPath) {
    return [System.IO.Path]::GetDirectoryName($PSCommandPath)
  }
  return (Get-Location).Path
}

function Convert-SecureStringToPlainText([System.Security.SecureString]$SecureValue) {
  if ($null -eq $SecureValue) {
    return ""
  }
  $ptr = [System.IntPtr]::Zero
  try {
    $ptr = [System.Runtime.InteropServices.Marshal]::SecureStringToBSTR($SecureValue)
    return [System.Runtime.InteropServices.Marshal]::PtrToStringBSTR($ptr)
  } finally {
    if ($ptr -ne [System.IntPtr]::Zero) {
      [System.Runtime.InteropServices.Marshal]::ZeroFreeBSTR($ptr)
    }
  }
}

function Read-HeaderFromBackup([string]$Path) {
  $magic = [System.Text.Encoding]::ASCII.GetBytes("CRVIBAK1")
  $bytes = [System.IO.File]::ReadAllBytes($Path)
  if ($bytes.Length -lt 12) {
    throw "Fichier trop court."
  }

  for ($i = 0; $i -lt $magic.Length; $i++) {
    if ($bytes[$i] -ne $magic[$i]) {
      throw "Format non reconnu. Ce script ne traite que les backups CRVI .crvibak."
    }
  }

  $headerLength = [System.BitConverter]::ToInt32($bytes, 8)
  $headerStart = 12
  $headerEnd = $headerStart + $headerLength
  if ($bytes.Length -lt $headerEnd) {
    throw "En-tete tronque."
  }

  $headerJson = [System.Text.Encoding]::UTF8.GetString($bytes[$headerStart..($headerEnd - 1)])
  $header = $headerJson | ConvertFrom-Json
  $payloadLength = $bytes.Length - $headerEnd
  $payload = New-Object byte[] $payloadLength
  [System.Array]::Copy($bytes, $headerEnd, $payload, 0, $payloadLength)

  return @{
    Header = $header
    Payload = $payload
  }
}

function Get-BackupLabel([System.IO.FileInfo]$File) {
  if ($File.Extension -ieq ".sqlite") {
    return "SQLite non chiffre"
  }

  try {
    $headerInfo = Read-HeaderFromBackup -Path $File.FullName
    switch ($headerInfo.Header.scheme) {
      "windows-dpapi" { return "Backup CRVI protege Windows" }
      "portable-passphrase" { return "Backup CRVI portable par mot de passe" }
      default { return "Backup CRVI ($($headerInfo.Header.scheme))" }
    }
  } catch {
    return "Backup CRVI illisible"
  }
}

function Get-BackupCandidates([string]$Directory) {
  $patterns = @("*.crvibak", "*.sqlite")
  $items = foreach ($pattern in $patterns) {
    Get-ChildItem -LiteralPath $Directory -File -Filter $pattern -ErrorAction SilentlyContinue
  }

  return $items |
    Sort-Object LastWriteTime -Descending |
    Where-Object { $_.Name -notlike "restore_tmp*" }
}

function Show-BackupList([System.IO.FileInfo[]]$Candidates) {
  if (-not $Candidates -or $Candidates.Count -eq 0) {
    Write-Host "Aucun backup .crvibak ou .sqlite trouve dans ce dossier."
    return
  }

  Write-Host ""
  Write-Host "Backups detectes dans le dossier:" -ForegroundColor Cyan
  for ($i = 0; $i -lt $Candidates.Count; $i++) {
    $candidate = $Candidates[$i]
    $label = Get-BackupLabel -File $candidate
    $sizeMb = [Math]::Round($candidate.Length / 1MB, 2)
    Write-Host ("[{0}] {1} | {2} | {3} MB | {4}" -f ($i + 1), $candidate.Name, $candidate.LastWriteTime.ToString("yyyy-MM-dd HH:mm:ss"), $sizeMb, $label)
  }
  Write-Host ""
}

function Select-BackupInteractively([string]$Directory) {
  $candidates = @(Get-BackupCandidates -Directory $Directory)
  Show-BackupList -Candidates $candidates

  if (-not $candidates -or $candidates.Count -eq 0) {
    throw "Aucun backup disponible."
  }

  while ($true) {
    $answer = Read-Host "Choisir le numero du backup a restaurer"
    $index = 0
    if ([int]::TryParse($answer, [ref]$index) -and $index -ge 1 -and $index -le $candidates.Count) {
      return $candidates[$index - 1].FullName
    }
    Write-Host "Choix invalide. Reessaie avec un numero de la liste." -ForegroundColor Yellow
  }
}

function Select-ActionInteractively {
  Write-Host ""
  Write-Host "Que veux-tu faire ?" -ForegroundColor Cyan
  Write-Host "[1] Lister les backups"
  Write-Host "[2] Exporter un backup en .sqlite"
  Write-Host "[3] Restaurer un backup vers un fichier .sqlite"
  Write-Host ""

  while ($true) {
    $answer = Read-Host "Choisir 1, 2 ou 3"
    switch ($answer) {
      "1" { return "List" }
      "2" { return "Export" }
      "3" { return "Restore" }
      default {
        Write-Host "Choix invalide. Tape 1, 2 ou 3." -ForegroundColor Yellow
      }
    }
  }
}

function Get-PlainBytesFromBackup([string]$ResolvedInputPath, [string]$ProvidedPassphrase) {
  $extension = [System.IO.Path]::GetExtension($ResolvedInputPath)
  if ($extension -ieq ".sqlite") {
    return [System.IO.File]::ReadAllBytes($ResolvedInputPath)
  }

  $headerInfo = Read-HeaderFromBackup -Path $ResolvedInputPath
  $header = $headerInfo.Header
  $payload = $headerInfo.Payload

  switch ($header.scheme) {
    "windows-dpapi" {
      Add-Type -AssemblyName System.Security
      return [System.Security.Cryptography.ProtectedData]::Unprotect(
        $payload,
        $null,
        [System.Security.Cryptography.DataProtectionScope]::CurrentUser
      )
    }
    "portable-passphrase" {
      $effectivePassphrase = $ProvidedPassphrase
      if ([string]::IsNullOrWhiteSpace($effectivePassphrase)) {
        $securePassphrase = Read-Host "Mot de passe du backup portable" -AsSecureString
        $effectivePassphrase = Convert-SecureStringToPlainText -SecureValue $securePassphrase
      }
      if ([string]::IsNullOrWhiteSpace($effectivePassphrase)) {
        throw "Ce backup portable requiert un mot de passe."
      }

      $salt = [Convert]::FromBase64String($header.salt_b64)
      $nonce = [Convert]::FromBase64String($header.nonce_b64)
      $iterations = [int]$header.pbkdf2_iterations
      $derive = [System.Security.Cryptography.Rfc2898DeriveBytes]::new(
        $effectivePassphrase,
        $salt,
        $iterations,
        [System.Security.Cryptography.HashAlgorithmName]::SHA256
      )
      try {
        $key = $derive.GetBytes(32)
      } finally {
        $derive.Dispose()
      }

      $tag = New-Object byte[] 16
      $cipherLength = $payload.Length - 16
      if ($cipherLength -le 0) {
        throw "Payload AES-GCM invalide."
      }
      $cipher = New-Object byte[] $cipherLength
      $plain = New-Object byte[] $cipherLength
      [System.Array]::Copy($payload, 0, $cipher, 0, $cipherLength)
      [System.Array]::Copy($payload, $cipherLength, $tag, 0, 16)

      $aes = [System.Security.Cryptography.AesGcm]::new($key)
      try {
        $aes.Decrypt($nonce, $cipher, $tag, $plain)
        return $plain
      } finally {
        $aes.Dispose()
      }
    }
    default {
      throw "Schema inconnu: $($header.scheme)"
    }
  }
}

function Get-DefaultOutputPath([string]$ResolvedInputPath) {
  $directory = [System.IO.Path]::GetDirectoryName($ResolvedInputPath)
  $baseName = [System.IO.Path]::GetFileNameWithoutExtension($ResolvedInputPath)
  return [System.IO.Path]::Combine($directory, "$baseName.sqlite")
}

function Prompt-RestoreDestination([string]$ResolvedInputPath) {
  $suggestedPath = Get-DefaultOutputPath -ResolvedInputPath $ResolvedInputPath
  $answer = Read-Host "Chemin du fichier .sqlite a restaurer (Entrée = $suggestedPath)"
  if ([string]::IsNullOrWhiteSpace($answer)) {
    return $suggestedPath
  }
  return $answer
}

$scriptDirectory = Get-ScriptDirectory

if ($ListOnly -and [string]::IsNullOrWhiteSpace($Mode)) {
  $Mode = "List"
}

if ([string]::IsNullOrWhiteSpace($Mode) -and [string]::IsNullOrWhiteSpace($InputPath)) {
  $Mode = Select-ActionInteractively
}

if ($Mode -eq "List") {
  $candidates = @(Get-BackupCandidates -Directory $scriptDirectory)
  Show-BackupList -Candidates $candidates
  exit 0
}

if ([string]::IsNullOrWhiteSpace($InputPath)) {
  $InputPath = Select-BackupInteractively -Directory $scriptDirectory
}

$resolvedInput = (Resolve-Path -LiteralPath $InputPath).Path
$plain = Get-PlainBytesFromBackup -ResolvedInputPath $resolvedInput -ProvidedPassphrase $Passphrase

if ($Mode -eq "Restore" -and [string]::IsNullOrWhiteSpace($RestoreTo)) {
  $RestoreTo = Prompt-RestoreDestination -ResolvedInputPath $resolvedInput
}

if (-not [string]::IsNullOrWhiteSpace($RestoreTo)) {
  $destinationDirectory = [System.IO.Path]::GetDirectoryName($RestoreTo)
  if (-not [string]::IsNullOrWhiteSpace($destinationDirectory)) {
    [System.IO.Directory]::CreateDirectory($destinationDirectory) | Out-Null
  }
  [System.IO.File]::WriteAllBytes($RestoreTo, $plain)
  Write-Host "Base restauree vers: $RestoreTo"
  exit 0
}

if ([string]::IsNullOrWhiteSpace($OutputPath)) {
  $OutputPath = Get-DefaultOutputPath -ResolvedInputPath $resolvedInput
}

[System.IO.File]::WriteAllBytes($OutputPath, $plain)
Write-Host "Backup exporte vers: $OutputPath"
"#
}

fn current_db_path(app: &AppHandle) -> Result<PathBuf, String> {
    if let Some(path) = db::current_path() {
        return Ok(PathBuf::from(path));
    }
    if let Some(path) = db::load_path(app) {
        return Ok(PathBuf::from(path));
    }
    Err("Aucune base connectée pour lancer un backup.".into())
}

fn backup_dir_for_db(app: &AppHandle, db_path: &Path) -> Result<PathBuf, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Impossible de déterminer le dossier applicatif: {}", e))?;
    let stem = db_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("crvi");
    let mut hasher = DefaultHasher::new();
    db_path.to_string_lossy().hash(&mut hasher);
    let suffix = hasher.finish();
    Ok(app_data
        .join("backups")
        .join(format!("{stem}_{suffix:016x}")))
}

fn is_supported_backup_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(BACKUP_EXTENSION | LEGACY_SQLITE_EXTENSION)
    )
}

#[cfg(windows)]
fn dpapi_protect(input: &[u8]) -> Result<Vec<u8>, String> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};

    let mut input_blob = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr() as *mut u8,
    };
    let mut output_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    let description: Vec<u16> = "CRVI Backup"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let ok = unsafe {
        CryptProtectData(
            &mut input_blob,
            description.as_ptr(),
            null(),
            null_mut(),
            null_mut(),
            0,
            &mut output_blob,
        )
    };
    if ok == 0 {
        return Err("Windows n'a pas pu chiffrer le backup local.".into());
    }

    let output = unsafe {
        let slice = std::slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize);
        let bytes = slice.to_vec();
        LocalFree(output_blob.pbData.cast());
        bytes
    };
    Ok(output)
}

#[cfg(not(windows))]
fn dpapi_protect(_input: &[u8]) -> Result<Vec<u8>, String> {
    Err("Le chiffrement DPAPI n'est disponible que sous Windows.".into())
}

#[cfg(windows)]
fn dpapi_unprotect(input: &[u8]) -> Result<Vec<u8>, String> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

    let mut input_blob = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr() as *mut u8,
    };
    let mut output_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    let mut description_ptr: *mut u16 = null_mut();

    let ok = unsafe {
        CryptUnprotectData(
            &mut input_blob,
            &mut description_ptr,
            null(),
            null_mut(),
            null_mut(),
            0,
            &mut output_blob,
        )
    };
    if ok == 0 {
        return Err("Windows n'a pas pu déchiffrer le backup local.".into());
    }

    let output = unsafe {
        let slice = std::slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize);
        let bytes = slice.to_vec();
        LocalFree(output_blob.pbData.cast());
        if !description_ptr.is_null() {
            LocalFree(description_ptr.cast());
        }
        bytes
    };
    Ok(output)
}

#[cfg(not(windows))]
fn dpapi_unprotect(_input: &[u8]) -> Result<Vec<u8>, String> {
    Err("Le déchiffrement DPAPI n'est disponible que sous Windows.".into())
}
