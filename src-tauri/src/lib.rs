mod auth;
mod backups;
mod commands;
mod db;
mod excel_export;
mod models;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle();
            // 1. Try last saved path
            if let Some(path) = db::load_path(handle) {
                println!("[CRVI-GRC] Dernière BDD: {}", path);
                if let Err(e) = db::connect(&path) {
                    eprintln!("[CRVI-GRC] Impossible de connecter la dernière BDD: {}", e);
                    // 2. Try auto-discover
                    if let Some(discovered) = db::discover_db(handle) {
                        println!("[CRVI-GRC] BDD découverte: {}", discovered);
                        if let Err(e2) = db::connect(&discovered) {
                            eprintln!(
                                "[CRVI-GRC] Impossible de connecter la BDD découverte: {}",
                                e2
                            );
                        } else {
                            db::save_path(handle, &discovered);
                            println!("[CRVI-GRC] BDD découverte connectée automatiquement");
                        }
                    }
                } else {
                    println!("[CRVI-GRC] BDD connectée automatiquement");
                }
            } else {
                // 2. Try auto-discover
                if let Some(discovered) = db::discover_db(handle) {
                    println!("[CRVI-GRC] BDD découverte: {}", discovered);
                    if let Err(e) = db::connect(&discovered) {
                        eprintln!(
                            "[CRVI-GRC] Impossible de connecter la BDD découverte: {}",
                            e
                        );
                    } else {
                        db::save_path(handle, &discovered);
                        println!("[CRVI-GRC] BDD découverte connectée automatiquement");
                    }
                } else {
                    println!("[CRVI-GRC] Aucune BDD trouvée — utiliser 'Choisir une base'");
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_db_status,
            commands::pick_db,
            commands::reconnect_db,
            commands::get_current_session,
            commands::login,
            commands::logout,
            commands::lister_permissions,
            commands::lister_roles,
            commands::sauvegarder_role,
            commands::supprimer_role,
            commands::lister_users,
            commands::sauvegarder_user,
            commands::supprimer_user,
            commands::changer_mot_de_passe_user,
            commands::changer_mon_mot_de_passe,
            commands::get_security_settings,
            commands::sauvegarder_security_settings,
            commands::acquire_edit_lock,
            commands::release_edit_lock,
            commands::run_auto_backup_check,
            commands::create_manual_backup,
            commands::list_local_backups,
            commands::get_backup_directory,
            commands::open_backup_directory,
            commands::restore_local_backup,
            commands::delete_local_backup,
            commands::exporter_classeur_excel_admin,
            commands::lister_personnes,
            commands::get_personne,
            commands::get_personne_rgpd,
            commands::sauvegarder_personne,
            commands::supprimer_personne,
            commands::lister_structures,
            commands::get_structure,
            commands::sauvegarder_structure,
            commands::supprimer_structure,
            commands::lister_categories,
            commands::sauvegarder_categorie,
            commands::supprimer_categorie,
            commands::lister_fonctions,
            commands::sauvegarder_fonction,
            commands::supprimer_fonction,
            commands::lister_affiliations_personne,
            commands::lister_affiliations_structure,
            commands::get_affiliation,
            commands::sauvegarder_affiliation,
            commands::supprimer_affiliation,
            commands::rechercher_personnes_par_email,
            commands::rechercher_structures_par_email,
            commands::lister_reunions,
            commands::get_reunion,
            commands::sauvegarder_reunion,
            commands::supprimer_reunion,
            commands::lister_presences_reunion,
            commands::sauvegarder_presence,
            commands::supprimer_presence,
            commands::get_personnes_rgpd,
            commands::maj_statut_rgpd,
            commands::anonymiser_personne,
            commands::lister_personnes_refus_bdd_presence,
            commands::anonymiser_personnes_en_masse,
            commands::lister_personnes_categorie_detaillee,
            commands::get_dashboard_stats,
            commands::executer_requete,
            commands::lister_presets,
            commands::sauvegarder_preset,
            commands::supprimer_preset,
            commands::charger_preset,
        ])
        .run(tauri::generate_context!())
        .expect("Erreur lors du lancement de CRVI-GRC");
}
