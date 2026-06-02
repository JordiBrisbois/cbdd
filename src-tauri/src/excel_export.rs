use crate::models::ExcelRebuildResult;
use chrono::Local;
use rusqlite::Connection;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

const HISTORICAL_SHEETS: &[SheetSpec] = &[
    SheetSpec::new("AG", "AG", SheetLayout::SplitName, "Institution"),
    SheetSpec::new("CA", "CA", SheetLayout::SplitName, "Institution"),
    SheetSpec::new(
        "Assocs hors arrondissement",
        "Assocs hors arrondissement",
        SheetLayout::Standard,
        "Institution",
    ),
    SheetSpec::new(
        "Bénévoles et Particuliers",
        "Bénévoles & Particuliers",
        SheetLayout::ContactOnly,
        "",
    ),
    SheetSpec::new("CPAS", "CPAS", SheetLayout::Standard, "Institution"),
    SheetSpec::new(
        "Villes et communes Politiques",
        "Villes et communes Politiques",
        SheetLayout::Standard,
        "Institution",
    ),
    SheetSpec::new(
        "Villes et communes travailleurs",
        "Villes et Communes travailleurs",
        SheetLayout::Standard,
        "Institution",
    ),
    SheetSpec::new("CRI", "CRI", SheetLayout::Standard, "Institution"),
    SheetSpec::new("CRVI", "CRVI", SheetLayout::Crvi, "Institution"),
    SheetSpec::new(
        "Culture & Loisirs",
        "Culture & Loisirs",
        SheetLayout::Standard,
        "Institution",
    ),
    SheetSpec::new("Divers", "Divers", SheetLayout::Standard, "Institution"),
    SheetSpec::new(
        "Enseignement",
        "Enseignement",
        SheetLayout::Standard,
        "Institution",
    ),
    SheetSpec::new(
        "Ecrivains publics",
        "Ecrivains publics",
        SheetLayout::EcrivainsPublics,
        "Institution",
    ),
    SheetSpec::new("ILI", "ILI", SheetLayout::Standard, "Institution"),
    SheetSpec::new(
        "Intégration & Social Verviers",
        "Intégration & Social Verviers",
        SheetLayout::Standard,
        "Institution",
    ),
    SheetSpec::new(
        "ISP-CISP-Emploi",
        "ISP-CISP-Emploi",
        SheetLayout::Standard,
        "Institution",
    ),
    SheetSpec::new("Jeunesse", "Jeunesse", SheetLayout::Standard, "Institution"),
    SheetSpec::new("PCS", "PCS", SheetLayout::Standard, "Institution"),
    SheetSpec::new(
        "Politique & Syndicats",
        "Politique & Syndicats",
        SheetLayout::Standard,
        "Parti",
    ),
    SheetSpec::new("Presse", "Presse", SheetLayout::Standard, "Média"),
    SheetSpec::new(
        "Région Wallonne",
        "Région Wallonne",
        SheetLayout::Standard,
        "Institution",
    ),
    SheetSpec::new("Santé", "Santé", SheetLayout::Standard, "Institution"),
];

#[derive(Debug, Clone, Copy)]
struct SheetSpec {
    category_name: &'static str,
    sheet_title: &'static str,
    layout: SheetLayout,
    first_label: &'static str,
}

