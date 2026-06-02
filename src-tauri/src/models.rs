use serde::{Deserialize, Serialize};

// ---- Personne ----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Personne {
    pub id_personne: i64,
    pub civilite: Option<String>,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub email_prive: Option<String>,
    pub telephone_prive: Option<String>,
    pub adresse_privee: Option<String>,
    pub code_postal_prive: Option<String>,
    pub commune_privee: Option<String>,
    pub pays: Option<String>,
    pub consentement_rgpd: bool,
    pub date_consentement: Option<String>,
    pub statut_compte: Option<String>,
    pub notes_commentaires: Option<String>,
    pub date_creation: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersonneInput {
    pub id_personne: Option<i64>,
    pub civilite: Option<String>,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub email_prive: Option<String>,
    pub telephone_prive: Option<String>,
    pub adresse_privee: Option<String>,
    pub code_postal_prive: Option<String>,
    pub commune_privee: Option<String>,
    pub pays: Option<String>,
    pub consentement_rgpd: bool,
    pub date_consentement: Option<String>,
    pub statut_compte: Option<String>,
    pub notes_commentaires: Option<String>,
    pub original_updated_at: Option<String>,
}

// ---- Structure ----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Structure {
    pub id_structure: i64,
    pub nom_structure: Option<String>,
    pub service_specifique: Option<String>,
    pub reseau_subvention: Option<String>,
    pub partenaire_direct: bool,
    pub adresse_structure: Option<String>,
    pub code_postal_structure: Option<String>,
    pub commune_structure: Option<String>,
    pub pays: Option<String>,
    pub telephone_general: Option<String>,
    pub email_general: Option<String>,
    pub site_web: Option<String>,
    pub notes_commentaires: Option<String>,
    pub date_creation: Option<String>,
    pub id_categorie: Option<i64>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StructureInput {
    pub id_structure: Option<i64>,
    pub nom_structure: Option<String>,
    pub service_specifique: Option<String>,
    pub reseau_subvention: Option<String>,
    pub partenaire_direct: bool,
    pub adresse_structure: Option<String>,
    pub code_postal_structure: Option<String>,
    pub commune_structure: Option<String>,
    pub pays: Option<String>,
    pub telephone_general: Option<String>,
    pub email_general: Option<String>,
    pub site_web: Option<String>,
    pub notes_commentaires: Option<String>,
    pub id_categorie: Option<i64>,
    pub original_updated_at: Option<String>,
}

// ---- Categorie ----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Categorie {
    pub id_categorie: i64,
    pub nom_categorie: Option<String>,
}

// ---- Fonction ----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Fonction {
    pub id_fonction: i64,
    pub libelle_fonction: Option<String>,
}

// ---- Affiliation ----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Affiliation {
    pub id_affiliation: i64,
    pub ref_personne: Option<i64>,
    pub ref_structure: Option<i64>,
    pub ref_fonction: Option<i64>,
    pub titre_specifique: Option<String>,
    pub service_specifique: Option<String>,
    pub email_professionnel: Option<String>,
    pub telephone_direct: Option<String>,
    pub gsm_professionnel: Option<String>,
    pub date_debut: Option<String>,
    pub date_fin: Option<String>,
    pub notes_commentaires: Option<String>,
    pub id_categorie: Option<i64>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AffiliationInput {
    pub id_affiliation: Option<i64>,
    pub ref_personne: Option<i64>,
    pub ref_structure: Option<i64>,
    pub ref_fonction: Option<i64>,
    pub titre_specifique: Option<String>,
    pub service_specifique: Option<String>,
    pub email_professionnel: Option<String>,
    pub telephone_direct: Option<String>,
    pub gsm_professionnel: Option<String>,
    pub date_debut: Option<String>,
    pub date_fin: Option<String>,
    pub notes_commentaires: Option<String>,
    pub id_categorie: Option<i64>,
    pub original_updated_at: Option<String>,
}

