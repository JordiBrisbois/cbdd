use super::*;
use crate::auth;
use crate::db;
use tauri::AppHandle;

#[derive(Debug, Clone)]
struct TableMeta {
    name: &'static str,
    alias: &'static str,
    pk: &'static str,
}

const TABLES: &[TableMeta] = &[
    TableMeta {
        name: "T_Personnes",
        alias: "p",
        pk: "ID_Personne",
    },
    TableMeta {
        name: "T_Structures",
        alias: "s",
        pk: "ID_Structure",
    },
    TableMeta {
        name: "T_Affiliations",
        alias: "a",
        pk: "ID_Affiliation",
    },
    TableMeta {
        name: "T_Categories",
        alias: "c",
        pk: "ID_Categorie",
    },
    TableMeta {
        name: "T_Fonctions",
        alias: "f",
        pk: "ID_Fonction",
    },
    TableMeta {
        name: "T_Reunions",
        alias: "r",
        pk: "ID_Reunion",
    },
    TableMeta {
        name: "T_Presences",
        alias: "pr",
        pk: "ID_Presence",
    },
];

fn alias_to_table(alias: &str) -> Option<&'static TableMeta> {
    TABLES.iter().find(|t| t.alias == alias)
}

fn get_col_sql(champ: &str) -> Result<String, String> {
    let champ_lc = champ.to_lowercase();
    let col = match champ_lc.as_str() {
        "p.id_personne" | "p.id" => "p.ID_Personne",
        "p.civilite" => "p.Civilite",
        "p.nom" => "p.Nom",
        "p.prenom" => "p.Prenom",
        "p.email_prive" | "p.email" => "p.Email_Prive",
        "p.telephone_prive" | "p.tel" => "p.Telephone_Prive",
        "p.adresse_privee" | "p.adresse" => "p.Adresse_Privee",
        "p.code_postal_prive" | "p.cp" => "p.Code_Postal_Prive",
        "p.commune_privee" | "p.commune" => "p.Commune_Privee",
        "p.pays" => "p.Pays",
        "p.consentement_rgpd" | "p.rgpd" => "p.Consentement_RGPD",
        "p.date_consentement" => "p.Date_Consentement",
        "p.statut_compte" | "p.statut" => "p.Statut_Compte",
        "p.notes_commentaires" | "p.notes" => "p.Notes_Commentaires",
        "p.date_creation" => "p.Date_Creation",
        "s.id_structure" | "s.id" => "s.ID_Structure",
        "s.nom_structure" | "s.nom" => "s.Nom_Structure",
        "s.type_structure" | "s.type" => "c.Nom_Categorie",
        "s.service_specifique" | "s.service" => "s.Service_Specifique",
        "s.reseau_subvention" | "s.reseau" => "s.Reseau_Subvention",
        "s.partenaire_direct" | "s.partenaire" => "s.Partenaire_Direct",
        "s.adresse_structure" | "s.adresse" => "s.Adresse_Structure",
        "s.code_postal_structure" | "s.cp" => "s.Code_Postal_Structure",
        "s.commune_structure" | "s.commune" => "s.Commune_Structure",
        "s.pays" => "s.Pays",
        "s.telephone_general" | "s.tel" => "s.Telephone_General",
        "s.email_general" | "s.email" => "s.Email_General",
        "s.site_web" | "s.site" => "s.Site_Web",
        "s.notes_commentaires" | "s.notes" => "s.Notes_Commentaires",
        "s.date_creation" => "s.Date_Creation",
        "a.id_affiliation" | "a.id" => "a.ID_Affiliation",
        "a.ref_personne" => "a.Ref_Personne",
        "a.ref_structure" => "a.Ref_Structure",
        "a.ref_fonction" => "a.Ref_Fonction",
        "a.titre_specifique" | "a.titre" => "a.Titre_Specifique",
        "a.service_specifique" | "a.service" => "a.Service_Specifique",
        "a.email_professionnel" | "a.email_pro" => "a.Email_Professionnel",
        "a.telephone_direct" | "a.tel_direct" => "a.Telephone_Direct",
        "a.gsm_professionnel" | "a.gsm" => "a.Gsm_Professionnel",
        "a.date_debut" => "a.Date_Debut",
        "a.date_fin" => "a.Date_Fin",
        "a.notes_commentaires" | "a.notes" => "a.Notes_Commentaires",
        "a.id_categorie" | "a.categorie" => "a.ID_Categorie",
        "c.id_categorie" | "c.id" => "c.ID_Categorie",
        "c.nom_categorie" | "c.nom" => "c.Nom_Categorie",
        "f.id_fonction" | "f.id" => "f.ID_Fonction",
        "f.libelle_fonction" | "f.libelle" => "f.Libelle_Fonction",
        "r.id_reunion" | "r.id" => "r.ID_Reunion",
        "r.titre_reunion" | "r.titre" => "r.Titre_Reunion",
        "r.date_reunion" | "r.date" => "r.Date_Reunion",
        "r.heure_reunion" | "r.heure" => "r.Heure_Reunion",
        "r.lieu_reunion" | "r.lieu" => "r.Lieu_Reunion",
        "r.ref_structure" => "r.Ref_Structure",
        "r.notes_commentaires" | "r.notes" => "r.Notes_Commentaires",
        "pr.id_presence" | "pr.id" => "pr.ID_Presence",
        "pr.ref_reunion" => "pr.Ref_Reunion",
        "pr.ref_personne" => "pr.Ref_Personne",
        "pr.statut_presence" | "pr.statut" => "pr.Statut_Presence",
        "pr.souhaite_rester_en_bdd" | "pr.reste_bdd" => "pr.Souhaite_Rester_En_BDD",
        "pr.notes_commentaires" | "pr.notes" => "pr.Notes_Commentaires",
        _ => return Err(format!("Champ inconnu: {}", champ)),
    };
    Ok(col.to_string())
}

