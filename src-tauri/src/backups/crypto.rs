use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use pbkdf2::pbkdf2_hmac_array;
use sha2::Sha256;
use std::path::Path;
use std::fs;
use super::envelope::{BackupEnvelopeHeader, BackupMode, parse_backup_envelope};
use super::dpapi::*;

pub const WINDOWS_DPAPI_SCHEME: &str = "windows-dpapi";
pub const PORTABLE_PASSPHRASE_SCHEME: &str = "portable-passphrase";
pub const PBKDF2_ITERATIONS: u32 = 600_000;

pub fn decrypt_backup_payload(
    path: &Path,
    header: &BackupEnvelopeHeader,
    passphrase: Option<&str>,
) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|e| format!("Impossible de lire le backup: {}", e))?;
    let (_, _, payload) = parse_backup_envelope(&bytes)?;
    decrypt_snapshot_bytes(payload, header, passphrase)
}

pub fn encrypt_snapshot_bytes(
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

pub fn decrypt_snapshot_bytes(
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

pub fn derive_portable_key(
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

pub fn decode_b64_required(value: Option<&str>, field_name: &str) -> Result<Vec<u8>, String> {
    let value = value.ok_or_else(|| format!("Champ de backup manquant: {field_name}"))?;
    BASE64
        .decode(value)
        .map_err(|e| format!("Champ de backup invalide ({field_name}): {e}"))
}

pub fn encryption_label(scheme: &str) -> &'static str {
    match scheme {
        WINDOWS_DPAPI_SCHEME => "Protégé Windows (DPAPI)",
        PORTABLE_PASSPHRASE_SCHEME => "Portable chiffré (passphrase)",
        _ => "Chiffrement inconnu",
    }
}