// ---- Affiliation enrichie (pour l'affichage) ----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AffiliationAvecDetails {
    pub id_affiliation: i64,
    pub ref_personne: Option<i64>,
    pub nom_personne: Option<String>,
    pub prenom_personne: Option<String>,
    pub ref_structure: Option<i64>,
    pub nom_structure: Option<String>,
    pub ref_fonction: Option<i64>,
    pub libelle_fonction: Option<String>,
    pub id_categorie: Option<i64>,
    pub nom_categorie: Option<String>,
    pub titre_specifique: Option<String>,
    pub email_professionnel: Option<String>,
    pub telephone_direct: Option<String>,
    pub gsm_professionnel: Option<String>,
    pub updated_at: Option<String>,
}

// ---- Reunion ----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Reunion {
    pub id_reunion: i64,
    pub titre_reunion: Option<String>,
    pub date_reunion: Option<String>,
    pub heure_reunion: Option<String>,
    pub lieu_reunion: Option<String>,
    pub ref_structure: Option<i64>,
    pub nom_structure: Option<String>,
    pub notes_commentaires: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReunionInput {
    pub id_reunion: Option<i64>,
    pub titre_reunion: Option<String>,
    pub date_reunion: Option<String>,
    pub heure_reunion: Option<String>,
    pub lieu_reunion: Option<String>,
    pub ref_structure: Option<i64>,
    pub notes_commentaires: Option<String>,
    pub original_updated_at: Option<String>,
}

// ---- Presence ----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Presence {
    pub id_presence: i64,
    pub ref_reunion: Option<i64>,
    pub ref_personne: Option<i64>,
    pub statut_presence: Option<String>,
    pub souhaite_rester_en_bdd: bool,
    pub notes_commentaires: Option<String>,
}

// ---- Presence enrichie (pour affichage) ----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PresenceAvecDetails {
    pub id_presence: i64,
    pub ref_personne: Option<i64>,
    pub nom_personne: Option<String>,
    pub prenom_personne: Option<String>,
    pub statut_presence: Option<String>,
    pub souhaite_rester_en_bdd: bool,
    pub notes_commentaires: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PresenceInput {
    pub id_presence: Option<i64>,
    pub ref_reunion: Option<i64>,
    pub ref_personne: Option<i64>,
    pub statut_presence: Option<String>,
    pub souhaite_rester_en_bdd: bool,
    pub notes_commentaires: Option<String>,
}

// ---- Personne catégorie détaillée ----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersonneRefusBDDPresence {
    pub id_personne: i64,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub email_prive: Option<String>,
    pub telephone_prive: Option<String>,
    pub consentement_rgpd: bool,
    pub statut_compte: Option<String>,
    pub reunion_titre: Option<String>,
    pub reunion_date: Option<String>,
}

// ---- Personne avec données détaillées (joins affiliations/structures/fonctions) ----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersonneCategorieDetaillee {
    pub id_personne: i64,
    pub civilite: Option<String>,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub email_prive: Option<String>,
    pub telephone_prive: Option<String>,
    pub adresse_privee: Option<String>,
    pub code_postal_prive: Option<String>,
    pub commune_privee: Option<String>,
    pub pays: Option<String>,
    pub consentement_rgpd: bool,
    pub date_consentement: Option<String>,
    pub statut_compte: Option<String>,
    pub notes_commentaires: Option<String>,
    pub date_creation: Option<String>,
    pub structures: Option<String>,
    pub fonctions: Option<String>,
    pub categories: Option<String>,
    pub emails_pro: Option<String>,
    pub titres: Option<String>,
    pub tels_directs: Option<String>,
    pub gsms_pro: Option<String>,
    pub dates_debut: Option<String>,
    pub dates_fin: Option<String>,
    pub type_entree: Option<String>,
}

// ---- Condition pour query builder ----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Condition {
    pub champ: String,
    pub operateur: String,
    pub valeur: Option<String>,
}

