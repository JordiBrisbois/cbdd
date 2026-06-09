use crate::auth;
use crate::backups;
use crate::db;
use crate::excel_export;
use crate::models::*;
use chrono::Utc;
use once_cell::sync::Lazy;
use rand_core::{OsRng, RngCore};
use rusqlite::OptionalExtension;
use tauri::AppHandle;

mod presets;
mod query_builder;

#[tauri::command]
pub fn executer_requete(
    app: AppHandle,
    table_principale: String,
    colonnes: Vec<String>,
    conditions: Vec<Condition>,
) -> Result<Vec<serde_json::Value>, String> {
    query_builder::executer_requete_impl(app, table_principale, colonnes, conditions)
}

#[tauri::command]
pub fn lister_presets(app: AppHandle) -> Result<Vec<Preset>, String> {
    presets::lister_presets_impl(app)
}

#[tauri::command]
pub fn sauvegarder_preset(app: AppHandle, preset: PresetInput) -> Result<Preset, String> {
    presets::sauvegarder_preset_impl(app, preset)
}

#[tauri::command]
pub fn supprimer_preset(app: AppHandle, id: i64) -> Result<(), String> {
    presets::supprimer_preset_impl(app, id)
}

#[tauri::command]
pub fn charger_preset(app: AppHandle, id: i64) -> Result<Preset, String> {
    presets::charger_preset_impl(app, id)
}

pub mod affiliations;
pub mod auth_commands;
pub mod backup_commands;
pub mod categories;
pub mod db_commands;
pub mod fonctions;
pub mod locks;
pub mod personnes;
pub mod presences;
pub mod reunions;
pub mod rgpd;
pub mod search;
pub mod stats;
pub mod structures;

pub use affiliations::*;
pub use auth_commands::*;
pub use backup_commands::*;
pub use categories::*;
pub use db_commands::*;
pub use fonctions::*;
pub use locks::*;
pub use personnes::*;
pub use presences::*;
pub use reunions::*;
pub use rgpd::*;
pub use search::*;
pub use stats::*;
pub use structures::*;

pub const ANONYMIZED_STATUS: &str = "Anonymisé";
pub const ANONYMIZED_LABEL: &str = "Participant anonymisé";
pub const EDIT_LOCK_MINUTES: i64 = 10;

static EDIT_LOCK_TOKEN: Lazy<String> = Lazy::new(|| {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
});

pub fn edit_lock_token() -> &'static str {
    EDIT_LOCK_TOKEN.as_str()
}

pub fn machine_label() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "Poste inconnu".to_string())
}

pub fn normalize_person_last_name(value: Option<String>) -> Option<String> {
    value
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
        .map(|raw| raw.to_uppercase())
}

pub fn rgpd_attention_predicate(alias: &str) -> String {
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

pub fn permission_for_resource(resource_type: &str) -> Option<&'static str> {
    match resource_type {
        "personnes" => Some("personnes.update"),
        "structures" => Some("structures.update"),
        "affiliations" => Some("affiliations.update"),
        "reunions" => Some("reunions.update"),
        _ => None,
    }
}

pub fn ensure_resource_not_locked_by_other(
    conn: &rusqlite::Connection,
    resource_type: &str,
    resource_id: i64,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM T_EditLocks WHERE Expires_At <= datetime('now')",
        [],
    )
    .map_err(|e| e.to_string())?;

    let token = edit_lock_token();
    let existing = conn
        .query_row(
            "SELECT Holder_Token, Holder_Label
             FROM T_EditLocks
             WHERE Resource_Type = ? AND Resource_Id = ?",
            rusqlite::params![resource_type, resource_id],
            |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if let Some((existing_token, existing_label)) = existing {
        if existing_token.as_deref() != Some(token) {
            return Err(format!(
                "Cette fiche est actuellement verrouillée par {}. Fermez ou laissez expirer l'édition avant de poursuivre.",
                existing_label
            ));
        }
    }

    Ok(())
}

pub fn ensure_affiliation_context_mutable(
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

pub fn ensure_presence_context_mutable(
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

pub fn anonymize_person(conn: &rusqlite::Connection, personne_id: i64) -> Result<(), String> {
    ensure_resource_not_locked_by_other(conn, "personnes", personne_id)?;
    crate::services::people::anonymize_person(conn, personne_id)
}

// ── ROW MAPPERS ──

pub fn map_personne(row: &rusqlite::Row) -> rusqlite::Result<Personne> {
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

pub fn map_structure(row: &rusqlite::Row) -> rusqlite::Result<Structure> {
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

pub fn map_categorie(row: &rusqlite::Row) -> rusqlite::Result<Categorie> {
    Ok(Categorie {
        id_categorie: row.get("ID_Categorie")?,
        nom_categorie: row.get("Nom_Categorie")?,
    })
}

pub fn map_fonction(row: &rusqlite::Row) -> rusqlite::Result<Fonction> {
    Ok(Fonction {
        id_fonction: row.get("ID_Fonction")?,
        libelle_fonction: row.get("Libelle_Fonction")?,
    })
}

pub fn map_reunion(row: &rusqlite::Row) -> rusqlite::Result<Reunion> {
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

pub fn append_date_filters(
    clauses: &mut Vec<String>,
    params: &mut Vec<String>,
    column: &str,
    filters: &StatsFilters,
) {
    if let Some(start) = filters
        .start_date
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        clauses.push(format!("date({}) >= date(?)", column));
        params.push(start.to_string());
    }

    if let Some(end) = filters
        .end_date
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        clauses.push(format!("date({}) <= date(?)", column));
        params.push(end.to_string());
    }
}

pub fn sqlite_date_expr(column: &str) -> String {
    format!(
        "CASE
            WHEN instr({column}, '/') > 0 THEN date('20' || substr({column}, 7, 2) || '-' || substr({column}, 1, 2) || '-' || substr({column}, 4, 2))
            ELSE date({column})
         END"
    )
}

pub fn sqlite_month_expr(column: &str) -> String {
    format!(
        "CASE
            WHEN instr({column}, '/') > 0 THEN '20' || substr({column}, 7, 2) || '-' || substr({column}, 1, 2)
            ELSE substr({column}, 1, 7)
         END"
    )
}

pub fn build_where_sql(clauses: &[String]) -> String {
    if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    }
}

pub fn get_query_meta_select_parts(main_alias: &str) -> Vec<String> {
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

pub fn query_i64(
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

pub fn query_f64(
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

pub fn load_stats_buckets(
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

pub fn load_top_meetings(
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

pub fn load_stats_participation(
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

pub fn merge_casefolded_buckets(items: Vec<StatsBucket>) -> Vec<StatsBucket> {
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

pub fn is_better_bucket_label(candidate: &str, current: &str) -> bool {
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