impl SheetSpec {
    const fn new(
        category_name: &'static str,
        sheet_title: &'static str,
        layout: SheetLayout,
        first_label: &'static str,
    ) -> Self {
        Self {
            category_name,
            sheet_title,
            layout,
            first_label,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SheetLayout {
    Standard,
    SplitName,
    ContactOnly,
    EcrivainsPublics,
    Crvi,
}

#[derive(Debug, Clone)]
struct ExportRow {
    institution: String,
    group_label: String,
    civilite: String,
    nom: String,
    prenom: String,
    full_name: String,
    fonction: String,
    email: String,
    telephone: String,
    adresse: String,
    commune: String,
    autre: String,
    is_structure_only: bool,
}

pub fn export_reconstructed_excel(
    app: &AppHandle,
    conn: &Connection,
) -> Result<Option<ExcelRebuildResult>, String> {
    let default_name = format!(
        "CRVI_reconstruction_categories_{}.xlsx",
        Local::now().format("%Y-%m-%d")
    );

    let Some(file_path) = app
        .dialog()
        .file()
        .add_filter("Classeur Excel", &["xlsx"])
        .set_file_name(&default_name)
        .blocking_save_file()
    else {
        return Ok(None);
    };

    let output_path = file_path
        .into_path()
        .map_err(|e| format!("Chemin d'export invalide: {}", e))?;

    let result = export_reconstructed_excel_to_path(conn, &output_path)?;

    Ok(Some(result))
}

pub fn export_reconstructed_excel_to_path(
    conn: &Connection,
    output_path: &Path,
) -> Result<ExcelRebuildResult, String> {
    let data = build_export_data(conn)?;
    write_workbook(output_path, &data)
}

fn build_export_data(conn: &Connection) -> Result<BTreeMap<String, Vec<ExportRow>>, String> {
    let mut grouped: BTreeMap<String, Vec<ExportRow>> = HISTORICAL_SHEETS
        .iter()
        .map(|sheet| (sheet.category_name.to_string(), Vec::new()))
        .collect();

    let mut stmt = conn
        .prepare(
            "
            SELECT
                c.Nom_Categorie,
                s.Nom_Structure,
                p.Civilite,
                p.Nom,
                p.Prenom,
                f.Libelle_Fonction,
                a.Titre_Specifique,
                a.Service_Specifique,
                a.Email_Professionnel,
                a.Telephone_Direct,
                a.Gsm_Professionnel,
                a.Notes_Commentaires,
                p.Email_Prive,
                p.Telephone_Prive,
                p.Adresse_Privee,
                p.Code_Postal_Prive,
                p.Commune_Privee,
                p.Notes_Commentaires,
                s.Adresse_Structure,
                s.Code_Postal_Structure,
                s.Commune_Structure,
                s.Email_General,
                s.Telephone_General,
                s.Service_Specifique,
                s.Reseau_Subvention,
                s.Notes_Commentaires
            FROM T_Affiliations a
            LEFT JOIN T_Categories c ON c.ID_Categorie = a.ID_Categorie
            LEFT JOIN T_Personnes p ON p.ID_Personne = a.Ref_Personne
            LEFT JOIN T_Structures s ON s.ID_Structure = a.Ref_Structure
            LEFT JOIN T_Fonctions f ON f.ID_Fonction = a.Ref_Fonction
            WHERE c.Nom_Categorie IS NOT NULL
              AND (p.ID_Personne IS NULL OR COALESCE(p.Statut_Compte, '') != 'Anonymisé')
            ORDER BY c.Nom_Categorie ASC, s.Nom_Structure ASC, p.Nom ASC, p.Prenom ASC
            ",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let category_name: String = row.get(0)?;
            let structure_name: Option<String> = row.get(1)?;
            let civilite: Option<String> = row.get(2)?;
            let nom: Option<String> = row.get(3)?;
            let prenom: Option<String> = row.get(4)?;
            let fonction: Option<String> = row.get(5)?;
            let titre_specifique: Option<String> = row.get(6)?;
            let service_affiliation: Option<String> = row.get(7)?;
            let email_pro: Option<String> = row.get(8)?;
            let telephone_direct: Option<String> = row.get(9)?;
            let gsm_pro: Option<String> = row.get(10)?;
            let notes_affiliation: Option<String> = row.get(11)?;
            let email_prive: Option<String> = row.get(12)?;
            let telephone_prive: Option<String> = row.get(13)?;
            let adresse_privee: Option<String> = row.get(14)?;
            let cp_prive: Option<String> = row.get(15)?;
            let commune_privee: Option<String> = row.get(16)?;
            let notes_personne: Option<String> = row.get(17)?;
            let adresse_structure: Option<String> = row.get(18)?;
            let cp_structure: Option<String> = row.get(19)?;
            let commune_structure: Option<String> = row.get(20)?;
            let email_general: Option<String> = row.get(21)?;
            let telephone_general: Option<String> = row.get(22)?;
            let service_structure: Option<String> = row.get(23)?;
            let reseau_subvention: Option<String> = row.get(24)?;
            let notes_structure: Option<String> = row.get(25)?;

            Ok((
                category_name,
                ExportRow {
                    institution: clean_text(structure_name.as_deref()),
                    group_label: "Equipe".to_string(),
                    civilite: normalize_civilite(civilite.as_deref()),
                    nom: clean_text(nom.as_deref()),
                    prenom: clean_text(prenom.as_deref()),
                    full_name: join_name_parts(&[prenom.as_deref(), nom.as_deref()]),
                    fonction: first_non_empty(&[
                        titre_specifique.as_deref(),
                        fonction.as_deref(),
                        service_affiliation.as_deref(),
                        service_structure.as_deref(),
                    ]),
                    email: first_non_empty(&[
                        email_pro.as_deref(),
                        email_general.as_deref(),
                        email_prive.as_deref(),
                    ]),
                    telephone: first_non_empty(&[
                        telephone_direct.as_deref(),
                        gsm_pro.as_deref(),
                        telephone_general.as_deref(),
                        telephone_prive.as_deref(),
                    ]),
                    adresse: {
                        let structure_address =
                            join_address(adresse_structure.as_deref(), cp_structure.as_deref());
                        if !structure_address.is_empty() {
                            structure_address
                        } else {
                            join_address(adresse_privee.as_deref(), cp_prive.as_deref())
                        }
                    },
                    commune: first_non_empty(&[
                        commune_structure.as_deref(),
                        commune_privee.as_deref(),
                    ]),
                    autre: merge_unique_lines(&[
                        notes_affiliation.as_deref(),
                        notes_structure.as_deref(),
                        notes_personne.as_deref(),
                        reseau_subvention.as_deref(),
                    ]),
                    is_structure_only: false,
                },
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in rows {
        let (category, export_row) = row.map_err(|e| e.to_string())?;
        if let Some(items) = grouped.get_mut(&category) {
            items.push(export_row);
        }
    }

    let structure_only_rows = load_structure_only_rows(conn)?;
    for (category, export_row) in structure_only_rows {
        if let Some(items) = grouped.get_mut(&category) {
            items.push(export_row);
        }
    }

    for rows in grouped.values_mut() {
        rows.sort_by(|a, b| {
            (
                a.institution.to_lowercase(),
                a.nom.to_lowercase(),
                a.prenom.to_lowercase(),
                a.full_name.to_lowercase(),
                a.fonction.to_lowercase(),
            )
                .cmp(&(
                    b.institution.to_lowercase(),
                    b.nom.to_lowercase(),
                    b.prenom.to_lowercase(),
                    b.full_name.to_lowercase(),
                    b.fonction.to_lowercase(),
                ))
        });
    }

    Ok(grouped)
}

fn load_structure_only_rows(conn: &Connection) -> Result<Vec<(String, ExportRow)>, String> {
    let mut stmt = conn
        .prepare(
            "
            SELECT
                c.Nom_Categorie,
                s.Nom_Structure,
                s.Adresse_Structure,
                s.Code_Postal_Structure,
                s.Commune_Structure,
                s.Email_General,
                s.Telephone_General,
                s.Service_Specifique,
                s.Reseau_Subvention,
                s.Notes_Commentaires
            FROM T_Structures s
            LEFT JOIN T_Categories c ON c.ID_Categorie = s.ID_Categorie
            LEFT JOIN T_Affiliations a ON a.Ref_Structure = s.ID_Structure
            WHERE c.Nom_Categorie IS NOT NULL
              AND a.ID_Affiliation IS NULL
            ORDER BY c.Nom_Categorie ASC, s.Nom_Structure ASC
            ",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let category_name: String = row.get(0)?;
            let structure_name: Option<String> = row.get(1)?;
            let adresse_structure: Option<String> = row.get(2)?;
            let cp_structure: Option<String> = row.get(3)?;
            let commune_structure: Option<String> = row.get(4)?;
            let email_general: Option<String> = row.get(5)?;
            let telephone_general: Option<String> = row.get(6)?;
            let service_structure: Option<String> = row.get(7)?;
            let reseau_subvention: Option<String> = row.get(8)?;
            let notes_structure: Option<String> = row.get(9)?;

            Ok((
                category_name,
                ExportRow {
                    institution: clean_text(structure_name.as_deref()),
                    group_label: "Equipe".to_string(),
                    civilite: String::new(),
                    nom: String::new(),
                    prenom: String::new(),
                    full_name: String::new(),
                    fonction: first_non_empty(&[
                        service_structure.as_deref(),
                        Some("Coordonnées générales du service"),
                    ]),
                    email: clean_text(email_general.as_deref()),
                    telephone: clean_text(telephone_general.as_deref()),
                    adresse: join_address(adresse_structure.as_deref(), cp_structure.as_deref()),
                    commune: clean_text(commune_structure.as_deref()),
                    autre: merge_unique_lines(&[
                        notes_structure.as_deref(),
                        reseau_subvention.as_deref(),
                    ]),
                    is_structure_only: true,
                },
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

fn write_workbook(
    output_path: &Path,
    data: &BTreeMap<String, Vec<ExportRow>>,
) -> Result<ExcelRebuildResult, String> {
    let mut workbook = Workbook::new();

    let title_format = Format::new()
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x1E4E79))
        .set_align(FormatAlign::Center);
    let header_format = Format::new()
        .set_bold()
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::RGB(0xD9EAF7))
        .set_align(FormatAlign::Center);
    let cell_format = Format::new().set_border(FormatBorder::Thin);
    let muted_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_font_color(Color::RGB(0x666666));

    let mut total_rows = 0usize;

    for spec in HISTORICAL_SHEETS {
        let rows = data.get(spec.category_name).cloned().unwrap_or_default();
        total_rows += rows.len();

        let worksheet = workbook.add_worksheet();
        worksheet
            .set_name(spec.sheet_title)
            .map_err(|e| e.to_string())?;

        write_sheet(
            worksheet,
            spec,
            &rows,
            &title_format,
            &header_format,
            &cell_format,
            &muted_format,
        )?;
    }

    let workbook_name = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("export.xlsx")
        .to_string();

    workbook.save(output_path).map_err(|e| e.to_string())?;

    Ok(ExcelRebuildResult {
        path: output_path.to_string_lossy().to_string(),
        workbook_name,
        sheet_count: HISTORICAL_SHEETS.len(),
        row_count: total_rows,
    })
}

fn write_sheet(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    spec: &SheetSpec,
    rows: &[ExportRow],
    title_format: &Format,
    header_format: &Format,
    cell_format: &Format,
    muted_format: &Format,
) -> Result<(), String> {
    let headers = headers_for(spec);
    let start_row = if matches!(spec.layout, SheetLayout::EcrivainsPublics) {
        worksheet
            .merge_range(
                0,
                0,
                0,
                7,
                "RESEAU ECRIVAINS PUBLICS - ARRONDISSEMENT DE VERVIERS",
                title_format,
            )
            .map_err(|e| e.to_string())?;
        1u32
    } else {
        0u32
    };

    for (col, header) in headers.iter().enumerate() {
        worksheet
            .write_with_format(start_row, col as u16, *header, header_format)
            .map_err(|e| e.to_string())?;
    }

    apply_column_widths(worksheet, spec)?;
    worksheet
        .set_freeze_panes(start_row + 1, 0)
        .map_err(|e| e.to_string())?;

    for (index, row) in rows.iter().enumerate() {
        let excel_row = start_row + 1 + index as u32;
        let values = values_for(spec, row);
        for (col, value) in values.iter().enumerate() {
            let format = if row.is_structure_only {
                muted_format
            } else {
                cell_format
            };
            worksheet
                .write_with_format(excel_row, col as u16, value, format)
                .map_err(|e| e.to_string())?;
        }
    }

    let end_row = if rows.is_empty() {
        start_row
    } else {
        start_row + rows.len() as u32
    };
    worksheet
        .autofilter(start_row, 0, end_row, headers.len() as u16 - 1)
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn headers_for(spec: &SheetSpec) -> Vec<&'static str> {
    match spec.layout {
        SheetLayout::Standard => vec![
            spec.first_label,
            "Civilité",
            "Nom-Prénom",
            "Fonction",
            "Adresse de messagerie",
            "Téléphone",
            "Adresse postale",
            "Commune",
            "Autre",
        ],
        SheetLayout::SplitName => vec![
            spec.first_label,
            "Civilité",
            "Nom",
            "Prénom",
            "Adresse de messagerie",
            "Téléphone",
            "Adresse postale",
            "Commune",
            "Autre",
        ],
        SheetLayout::ContactOnly | SheetLayout::EcrivainsPublics => vec![
            "Civilité",
            "Nom-Prénom",
            "Adresse de messagerie",
            "Téléphone",
            "Adresse postale",
            "Commune",
            "Autre",
        ],
        SheetLayout::Crvi => vec![
            "Institution",
            "",
            "Civilité",
            "Nom-Prénom",
            "Département",
            "Email",
        ],
    }
}

fn values_for(spec: &SheetSpec, row: &ExportRow) -> Vec<String> {
    match spec.layout {
        SheetLayout::Standard => vec![
            row.institution.clone(),
            row.civilite.clone(),
            row.full_name.clone(),
            row.fonction.clone(),
            row.email.clone(),
            row.telephone.clone(),
            row.adresse.clone(),
            row.commune.clone(),
            row.autre.clone(),
        ],
        SheetLayout::SplitName => vec![
            row.institution.clone(),
            row.civilite.clone(),
            row.nom.clone(),
            row.prenom.clone(),
            row.email.clone(),
            row.telephone.clone(),
            row.adresse.clone(),
            row.commune.clone(),
            row.autre.clone(),
        ],
        SheetLayout::ContactOnly | SheetLayout::EcrivainsPublics => vec![
            row.civilite.clone(),
            row.full_name.clone(),
            row.email.clone(),
            row.telephone.clone(),
            row.adresse.clone(),
            row.commune.clone(),
            row.autre.clone(),
        ],
        SheetLayout::Crvi => vec![
            row.institution.clone(),
            row.group_label.clone(),
            row.civilite.clone(),
            row.full_name.clone(),
            first_non_empty(&[Some(row.fonction.as_str()), Some(row.autre.as_str())]),
            row.email.clone(),
        ],
    }
}

fn apply_column_widths(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    spec: &SheetSpec,
) -> Result<(), String> {
    let widths: &[f64] = match spec.layout {
        SheetLayout::Standard => &[28.0, 12.0, 24.0, 24.0, 30.0, 18.0, 30.0, 18.0, 32.0],
        SheetLayout::SplitName => &[28.0, 12.0, 18.0, 18.0, 30.0, 18.0, 30.0, 18.0, 32.0],
        SheetLayout::ContactOnly | SheetLayout::EcrivainsPublics => {
            &[12.0, 24.0, 30.0, 18.0, 30.0, 18.0, 32.0]
        }
        SheetLayout::Crvi => &[22.0, 14.0, 12.0, 24.0, 22.0, 30.0],
    };

    for (col, width) in widths.iter().enumerate() {
        worksheet
            .set_column_width(col as u16, *width)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn clean_text(value: Option<&str>) -> String {
    value.map(clean_str).unwrap_or_default()
}

fn clean_str(value: &str) -> String {
    value
        .replace('\u{00a0}', " ")
        .replace('\r', " ")
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .trim_matches('\'')
        .to_string()
}

fn normalize_civilite(value: Option<&str>) -> String {
    match value.map(clean_str).as_deref() {
        Some("M") | Some("Monsieur") => "Monsieur".to_string(),
        Some("Mme") | Some("Madame") => "Madame".to_string(),
        Some("Mlle") => "Mlle".to_string(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

fn join_name_parts(parts: &[Option<&str>]) -> String {
    parts
        .iter()
        .filter_map(|value| value.map(clean_str))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn join_address(address: Option<&str>, postal_code: Option<&str>) -> String {
    let address = address.map(clean_str).unwrap_or_default();
    let postal_code = postal_code.map(clean_str).unwrap_or_default();
    match (address.is_empty(), postal_code.is_empty()) {
        (true, true) => String::new(),
        (false, true) => address,
        (true, false) => postal_code,
        (false, false) => format!("{address}, {postal_code}"),
    }
}

fn first_non_empty(values: &[Option<&str>]) -> String {
    values
        .iter()
        .filter_map(|value| *value)
        .map(clean_str)
        .find(|value| !value.is_empty())
        .unwrap_or_default()
}

fn merge_unique_lines(values: &[Option<&str>]) -> String {
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();

    for value in values.iter().filter_map(|value| *value) {
        let cleaned = clean_str(value);
        if cleaned.is_empty() {
            continue;
        }

        let lowered = cleaned.to_lowercase();
        if seen.insert(lowered) {
            result.push(cleaned);
        }
    }

    result.join(" | ")
}

#[cfg(test)]
mod tests {
    use super::export_reconstructed_excel_to_path;
    use rusqlite::Connection;

    #[test]
    fn writes_a_reconstructed_workbook() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        conn.execute_batch(
            "
            CREATE TABLE T_Categories (ID_Categorie INTEGER PRIMARY KEY, Nom_Categorie TEXT);
            CREATE TABLE T_Structures (
                ID_Structure INTEGER PRIMARY KEY,
                Nom_Structure TEXT,
                Service_Specifique TEXT,
                Reseau_Subvention TEXT,
                Partenaire_Direct INTEGER,
                Adresse_Structure TEXT,
                Code_Postal_Structure TEXT,
                Commune_Structure TEXT,
                Pays TEXT,
                Telephone_General TEXT,
                Email_General TEXT,
                Site_Web TEXT,
                Notes_Commentaires TEXT,
                Date_Creation TEXT,
                ID_Categorie INTEGER
            );
            CREATE TABLE T_Personnes (
                ID_Personne INTEGER PRIMARY KEY,
                Civilite TEXT,
                Nom TEXT,
                Prenom TEXT,
                Email_Prive TEXT,
                Telephone_Prive TEXT,
                Adresse_Privee TEXT,
                Code_Postal_Prive TEXT,
                Commune_Privee TEXT,
                Pays TEXT,
                Consentement_RGPD INTEGER NOT NULL DEFAULT 1,
                Date_Consentement TEXT,
                Statut_Compte TEXT,
                Notes_Commentaires TEXT,
                Date_Creation TEXT
            );
            CREATE TABLE T_Fonctions (ID_Fonction INTEGER PRIMARY KEY, Libelle_Fonction TEXT);
            CREATE TABLE T_Affiliations (
                ID_Affiliation INTEGER PRIMARY KEY,
                Ref_Personne INTEGER,
                Ref_Structure INTEGER,
                Ref_Fonction INTEGER,
                Titre_Specifique TEXT,
                Service_Specifique TEXT,
                Email_Professionnel TEXT,
                Telephone_Direct TEXT,
                Gsm_Professionnel TEXT,
                Date_Debut TEXT,
                Date_Fin TEXT,
                Notes_Commentaires TEXT,
                ID_Categorie INTEGER
            );
            ",
        )
        .expect("schema");

        conn.execute(
            "INSERT INTO T_Categories (ID_Categorie, Nom_Categorie) VALUES (1, 'AG')",
            [],
        )
        .expect("insert category");
        conn.execute(
            "INSERT INTO T_Structures (ID_Structure, Nom_Structure, Adresse_Structure, Code_Postal_Structure, Commune_Structure, Telephone_General, Email_General, ID_Categorie)
             VALUES (1, 'Commune de Test', 'Rue du Test 1', '4800', 'Verviers', '087000000', 'contact@test.be', 1)",
            [],
        )
        .expect("insert structure");
        conn.execute(
            "INSERT INTO T_Personnes (ID_Personne, Civilite, Nom, Prenom, Email_Prive, Consentement_RGPD, Statut_Compte)
             VALUES (1, 'Madame', 'Dupont', 'Jeanne', 'jeanne@test.be', 1, 'Actif')",
            [],
        )
        .expect("insert person");
        conn.execute(
            "INSERT INTO T_Fonctions (ID_Fonction, Libelle_Fonction) VALUES (1, 'Administrateur / Administratrice (CA)')",
            [],
        )
        .expect("insert function");
        conn.execute(
            "INSERT INTO T_Affiliations (ID_Affiliation, Ref_Personne, Ref_Structure, Ref_Fonction, Email_Professionnel, Telephone_Direct, ID_Categorie)
             VALUES (1, 1, 1, 1, 'jeanne.dupont@test.be', '0499000000', 1)",
            [],
        )
        .expect("insert affiliation");

        let output_path =
            std::env::temp_dir().join(format!("crvi_export_test_{}.xlsx", std::process::id()));
        if output_path.exists() {
            let _ = std::fs::remove_file(&output_path);
        }

        let result = export_reconstructed_excel_to_path(&conn, &output_path).expect("export");

        assert!(output_path.exists(), "xlsx file should exist");
        assert_eq!(result.sheet_count, super::HISTORICAL_SHEETS.len());
        assert!(result.row_count >= 1);

        let _ = std::fs::remove_file(output_path);
    }
}
