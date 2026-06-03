export interface QBFieldEntry {
  key: string;
  label: string;
}

export interface QBFieldGroup {
  alias: string;
  table: string;
  fields: QBFieldEntry[];
}

export interface QBTableEntry {
  key: string;
  label: string;
  alias: string;
}

export interface QBOperator {
  key: string;
  label: string;
}

export const QB_FIELDS: QBFieldGroup[] = [
  { alias: "p", table: "Personnes", fields: [
    { key: "p.Nom", label: "Nom" }, { key: "p.Prenom", label: "Prénom" }, { key: "p.Civilite", label: "Civilité" },
    { key: "p.Email_Prive", label: "Email privé" }, { key: "p.Telephone_Prive", label: "Téléphone privé" },
    { key: "p.Adresse_Privee", label: "Adresse" }, { key: "p.Code_Postal_Prive", label: "Code postal" },
    { key: "p.Commune_Privee", label: "Commune" }, { key: "p.Pays", label: "Pays" },
    { key: "p.Statut_Compte", label: "Statut" }, { key: "p.Consentement_RGPD", label: "RGPD" },
    { key: "p.Date_Consentement", label: "Date consentement" }, { key: "p.Notes_Commentaires", label: "Notes" },
    { key: "p.Date_Creation", label: "Date création" },
  ]},
  { alias: "s", table: "Structures", fields: [
    { key: "s.Nom_Structure", label: "Nom" }, { key: "s.Service_Specifique", label: "Service" },
    { key: "s.Reseau_Subvention", label: "Réseau subvention" }, { key: "s.Partenaire_Direct", label: "Partenaire direct" },
    { key: "s.Adresse_Structure", label: "Adresse" }, { key: "s.Code_Postal_Structure", label: "Code postal" },
    { key: "s.Commune_Structure", label: "Commune" }, { key: "s.Pays", label: "Pays" },
    { key: "s.Telephone_General", label: "Téléphone" }, { key: "s.Email_General", label: "Email" },
    { key: "s.Site_Web", label: "Site web" }, { key: "s.Notes_Commentaires", label: "Notes" },
  ]},
  { alias: "a", table: "Affiliations", fields: [
    { key: "a.Titre_Specifique", label: "Titre spécifique" }, { key: "a.Service_Specifique", label: "Service" },
    { key: "a.Email_Professionnel", label: "Email professionnel" }, { key: "a.Telephone_Direct", label: "Téléphone direct" },
    { key: "a.Gsm_Professionnel", label: "GSM professionnel" }, { key: "a.Date_Debut", label: "Date début" },
    { key: "a.Date_Fin", label: "Date fin" }, { key: "a.Notes_Commentaires", label: "Notes" },
  ]},
  { alias: "c", table: "Catégories", fields: [
    { key: "c.Nom_Categorie", label: "Nom catégorie" },
  ]},
  { alias: "f", table: "Fonctions", fields: [
    { key: "f.Libelle_Fonction", label: "Libellé fonction" },
  ]},
  { alias: "r", table: "Réunions", fields: [
    { key: "r.Titre_Reunion", label: "Titre" }, { key: "r.Date_Reunion", label: "Date" },
    { key: "r.Heure_Reunion", label: "Heure" }, { key: "r.Lieu_Reunion", label: "Lieu" },
    { key: "r.Notes_Commentaires", label: "Notes" },
  ]},
  { alias: "pr", table: "Présences", fields: [
    { key: "pr.Statut_Presence", label: "Statut" }, { key: "pr.Souhaite_Rester_En_BDD", label: "Reste en BDD" },
    { key: "pr.Notes_Commentaires", label: "Notes" },
  ]},
];

export const QB_TABLES: QBTableEntry[] = [
  { key: "personnes", label: "Personnes", alias: "p" },
  { key: "structures", label: "Structures", alias: "s" },
  { key: "affiliations", label: "Affiliations", alias: "a" },
  { key: "reunions", label: "Réunions", alias: "r" },
  { key: "presences", label: "Présences", alias: "pr" },
];

export const QB_OPERATORS: QBOperator[] = [
  { key: "=", label: "=" }, { key: "!=", label: "≠" },
  { key: ">", label: ">" }, { key: "<", label: "<" },
  { key: ">=", label: "≥" }, { key: "<=", label: "≤" },
  { key: "LIKE", label: "contient" },
  { key: "commence_par", label: "commence par" },
  { key: "finit_par", label: "finit par" },
  { key: "IS NULL", label: "est vide" },
  { key: "IS NOT NULL", label: "n'est pas vide" },
  { key: "is_duplicate", label: "est un doublon" },
  { key: "is_not_duplicate", label: "est unique" },
  { key: "in_list", label: "dans la liste" },
  { key: "not_in_list", label: "pas dans la liste" },
  { key: "between", label: "entre" },
];
