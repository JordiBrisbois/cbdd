use crate::models::BackupInfo;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const BACKUP_MAGIC: &[u8; 8] = b"CRVIBAK1";
pub const LEGACY_SQLITE_EXTENSION: &str = "sqlite";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEnvelopeHeader {
    pub version: u8,
    pub scheme: String,
    pub created_at: String,
    pub automatic: bool,
    pub salt_b64: Option<String>,
    pub nonce_b64: Option<String>,
    pub pbkdf2_iterations: Option<u32>,
}

#[derive(Debug, Clone)]
pub enum BackupMode {
    WindowsLocal,
    PortablePassphrase(String),
}

#[derive(Debug, Clone)]
pub enum BackupDescriptor {
    LegacyPlain { automatic: bool },
    Encrypted { header: BackupEnvelopeHeader },
}

pub fn write_backup_envelope(
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

    let mut output =
        Vec::with_capacity(BACKUP_MAGIC.len() + 4 + header_bytes.len() + payload.len());
    output.extend_from_slice(BACKUP_MAGIC);
    output.extend_from_slice(&header_len.to_le_bytes());
    output.extend_from_slice(&header_bytes);
    output.extend_from_slice(payload);
    fs::write(path, output).map_err(|e| format!("Impossible d'écrire le backup chiffré: {}", e))
}

pub fn parse_backup_envelope(bytes: &[u8]) -> Result<(&[u8], BackupEnvelopeHeader, &[u8]), String> {
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

pub fn describe_backup_file(path: &Path) -> Result<BackupDescriptor, String> {
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

pub fn backup_info_from_descriptor(
    path: &Path,
    metadata: &fs::Metadata,
    descriptor: BackupDescriptor,
) -> BackupInfo {
    use super::crypto::encryption_label;

    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("backup")
        .to_string();
    let modified = metadata.modified().ok().map(|time| {
        chrono::DateTime::<chrono::Local>::from(time)
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
            requires_passphrase: header.scheme == super::crypto::PORTABLE_PASSPHRASE_SCHEME,
            legacy_unencrypted: false,
        },
    }
}