// ---- Preset de filtre QB ----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Preset {
    pub id_preset: i64,
    pub nom_preset: String,
    pub table_principale: String,
    pub colonnes: String,
    pub conditions: String,
    pub date_creation: Option<String>,
    pub date_modification: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PresetInput {
    pub id_preset: Option<i64>,
    pub nom_preset: String,
    pub table_principale: String,
    pub colonnes: String,
    pub conditions: String,
}

// ---- Sécurité ----
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurrentSession {
    pub is_authenticated: bool,
    pub is_anonymous: bool,
    pub anonymous_access_enabled: bool,
    pub must_change_password: bool,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub role_codes: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Permission {
    pub code: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoleDetails {
    pub id_role: i64,
    pub code_role: String,
    pub nom_role: String,
    pub is_system: bool,
    pub permission_codes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoleInput {
    pub id_role: Option<i64>,
    pub nom_role: String,
    pub permission_codes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSummary {
    pub id_user: i64,
    pub username: String,
    pub display_name: Option<String>,
    pub is_active: bool,
    pub must_change_password: bool,
    pub role_ids: Vec<i64>,
    pub role_codes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInput {
    pub id_user: Option<i64>,
    pub username: String,
    pub display_name: Option<String>,
    pub is_active: bool,
    pub must_change_password: bool,
    pub role_ids: Vec<i64>,
    pub password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PasswordChangeInput {
    pub user_id: i64,
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OwnPasswordChangeInput {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecuritySettings {
    pub anonymous_access_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExcelRebuildResult {
    pub path: String,
    pub workbook_name: String,
    pub sheet_count: usize,
    pub row_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EditLockStatus {
    pub resource_type: String,
    pub resource_id: i64,
    pub acquired: bool,
    pub holder_label: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackupInfo {
    pub name: String,
    pub path: String,
    pub created_at: Option<String>,
    pub size_bytes: u64,
    pub automatic: bool,
    pub encryption_kind: String,
    pub encryption_label: String,
    pub requires_passphrase: bool,
    pub legacy_unencrypted: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ManualBackupRequest {
    pub portable: bool,
    pub passphrase: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RestoreBackupRequest {
    pub backup_path: String,
    pub passphrase: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BackupRunStatus {
    Created,
    Skipped,
    Restored,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackupRunResult {
    pub status: BackupRunStatus,
    pub message: String,
    pub backup: Option<BackupInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatsFilters {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatsBucket {
    pub key: String,
    pub label: String,
    pub value: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatsTopMeeting {
    pub label: String,
    pub value: i64,
    pub date_reunion: Option<String>,
    pub organisme: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatsParticipation {
    pub key: String,
    pub label: String,
    pub invitations: i64,
    pub presents: i64,
    pub presence_rate: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DashboardOverview {
    pub total_contacts: i64,
    pub active_contacts: i64,
    pub anonymized_contacts: i64,
    pub contacts_to_delete: i64,
    pub total_structures: i64,
    pub total_categories: i64,
    pub total_affiliations: i64,
    pub total_reunions: i64,
    pub period_reunions: i64,
    pub period_presences: i64,
    pub average_presences_per_reunion: f64,
    pub rgpd_consent_rate: f64,
    pub partner_direct_structures: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DashboardStats {
    pub generated_at: String,
    pub available_start_date: Option<String>,
    pub available_end_date: Option<String>,
    pub overview: DashboardOverview,
    pub meetings_by_month: Vec<StatsBucket>,
    pub attendance_by_status: Vec<StatsBucket>,
    pub structures_by_category: Vec<StatsBucket>,
    pub contacts_by_commune: Vec<StatsBucket>,
    pub structures_by_commune: Vec<StatsBucket>,
    pub meetings_by_organisme: Vec<StatsBucket>,
    pub account_statuses: Vec<StatsBucket>,
    pub contacts_created_by_month: Vec<StatsBucket>,
    pub quality_checks: Vec<StatsBucket>,
    pub top_meetings: Vec<StatsTopMeeting>,
    pub top_structures_presence_rate: Vec<StatsParticipation>,
    pub top_structures_presence_volume: Vec<StatsParticipation>,
    pub top_people_presence: Vec<StatsParticipation>,
}