fn extract_alias(champ: &str) -> &str {
    champ.split('.').next().unwrap_or("")
}

fn normalize_query_operator(operator: &str) -> String {
    operator.trim().to_lowercase().replace([' ', '-'], "_")
}

fn build_joins(main_alias: &str, needed: &std::collections::HashSet<&str>) -> Vec<String> {
    let mut joins = Vec::new();
    let needed_vec: Vec<&str> = needed
        .iter()
        .copied()
        .filter(|a| *a != main_alias)
        .collect();

    let needs_affiliation = needed_vec.contains(&"a");
    let needs_personne = needed_vec.contains(&"p") || main_alias == "p";
    let needs_structure = needed_vec.contains(&"s") || main_alias == "s";
    let needs_categorie = needed_vec.contains(&"c");
    let needs_fonction = needed_vec.contains(&"f");
    let needs_reunion = needed_vec.contains(&"r") || main_alias == "r";
    let needs_presence = needed_vec.contains(&"pr");

    match main_alias {
        "p" => {
            if needs_affiliation || needs_structure || needs_categorie || needs_fonction {
                joins.push(
                    "LEFT JOIN T_Affiliations a ON a.Ref_Personne = p.ID_Personne".to_string(),
                );
            }
            if needs_structure {
                joins.push(
                    "LEFT JOIN T_Structures s ON s.ID_Structure = a.Ref_Structure".to_string(),
                );
            }
            if needs_categorie {
                if !needs_affiliation {
                    joins.push(
                        "LEFT JOIN T_Affiliations a ON a.Ref_Personne = p.ID_Personne".to_string(),
                    );
                }
                joins.push(
                    "LEFT JOIN T_Categories c ON c.ID_Categorie = a.ID_Categorie".to_string(),
                );
            }
            if needs_fonction {
                if !needs_affiliation {
                    joins.push(
                        "LEFT JOIN T_Affiliations a ON a.Ref_Personne = p.ID_Personne".to_string(),
                    );
                }
                joins.push("LEFT JOIN T_Fonctions f ON f.ID_Fonction = a.Ref_Fonction".to_string());
            }
            if needs_reunion {
                joins.push(
                    "LEFT JOIN T_Presences pr ON pr.Ref_Personne = p.ID_Personne".to_string(),
                );
                joins.push("LEFT JOIN T_Reunions r ON r.ID_Reunion = pr.Ref_Reunion".to_string());
            }
            if needs_presence && !needs_reunion {
                joins.push(
                    "LEFT JOIN T_Presences pr ON pr.Ref_Personne = p.ID_Personne".to_string(),
                );
            }
        }
        "s" => {
            if needs_affiliation || needs_personne || needs_fonction {
                joins.push(
                    "LEFT JOIN T_Affiliations a ON a.Ref_Structure = s.ID_Structure".to_string(),
                );
            }
            if needs_personne {
                joins.push("LEFT JOIN T_Personnes p ON p.ID_Personne = a.Ref_Personne".to_string());
            }
            if needs_categorie {
                joins.push(
                    "LEFT JOIN T_Categories c ON c.ID_Categorie = s.ID_Categorie".to_string(),
                );
            }
            if needs_fonction {
                if !needs_affiliation {
                    joins.push(
                        "LEFT JOIN T_Affiliations a ON a.Ref_Structure = s.ID_Structure"
                            .to_string(),
                    );
                }
                joins.push("LEFT JOIN T_Fonctions f ON f.ID_Fonction = a.Ref_Fonction".to_string());
            }
            if needs_reunion {
                joins
                    .push("LEFT JOIN T_Reunions r ON r.Ref_Structure = s.ID_Structure".to_string());
            }
        }
        "a" => {
            joins.push("LEFT JOIN T_Personnes p ON p.ID_Personne = a.Ref_Personne".to_string());
            joins.push("LEFT JOIN T_Structures s ON s.ID_Structure = a.Ref_Structure".to_string());
            if needs_categorie {
                joins.push(
                    "LEFT JOIN T_Categories c ON c.ID_Categorie = a.ID_Categorie".to_string(),
                );
            }
            if needs_fonction {
                joins.push("LEFT JOIN T_Fonctions f ON f.ID_Fonction = a.Ref_Fonction".to_string());
            }
        }
        "r" => {
            if needs_structure {
                joins.push(
                    "LEFT JOIN T_Structures s ON s.ID_Structure = r.Ref_Structure".to_string(),
                );
            }
            if needs_personne || needs_presence {
                joins.push("LEFT JOIN T_Presences pr ON pr.Ref_Reunion = r.ID_Reunion".to_string());
            }
            if needs_personne {
                joins
                    .push("LEFT JOIN T_Personnes p ON p.ID_Personne = pr.Ref_Personne".to_string());
            }
        }
        "pr" => {
            joins.push("LEFT JOIN T_Personnes p ON p.ID_Personne = pr.Ref_Personne".to_string());
            joins.push("LEFT JOIN T_Reunions r ON r.ID_Reunion = pr.Ref_Reunion".to_string());
            if needs_structure {
                joins.push(
                    "LEFT JOIN T_Structures s ON s.ID_Structure = r.Ref_Structure".to_string(),
                );
            }
        }
        _ => {}
    }

    joins
}

