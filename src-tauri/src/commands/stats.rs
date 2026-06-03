use super::*;

#[tauri::command]
pub fn get_dashboard_stats(
    app: AppHandle,
    filters: Option<StatsFilters>,
) -> Result<DashboardStats, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "stats.read")?;
    let filters = filters.unwrap_or(StatsFilters {
        start_date: None,
        end_date: None,
    });

    let (available_start_date, available_end_date): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT MIN(Date_Reunion), MAX(Date_Reunion) FROM T_Reunions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    let total_contacts = query_i64(&conn, "SELECT COUNT(*) FROM T_Personnes", Vec::new())?;
    let active_contacts = query_i64(
        &conn,
        "SELECT COUNT(*) FROM T_Personnes WHERE COALESCE(Statut_Compte, '') != ?",
        vec![ANONYMIZED_STATUS.to_string()],
    )?;
    let anonymized_contacts = query_i64(
        &conn,
        "SELECT COUNT(*) FROM T_Personnes WHERE Statut_Compte = ?",
        vec![ANONYMIZED_STATUS.to_string()],
    )?;
    let contacts_to_delete = query_i64(
        &conn,
        &format!(
            "SELECT COUNT(*) FROM T_Personnes p WHERE {}",
            rgpd_attention_predicate("p")
        ),
        Vec::new(),
    )?;
    let total_structures = query_i64(&conn, "SELECT COUNT(*) FROM T_Structures", Vec::new())?;
    let total_categories = query_i64(&conn, "SELECT COUNT(*) FROM T_Categories", Vec::new())?;
    let total_affiliations =
        query_i64(&conn, "SELECT COUNT(*) FROM T_Affiliations", Vec::new())?;
    let total_reunions = query_i64(&conn, "SELECT COUNT(*) FROM T_Reunions", Vec::new())?;
    let partner_direct_structures = query_i64(
        &conn,
        "SELECT COUNT(*) FROM T_Structures WHERE Partenaire_Direct = 1",
        Vec::new(),
    )?;
    let rgpd_consent_rate = query_f64(
        &conn,
        "SELECT COALESCE(
            AVG(
                CASE
                    WHEN p.Consentement_RGPD = 1
                      AND COALESCE(p.Statut_Compte, '') != 'A supprimer'
                      AND NOT EXISTS (
                          SELECT 1
                          FROM T_Presences pr
                          WHERE pr.Ref_Personne = p.ID_Personne
                            AND pr.Souhaite_Rester_En_BDD = 0
                      )
                    THEN 100.0
                    ELSE 0.0
                END
            ),
            0
         )
         FROM T_Personnes p
         WHERE COALESCE(p.Statut_Compte, '') != ?",
        vec![ANONYMIZED_STATUS.to_string()],
    )?;

    let mut period_clauses = Vec::new();
    let mut period_params = Vec::new();
    append_date_filters(&mut period_clauses, &mut period_params, "r.Date_Reunion", &filters);
    let period_where = build_where_sql(&period_clauses);

    let period_reunions = query_i64(
        &conn,
        &format!("SELECT COUNT(*) FROM T_Reunions r{}", period_where),
        period_params.clone(),
    )?;
    let period_presences = query_i64(
        &conn,
        &format!(
            "SELECT COUNT(*)
             FROM T_Presences p
             INNER JOIN T_Reunions r ON r.ID_Reunion = p.Ref_Reunion{}",
            period_where
        ),
        period_params.clone(),
    )?;
    let average_presences_per_reunion = query_f64(
        &conn,
        &format!(
            "SELECT COALESCE(AVG(attendee_count), 0)
             FROM (
               SELECT COUNT(p.ID_Presence) AS attendee_count
               FROM T_Reunions r
               LEFT JOIN T_Presences p ON p.Ref_Reunion = r.ID_Reunion{}
               GROUP BY r.ID_Reunion
             )",
            period_where
        ),
        period_params.clone(),
    )?;

    let meetings_by_month = {
        let mut clauses = vec!["r.Date_Reunion IS NOT NULL".to_string()];
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, "r.Date_Reunion", &filters);
        load_stats_buckets(
            &conn,
            &format!(
                "SELECT substr(r.Date_Reunion, 1, 7), substr(r.Date_Reunion, 1, 7), COUNT(*)
                 FROM T_Reunions r{}
                 GROUP BY substr(r.Date_Reunion, 1, 7)
                 ORDER BY substr(r.Date_Reunion, 1, 7) ASC",
                build_where_sql(&clauses)
            ),
            params,
        )?
    };

    let contacts_created_by_month = {
        let creation_date_expr = sqlite_date_expr("p.Date_Creation");
        let creation_month_expr = sqlite_month_expr("p.Date_Creation");
        let mut clauses = vec![format!("{} IS NOT NULL", creation_date_expr)];
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, &creation_date_expr, &filters);
        load_stats_buckets(
            &conn,
            &format!(
                "SELECT {month_expr}, {month_expr}, COUNT(*)
                 FROM T_Personnes p{}
                 GROUP BY {month_expr}
                 ORDER BY {month_expr} ASC",
                build_where_sql(&clauses),
                month_expr = creation_month_expr,
            ),
            params,
        )?
    };

    let attendance_by_status = {
        let mut clauses = vec!["COALESCE(p.Statut_Presence, '') != ''".to_string()];
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, "r.Date_Reunion", &filters);
        load_stats_buckets(
            &conn,
            &format!(
                "SELECT p.Statut_Presence, p.Statut_Presence, COUNT(*)
                 FROM T_Presences p
                 INNER JOIN T_Reunions r ON r.ID_Reunion = p.Ref_Reunion{}
                 GROUP BY p.Statut_Presence
                 ORDER BY COUNT(*) DESC, p.Statut_Presence ASC",
                build_where_sql(&clauses)
            ),
            params,
        )?
    };

    let structures_by_category = load_stats_buckets(
        &conn,
        "SELECT
            COALESCE(CAST(s.ID_Categorie AS TEXT), 'none'),
            COALESCE(c.Nom_Categorie, 'Sans catégorie'),
            COUNT(*)
         FROM T_Structures s
         LEFT JOIN T_Categories c ON c.ID_Categorie = s.ID_Categorie
         GROUP BY COALESCE(c.Nom_Categorie, 'Sans catégorie'), COALESCE(CAST(s.ID_Categorie AS TEXT), 'none')
         ORDER BY COUNT(*) DESC, COALESCE(c.Nom_Categorie, 'Sans catégorie') ASC",
        Vec::new(),
    )?;

    let contacts_by_commune = merge_casefolded_buckets(load_stats_buckets(
        &conn,
        "SELECT
            COALESCE(NULLIF(TRIM(Commune_Privee), ''), 'Non renseignée'),
            COALESCE(NULLIF(TRIM(Commune_Privee), ''), 'Non renseignée'),
            COUNT(*)
         FROM T_Personnes
         WHERE COALESCE(Statut_Compte, '') != 'Anonymisé'
         GROUP BY COALESCE(NULLIF(TRIM(Commune_Privee), ''), 'Non renseignée')
         ORDER BY COUNT(*) DESC, COALESCE(NULLIF(TRIM(Commune_Privee), ''), 'Non renseignée') ASC
         LIMIT 8",
        Vec::new(),
    )?);

    let structures_by_commune = merge_casefolded_buckets(load_stats_buckets(
        &conn,
        "SELECT
            COALESCE(NULLIF(TRIM(Commune_Structure), ''), 'Non renseignée'),
            COALESCE(NULLIF(TRIM(Commune_Structure), ''), 'Non renseignée'),
            COUNT(*)
         FROM T_Structures
         GROUP BY COALESCE(NULLIF(TRIM(Commune_Structure), ''), 'Non renseignée')
         ORDER BY COUNT(*) DESC, COALESCE(NULLIF(TRIM(Commune_Structure), ''), 'Non renseignée') ASC
         LIMIT 8",
        Vec::new(),
    )?);

    let meetings_by_organisme = {
        let mut clauses = Vec::new();
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, "r.Date_Reunion", &filters);
        load_stats_buckets(
            &conn,
            &format!(
                "SELECT
                    COALESCE(CAST(r.Ref_Structure AS TEXT), 'none'),
                    COALESCE(s.Nom_Structure, 'Sans organisme'),
                    COUNT(*)
                 FROM T_Reunions r
                 LEFT JOIN T_Structures s ON s.ID_Structure = r.Ref_Structure{}
                 GROUP BY COALESCE(s.Nom_Structure, 'Sans organisme'), COALESCE(CAST(r.Ref_Structure AS TEXT), 'none')
                 ORDER BY COUNT(*) DESC, COALESCE(s.Nom_Structure, 'Sans organisme') ASC
                 LIMIT 8",
                build_where_sql(&clauses)
            ),
            params,
        )?
    };

    let account_statuses = load_stats_buckets(
        &conn,
        "SELECT
            COALESCE(NULLIF(TRIM(Statut_Compte), ''), 'Inconnu'),
            COALESCE(NULLIF(TRIM(Statut_Compte), ''), 'Inconnu'),
            COUNT(*)
         FROM T_Personnes
         GROUP BY COALESCE(NULLIF(TRIM(Statut_Compte), ''), 'Inconnu')
         ORDER BY COUNT(*) DESC, COALESCE(NULLIF(TRIM(Statut_Compte), ''), 'Inconnu') ASC",
        Vec::new(),
    )?;

    let quality_checks = vec![
        StatsBucket {
            key: "contacts_without_email".to_string(),
            label: "Contacts sans email".to_string(),
            value: query_i64(
                &conn,
                "SELECT COUNT(*) FROM T_Personnes
                 WHERE COALESCE(Statut_Compte, '') != 'Anonymisé'
                   AND COALESCE(NULLIF(TRIM(Email_Prive), ''), '') = ''",
                Vec::new(),
            )?,
        },
        StatsBucket {
            key: "contacts_without_phone".to_string(),
            label: "Contacts sans téléphone".to_string(),
            value: query_i64(
                &conn,
                "SELECT COUNT(*) FROM T_Personnes
                 WHERE COALESCE(Statut_Compte, '') != 'Anonymisé'
                   AND COALESCE(NULLIF(TRIM(Telephone_Prive), ''), '') = ''",
                Vec::new(),
            )?,
        },
        StatsBucket {
            key: "structures_without_category".to_string(),
            label: "Structures sans catégorie".to_string(),
            value: query_i64(
                &conn,
                "SELECT COUNT(*) FROM T_Structures WHERE ID_Categorie IS NULL",
                Vec::new(),
            )?,
        },
        StatsBucket {
            key: "structures_without_email".to_string(),
            label: "Structures sans email".to_string(),
            value: query_i64(
                &conn,
                "SELECT COUNT(*) FROM T_Structures
                 WHERE COALESCE(NULLIF(TRIM(Email_General), ''), '') = ''",
                Vec::new(),
            )?,
        },
        StatsBucket {
            key: "meetings_without_structure".to_string(),
            label: "Réunions sans organisme".to_string(),
            value: query_i64(
                &conn,
                "SELECT COUNT(*) FROM T_Reunions WHERE Ref_Structure IS NULL",
                Vec::new(),
            )?,
        },
    ];

    let top_meetings = {
        let mut clauses = Vec::new();
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, "r.Date_Reunion", &filters);
        load_top_meetings(
            &conn,
            &format!(
                "SELECT
                    COALESCE(r.Titre_Reunion, 'Réunion sans titre'),
                    COUNT(p.ID_Presence),
                    r.Date_Reunion,
                    COALESCE(s.Nom_Structure, 'Sans organisme')
                 FROM T_Reunions r
                 LEFT JOIN T_Presences p ON p.Ref_Reunion = r.ID_Reunion
                 LEFT JOIN T_Structures s ON s.ID_Structure = r.Ref_Structure{}
                 GROUP BY r.ID_Reunion, r.Titre_Reunion, r.Date_Reunion, s.Nom_Structure
                 ORDER BY COUNT(p.ID_Presence) DESC, r.Date_Reunion DESC
                 LIMIT 8",
                build_where_sql(&clauses)
            ),
            params,
        )?
    };

    let top_structures_presence_rate = {
        let mut clauses = Vec::new();
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, "r.Date_Reunion", &filters);
        let where_sql = build_where_sql(&clauses);
        load_stats_participation(
            &conn,
            &format!(
                "WITH presence_structure AS (
                    SELECT DISTINCT
                        p.ID_Presence AS presence_id,
                        s.ID_Structure AS structure_id,
                        COALESCE(s.Nom_Structure, 'Structure sans nom') AS structure_name,
                        COALESCE(LOWER(TRIM(p.Statut_Presence)), '') AS status_key
                    FROM T_Presences p
                    INNER JOIN T_Reunions r ON r.ID_Reunion = p.Ref_Reunion
                    INNER JOIN T_Affiliations a ON a.Ref_Personne = p.Ref_Personne
                    INNER JOIN T_Structures s ON s.ID_Structure = a.Ref_Structure
                    {where_sql}
                )
                SELECT
                    CAST(structure_id AS TEXT),
                    structure_name,
                    COUNT(*) AS invitations,
                    SUM(CASE WHEN status_key IN ('present', 'présent') THEN 1 ELSE 0 END) AS presents,
                    COALESCE(ROUND(
                        SUM(CASE WHEN status_key IN ('present', 'présent') THEN 100.0 ELSE 0.0 END) / NULLIF(COUNT(*), 0),
                        1
                    ), 0) AS presence_rate
                FROM presence_structure
                GROUP BY structure_id, structure_name
                HAVING COUNT(*) >= 3
                ORDER BY presence_rate DESC, presents DESC, structure_name ASC
                LIMIT 8"
            ),
            params,
        )?
    };

    let top_structures_presence_volume = {
        let mut clauses = Vec::new();
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, "r.Date_Reunion", &filters);
        let where_sql = build_where_sql(&clauses);
        load_stats_participation(
            &conn,
            &format!(
                "WITH presence_structure AS (
                    SELECT DISTINCT
                        p.ID_Presence AS presence_id,
                        s.ID_Structure AS structure_id,
                        COALESCE(s.Nom_Structure, 'Structure sans nom') AS structure_name,
                        COALESCE(LOWER(TRIM(p.Statut_Presence)), '') AS status_key
                    FROM T_Presences p
                    INNER JOIN T_Reunions r ON r.ID_Reunion = p.Ref_Reunion
                    INNER JOIN T_Affiliations a ON a.Ref_Personne = p.Ref_Personne
                    INNER JOIN T_Structures s ON s.ID_Structure = a.Ref_Structure
                    {where_sql}
                )
                SELECT
                    CAST(structure_id AS TEXT),
                    structure_name,
                    COUNT(*) AS invitations,
                    SUM(CASE WHEN status_key IN ('present', 'présent') THEN 1 ELSE 0 END) AS presents,
                    COALESCE(ROUND(
                        SUM(CASE WHEN status_key IN ('present', 'présent') THEN 100.0 ELSE 0.0 END) / NULLIF(COUNT(*), 0),
                        1
                    ), 0) AS presence_rate
                FROM presence_structure
                GROUP BY structure_id, structure_name
                ORDER BY presents DESC, presence_rate DESC, structure_name ASC
                LIMIT 8"
            ),
            params,
        )?
    };

    let top_people_presence = {
        let mut clauses = vec!["COALESCE(pe.Statut_Compte, '') != 'Anonymisé'".to_string()];
        let mut params = Vec::new();
        append_date_filters(&mut clauses, &mut params, "r.Date_Reunion", &filters);
        load_stats_participation(
            &conn,
            &format!(
                "SELECT
                    CAST(pe.ID_Personne AS TEXT),
                    TRIM(COALESCE(pe.Nom, '') || ' ' || COALESCE(pe.Prenom, '')),
                    COUNT(*) AS invitations,
                    SUM(CASE WHEN COALESCE(LOWER(TRIM(p.Statut_Presence)), '') IN ('present', 'présent') THEN 1 ELSE 0 END) AS presents,
                    COALESCE(ROUND(
                        SUM(CASE WHEN COALESCE(LOWER(TRIM(p.Statut_Presence)), '') IN ('present', 'présent') THEN 100.0 ELSE 0.0 END) / NULLIF(COUNT(*), 0),
                        1
                    ), 0) AS presence_rate
                 FROM T_Presences p
                 INNER JOIN T_Reunions r ON r.ID_Reunion = p.Ref_Reunion
                 INNER JOIN T_Personnes pe ON pe.ID_Personne = p.Ref_Personne
                 {}
                 GROUP BY pe.ID_Personne, pe.Nom, pe.Prenom
                 HAVING COUNT(*) >= 2
                 ORDER BY presents DESC, presence_rate DESC, pe.Nom ASC, pe.Prenom ASC
                 LIMIT 8",
                build_where_sql(&clauses)
            ),
            params,
        )?
    };

    Ok(DashboardStats {
        generated_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        available_start_date,
        available_end_date,
        overview: DashboardOverview {
            total_contacts,
            active_contacts,
            anonymized_contacts,
            contacts_to_delete,
            total_structures,
            total_categories,
            total_affiliations,
            total_reunions,
            period_reunions,
            period_presences,
            average_presences_per_reunion,
            rgpd_consent_rate,
            partner_direct_structures,
        },
        meetings_by_month,
        attendance_by_status,
        structures_by_category,
        contacts_by_commune,
        structures_by_commune,
        meetings_by_organisme,
        account_statuses,
        contacts_created_by_month,
        quality_checks,
        top_meetings,
        top_structures_presence_rate,
        top_structures_presence_volume,
        top_people_presence,
    })
}
