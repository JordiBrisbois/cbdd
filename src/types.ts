declare global {
  interface Window {
    __TAURI_INTERNALS__?: Record<string, unknown>;
  }
}

export interface Personne {
  id_personne: number;
  civilite: string | null;
  nom: string | null;
  prenom: string | null;
  email_prive: string | null;
  telephone_prive: string | null;
  adresse_privee: string | null;
  code_postal_prive: string | null;
  commune_privee: string | null;
  pays: string | null;
  consentement_rgpd: boolean;
  date_consentement: string | null;
  statut_compte: string | null;
  notes_commentaires: string | null;
  date_creation: string | null;
  updated_at: string | null;
}

export interface PersonneInput {
  id_personne?: number | null;
  civilite?: string | null;
  nom?: string | null;
  prenom?: string | null;
  email_prive?: string | null;
  telephone_prive?: string | null;
  adresse_privee?: string | null;
  code_postal_prive?: string | null;
  commune_privee?: string | null;
  pays?: string | null;
  consentement_rgpd: boolean;
  date_consentement?: string | null;
  statut_compte?: string | null;
  notes_commentaires?: string | null;
  original_updated_at?: string | null;
}

export interface Structure {
  id_structure: number;
  nom_structure: string | null;
  service_specifique: string | null;
  reseau_subvention: string | null;
  partenaire_direct: boolean;
  adresse_structure: string | null;
  code_postal_structure: string | null;
  commune_structure: string | null;
  pays: string | null;
  telephone_general: string | null;
  email_general: string | null;
  site_web: string | null;
  notes_commentaires: string | null;
  date_creation: string | null;
  id_categorie: number | null;
  updated_at: string | null;
}

export interface StructureInput {
  id_structure?: number | null;
  nom_structure?: string | null;
  service_specifique?: string | null;
  reseau_subvention?: string | null;
  partenaire_direct: boolean;
  adresse_structure?: string | null;
  code_postal_structure?: string | null;
  commune_structure?: string | null;
  pays?: string | null;
  telephone_general?: string | null;
  email_general?: string | null;
  site_web?: string | null;
  notes_commentaires?: string | null;
  id_categorie?: number | null;
  original_updated_at?: string | null;
}

export interface Categorie {
  id_categorie: number;
  nom_categorie: string | null;
}

export interface Fonction {
  id_fonction: number;
  libelle_fonction: string | null;
}

export interface AffiliationAvecDetails {
  id_affiliation: number;
  ref_personne: number | null;
  nom_personne: string | null;
  prenom_personne: string | null;
  ref_structure: number | null;
  nom_structure: string | null;
  ref_fonction: number | null;
  libelle_fonction: string | null;
  id_categorie: number | null;
  nom_categorie: string | null;
  titre_specifique: string | null;
  email_professionnel: string | null;
  telephone_direct: string | null;
  gsm_professionnel: string | null;
  updated_at: string | null;
}

export interface AffiliationInput {
  id_affiliation?: number | null;
  ref_personne?: number | null;
  ref_structure?: number | null;
  ref_fonction?: number | null;
  titre_specifique?: string | null;
  service_specifique?: string | null;
  email_professionnel?: string | null;
  telephone_direct?: string | null;
  gsm_professionnel?: string | null;
  date_debut?: string | null;
  date_fin?: string | null;
  notes_commentaires?: string | null;
  id_categorie?: number | null;
  original_updated_at?: string | null;
}

export interface Reunion {
  id_reunion: number;
  titre_reunion: string | null;
  date_reunion: string | null;
  heure_reunion: string | null;
  lieu_reunion: string | null;
  ref_structure: number | null;
  nom_structure?: string | null;
  notes_commentaires: string | null;
  updated_at: string | null;
}

export interface ReunionInput {
  id_reunion?: number | null;
  titre_reunion?: string | null;
  date_reunion?: string | null;
  heure_reunion?: string | null;
  lieu_reunion?: string | null;
  ref_structure?: number | null;
  notes_commentaires?: string | null;
  original_updated_at?: string | null;
}

export interface PresenceAvecDetails {
  id_presence: number;
  ref_personne: number | null;
  nom_personne: string | null;
  prenom_personne: string | null;
  statut_presence: string | null;
  souhaite_rester_en_bdd: boolean;
  notes_commentaires: string | null;
}

export interface PresenceInput {
  id_presence?: number | null;
  ref_reunion?: number | null;
  ref_personne?: number | null;
  statut_presence?: string | null;
  souhaite_rester_en_bdd: boolean;
  notes_commentaires?: string | null;
}

export interface DBStatus {
  connected: boolean;
  path: string | null;
  error?: string | null;
}

export interface PersonneRefusBDDPresence {
  id_personne: number;
  nom: string | null;
  prenom: string | null;
  email_prive: string | null;
  telephone_prive: string | null;
  consentement_rgpd: boolean;
  statut_compte: string | null;
  reunion_titre: string | null;
  reunion_date: string | null;
}