fn insert_json_string(
    map: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value {
        map.insert(key.to_string(), serde_json::Value::String(value));
    }
}

fn insert_json_i64(
    map: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<i64>,
) {
    if let Some(value) = value {
        map.insert(
            key.to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }
}

pub fn executer_requete_impl(
    app: AppHandle,
    table_principale: String,
    colonnes: Vec<String>,
    conditions: Vec<Condition>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "search.read")?;

    let table = table_principale.to_lowercase();
    let main_meta = match table.as_str() {
        "personnes" => alias_to_table("p").unwrap(),
        "structures" => alias_to_table("s").unwrap(),
        "affiliations" => alias_to_table("a").unwrap(),
        "reunions" => alias_to_table("r").unwrap(),
        "presences" => alias_to_table("pr").unwrap(),
        _ => return Err(format!("Table inconnue: {}", table_principale)),
    };

    let mut needed_aliases = std::collections::HashSet::new();
    needed_aliases.insert(main_meta.alias);

    for col in &colonnes {
        needed_aliases.insert(extract_alias(col));
    }
    for cond in &conditions {
        needed_aliases.insert(extract_alias(&cond.champ));
    }

    let mut select_parts = Vec::new();
    for col in &colonnes {
        select_parts.push(get_col_sql(col)?);
    }
    if select_parts.is_empty() {
        select_parts.push(format!("{}.{}", main_meta.alias, main_meta.pk));
    }
    let meta_select_parts = super::get_query_meta_select_parts(main_meta.alias);

    let joins = build_joins(main_meta.alias, &needed_aliases);

    let mut sql_conditions: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if main_meta.alias == "p" {
        sql_conditions.push("COALESCE(p.Statut_Compte, '') != 'Anonymisé'".to_string());
    }

    for cond in &conditions {
        let col_sql = get_col_sql(&cond.champ)?;
        let op = normalize_query_operator(&cond.operateur);
        let valeur = cond.valeur.as_deref().unwrap_or("");

        let condition_str = match op.as_str() {
            "=" | "egal" | "eq" => {
                params.push(Box::new(valeur.to_string()));
                format!("{} = ?", col_sql)
            }
            "!=" | "different" | "neq" => {
                params.push(Box::new(valeur.to_string()));
                format!("{} != ?", col_sql)
            }
            ">" | "superieur" | "gt" => {
                params.push(Box::new(valeur.to_string()));
                format!("{} > ?", col_sql)
            }
            "<" | "inferieur" | "lt" => {
                params.push(Box::new(valeur.to_string()));
                format!("{} < ?", col_sql)
            }
            ">=" | "gte" => {
                params.push(Box::new(valeur.to_string()));
                format!("{} >= ?", col_sql)
            }
            "<=" | "lte" => {
                params.push(Box::new(valeur.to_string()));
                format!("{} <= ?", col_sql)
            }
            "like" | "contient" | "contains" => {
                params.push(Box::new(format!("%{}%", valeur)));
                format!("{} LIKE ?", col_sql)
            }
            "commence_par" | "starts_with" | "startswith" => {
                params.push(Box::new(format!("{}%", valeur)));
                format!("{} LIKE ?", col_sql)
            }
            "finit_par" | "ends_with" | "endswith" => {
                params.push(Box::new(format!("%{}", valeur)));
                format!("{} LIKE ?", col_sql)
            }
            "is_null" | "est_vide" | "null" => format!("{} IS NULL", col_sql),
            "is_not_null" | "pas_vide" | "not_null" => format!("{} IS NOT NULL", col_sql),
            "is_duplicate" | "doublon" | "est_un_doublon" => {
                let alias = extract_alias(&cond.champ);
                match alias_to_table(alias) {
                    Some(meta) => {
                        let raw_col = col_sql.split('.').next_back().unwrap_or(&col_sql);
                        format!(
                            "{} IN (SELECT {} FROM {} WHERE {} IS NOT NULL GROUP BY {} HAVING COUNT(*) > 1)",
                            col_sql, raw_col, meta.name, raw_col, raw_col
                        )
                    }
                    None => {
                        return Err(
                            "Impossible de déterminer la table pour la recherche de doublons"
                                .into(),
                        );
                    }
                }
            }
            "is_not_duplicate" | "pas_doublon" | "unique" => {
                let alias = extract_alias(&cond.champ);
                match alias_to_table(alias) {
                    Some(meta) => {
                        let raw_col = col_sql.split('.').next_back().unwrap_or(&col_sql);
                        format!(
                            "{} IN (SELECT {} FROM {} WHERE {} IS NOT NULL GROUP BY {} HAVING COUNT(*) = 1)",
                            col_sql, raw_col, meta.name, raw_col, raw_col
                        )
                    }
                    None => {
                        return Err(
                            "Impossible de déterminer la table pour la recherche de doublons"
                                .into(),
                        );
                    }
                }
            }
            "in_list" | "in" | "dans_liste" | "dans" => {
                let values: Vec<&str> = valeur
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                if values.is_empty() {
                    return Err("La liste est vide. Séparez les valeurs par des virgules.".into());
                }
                let placeholders: Vec<String> = values.iter().map(|_| "?".to_string()).collect();
                for v in &values {
                    params.push(Box::new(v.to_string()));
                }
                format!("{} IN ({})", col_sql, placeholders.join(", "))
            }
            "not_in_list" | "not_in" | "pas_dans" => {
                let values: Vec<&str> = valeur
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                if values.is_empty() {
                    return Err("La liste est vide. Séparez les valeurs par des virgules.".into());
                }
                let placeholders: Vec<String> = values.iter().map(|_| "?".to_string()).collect();
                for v in &values {
                    params.push(Box::new(v.to_string()));
                }
                format!("{} NOT IN ({})", col_sql, placeholders.join(", "))
            }
            "between" | "entre" => {
                let parts: Vec<&str> = valeur.splitn(2, ',').collect();
                if parts.len() != 2 {
                    return Err("Le format attendu est 'valeur1, valeur2'".into());
                }
                params.push(Box::new(parts[0].trim().to_string()));
                params.push(Box::new(parts[1].trim().to_string()));
                format!("{} BETWEEN ? AND ?", col_sql)
            }
            _ => return Err(format!("Opérateur inconnu: {}", op)),
        };
        sql_conditions.push(condition_str);
    }

    let mut sql = format!(
        "SELECT DISTINCT {}{} FROM {} {}",
        select_parts.join(", "),
        if meta_select_parts.is_empty() {
            String::new()
        } else {
            format!(", {}", meta_select_parts.join(", "))
        },
        main_meta.name,
        main_meta.alias
    );
    for join in &joins {
        sql.push(' ');
        sql.push_str(join);
    }
    if !sql_conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&sql_conditions.join(" AND "));
    }

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            let mut map = serde_json::Map::new();
            for (i, col_name) in colonnes.iter().enumerate() {
                let key = col_name
                    .split('.')
                    .next_back()
                    .unwrap_or(col_name.as_str())
                    .to_string();
                let val: String = row.get(i).unwrap_or_default();
                map.insert(key, serde_json::Value::String(val));
            }
            let base_index = colonnes.len();
            match main_meta.alias {
                "p" | "s" | "r" => {
                    let id: Option<i64> = row.get(base_index).ok();
                    insert_json_string(&mut map, "_entity", Some(main_meta.alias.to_string()));
                    insert_json_i64(&mut map, "_id", id);
                }
                "a" => {
                    let id: Option<i64> = row.get(base_index).ok();
                    let personne_id: Option<i64> = row.get(base_index + 1).ok();
                    let structure_id: Option<i64> = row.get(base_index + 2).ok();
                    insert_json_string(&mut map, "_entity", Some(main_meta.alias.to_string()));
                    insert_json_i64(&mut map, "_id", id);
                    insert_json_i64(&mut map, "_personne_id", personne_id);
                    insert_json_i64(&mut map, "_structure_id", structure_id);
                }
                "pr" => {
                    let id: Option<i64> = row.get(base_index).ok();
                    let reunion_id: Option<i64> = row.get(base_index + 1).ok();
                    insert_json_string(&mut map, "_entity", Some(main_meta.alias.to_string()));
                    insert_json_i64(&mut map, "_id", id);
                    insert_json_i64(&mut map, "_reunion_id", reunion_id);
                }
                _ => {}
            }
            Ok(serde_json::Value::Object(map))
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::normalize_query_operator;

    #[test]
    fn normalizes_query_operators_from_search_page() {
        let cases = [
            ("=", "="),
            ("!=", "!="),
            (">", ">"),
            ("<", "<"),
            (">=", ">="),
            ("<=", "<="),
            ("LIKE", "like"),
            ("commence_par", "commence_par"),
            ("finit_par", "finit_par"),
            ("IS NULL", "is_null"),
            ("IS NOT NULL", "is_not_null"),
            ("is_duplicate", "is_duplicate"),
            ("is_not_duplicate", "is_not_duplicate"),
            ("in_list", "in_list"),
            ("not_in_list", "not_in_list"),
            ("between", "between"),
        ];

        for (input, expected) in cases {
            assert_eq!(normalize_query_operator(input), expected);
        }
    }
}