export interface PersonneCategorieDetaillee {
  id_personne: number;
  civilite: string | null;
  nom: string | null;
  prenom: string | null;
  email_prive: string | null;
  telephone_prive: string | null;
  adresse_privee: string | null;
  code_postal_prive: string | null;
  commune_privee: string | null;
  pays: string | null;
  consentement_rgpd: boolean;
  date_consentement: string | null;
  statut_compte: string | null;
  notes_commentaires: string | null;
  date_creation: string | null;
  structures: string | null;
  fonctions: string | null;
  categories: string | null;
  emails_pro: string | null;
  titres: string | null;
  tels_directs: string | null;
  gsms_pro: string | null;
  dates_debut: string | null;
  dates_fin: string | null;
  type_entree?: string | null;
}

export interface Condition {
  champ: string;
  operateur: string;
  valeur: string | null;
}

export interface Preset {
  id_preset: number;
  nom_preset: string;
  table_principale: string;
  colonnes: string;
  conditions: string;
  date_creation: string | null;
  date_modification: string | null;
}

export interface CurrentSession {
  is_authenticated: boolean;
  is_anonymous: boolean;
  anonymous_access_enabled: boolean;
  must_change_password: boolean;
  user_id: number | null;
  username: string | null;
  display_name: string | null;
  role_codes: string[];
  permissions: string[];
}

export interface Permission {
  code: string;
  description: string;
}

export interface RoleDetails {
  id_role: number;
  code_role: string;
  nom_role: string;
  is_system: boolean;
  permission_codes: string[];
}

export interface RoleInput {
  id_role?: number | null;
  nom_role: string;
  permission_codes: string[];
}

export interface UserSummary {
  id_user: number;
  username: string;
  display_name: string | null;
  is_active: boolean;
  must_change_password: boolean;
  role_ids: number[];
  role_codes: string[];
}

export interface UserInput {
  id_user?: number | null;
  username: string;
  display_name?: string | null;
  is_active: boolean;
  must_change_password: boolean;
  role_ids: number[];
  password?: string | null;
}

export interface LoginInput {
  username: string;
  password: string;
}

export interface PasswordChangeInput {
  user_id: number;
  new_password: string;
}

export interface OwnPasswordChangeInput {
  current_password: string;
  new_password: string;
}

export interface SecuritySettings {
  anonymous_access_enabled: boolean;
}

export interface ExcelRebuildResult {
  path: string;
  workbook_name: string;
  sheet_count: number;
  row_count: number;
}

export interface EditLockStatus {
  resource_type: string;
  resource_id: number;
  acquired: boolean;
  holder_label: string | null;
  expires_at: string | null;
}

export interface BackupInfo {
  name: string;
  path: string;
  created_at: string | null;
  size_bytes: number;
  automatic: boolean;
  encryption_kind: string;
  encryption_label: string;
  requires_passphrase: boolean;
  legacy_unencrypted: boolean;
}

export interface ManualBackupRequest {
  portable: boolean;
  passphrase?: string | null;
}

export interface RestoreBackupRequest {
  backup_path: string;
  passphrase?: string | null;
}

export type BackupRunStatus = "Created" | "Skipped" | "Restored";

export interface BackupRunResult {
  status: BackupRunStatus;
  message: string;
  backup: BackupInfo | null;
}

export interface StatsFilters {
  start_date?: string | null;
  end_date?: string | null;
}

export interface StatsBucket {
  key: string;
  label: string;
  value: number;
}

export interface StatsTopMeeting {
  label: string;
  value: number;
  date_reunion: string | null;
  organisme: string | null;
}

export interface StatsParticipation {
  key: string;
  label: string;
  invitations: number;
  presents: number;
  presence_rate: number;
}

export interface DashboardOverview {
  total_contacts: number;
  active_contacts: number;
  anonymized_contacts: number;
  contacts_to_delete: number;
  total_structures: number;
  total_categories: number;
  total_affiliations: number;
  total_reunions: number;
  period_reunions: number;
  period_presences: number;
  average_presences_per_reunion: number;
  rgpd_consent_rate: number;
  partner_direct_structures: number;
}

export interface DashboardStats {
  generated_at: string;
  available_start_date: string | null;
  available_end_date: string | null;
  overview: DashboardOverview;
  meetings_by_month: StatsBucket[];
  attendance_by_status: StatsBucket[];
  structures_by_category: StatsBucket[];
  contacts_by_commune: StatsBucket[];
  structures_by_commune: StatsBucket[];
  meetings_by_organisme: StatsBucket[];
  account_statuses: StatsBucket[];
  contacts_created_by_month: StatsBucket[];
  quality_checks: StatsBucket[];
  top_meetings: StatsTopMeeting[];
  top_structures_presence_rate: StatsParticipation[];
  top_structures_presence_volume: StatsParticipation[];
  top_people_presence: StatsParticipation[];
}

export type Page = 'contacts' | 'structures' | 'categories' | 'reunions' | 'rgpd' | 'stats' | 'search' | 'admin';
