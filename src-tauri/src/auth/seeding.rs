use super::passwords::hash_password;
use crate::models::*;
use rusqlite::{params, Connection, OptionalExtension};

static PERMISSIONS: &[(&str, &str)] = &[
    ("personnes.read", "Consulter les personnes"),
    ("personnes.create", "Créer des personnes"),
    ("personnes.update", "Modifier des personnes"),
    ("personnes.delete", "Supprimer des personnes"),
    ("structures.read", "Consulter les structures"),
    ("structures.create", "Créer des structures"),
    ("structures.update", "Modifier des structures"),
    ("structures.delete", "Supprimer des structures"),
    ("categories.read", "Consulter les catégories"),
    ("categories.create", "Créer des catégories"),
    ("categories.update", "Modifier des catégories"),
    ("categories.delete", "Supprimer des catégories"),
    ("affiliations.read", "Consulter les affiliations"),
    ("affiliations.create", "Créer des affiliations"),
    ("affiliations.update", "Modifier des affiliations"),
    ("affiliations.delete", "Supprimer des affiliations"),
    ("reunions.read", "Consulter les réunions"),
    ("reunions.create", "Créer des réunions"),
    ("reunions.update", "Modifier des réunions"),
    ("reunions.delete", "Supprimer des réunions"),
    ("presences.read", "Consulter les présences"),
    ("presences.create", "Créer des présences"),
    ("presences.update", "Modifier des présences"),
    ("presences.delete", "Supprimer des présences"),
    ("rgpd.read", "Consulter le RGPD"),
    ("rgpd.update", "Modifier les statuts RGPD"),
    ("rgpd.anonymize", "Anonymiser des personnes"),
    ("rgpd.anonymize.bulk", "Anonymiser des personnes en masse"),
    ("stats.read", "Consulter les statistiques"),
    ("search.read", "Utiliser la recherche avancée"),
    ("presets.read", "Consulter les presets"),
    ("presets.create", "Créer des presets"),
    ("presets.update", "Modifier des presets"),
    ("presets.delete", "Supprimer des presets"),
    ("admin.users", "Gérer les comptes"),
    ("admin.roles", "Gérer les rôles"),
    ("admin.settings", "Gérer les réglages de sécurité"),
    ("admin.exports", "Exporter les données métier"),
    ("admin.backups", "Gérer les sauvegardes et restaurations"),
];

pub const ADMIN_ROLE_CODE: &str = "ADMIN";
pub const PUBLIC_ROLE_CODE: &str = "PUBLIC";
pub const ANON_SETTING_KEY: &str = "anonymous_access_enabled";
pub const SEEDED_ADMIN_USERNAME: &str = "admin@admin.com";
pub const SEEDED_ADMIN_PASSWORD: &str = "admin";
pub const LEGACY_ADMIN_USERNAME: &str = "jordi@brisbois.dev";

pub const DEFAULT_PUBLIC_PERMISSIONS: &[&str] = &[
    "personnes.read",
    "structures.read",
    "categories.read",
    "affiliations.read",
    "reunions.read",
    "presences.read",
    "search.read",
    "presets.read",
];

pub fn list_permissions() -> Vec<Permission> {
    PERMISSIONS
        .iter()
        .map(|(code, description)| Permission {
            code: (*code).to_string(),
            description: (*description).to_string(),
        })
        .collect()
}

pub fn all_permission_codes() -> Vec<String> {
    PERMISSIONS
        .iter()
        .map(|(code, _)| (*code).to_string())
        .collect()
}

pub fn seed_permissions(conn: &Connection) -> Result<(), String> {
    for (code, description) in PERMISSIONS {
        conn.execute(
            "INSERT OR IGNORE INTO T_Permissions (Code_Permission, Description) VALUES (?, ?)",
            params![code, description],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn seed_system_roles(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "INSERT OR IGNORE INTO T_Roles (Code_Role, Nom_Role, Is_System) VALUES (?, ?, 1)",
        params![ADMIN_ROLE_CODE, "Administrateur"],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR IGNORE INTO T_Roles (Code_Role, Nom_Role, Is_System) VALUES (?, ?, 1)",
        params![PUBLIC_ROLE_CODE, "Public"],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn seed_public_permissions(conn: &Connection) -> Result<(), String> {
    let public_role_id: i64 = conn
        .query_row(
            "SELECT ID_Role FROM T_Roles WHERE Code_Role = ?",
            params![PUBLIC_ROLE_CODE],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    for code in DEFAULT_PUBLIC_PERMISSIONS {
        let permission_id: i64 = conn
            .query_row(
                "SELECT ID_Permission FROM T_Permissions WHERE Code_Permission = ?",
                params![code],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT OR IGNORE INTO T_RolePermissions (Ref_Role, Ref_Permission) VALUES (?, ?)",
            params![public_role_id, permission_id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn seed_statistics_permissions(conn: &Connection) -> Result<(), String> {
    let stats_permission_id: i64 = conn
        .query_row(
            "SELECT ID_Permission FROM T_Permissions WHERE Code_Permission = ?",
            params!["stats.read"],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT rp.Ref_Role
             FROM T_RolePermissions rp
             INNER JOIN T_Permissions p ON p.ID_Permission = rp.Ref_Permission
             WHERE p.Code_Permission = ?",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params!["search.read"], |row| row.get::<_, i64>(0))
        .map_err(|e| e.to_string())?;

    for row in rows {
        let role_id = row.map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR IGNORE INTO T_RolePermissions (Ref_Role, Ref_Permission) VALUES (?, ?)",
            params![role_id, stats_permission_id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn seed_default_settings(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "INSERT OR IGNORE INTO T_AppSettings (Setting_Key, Setting_Value) VALUES (?, ?)",
        params![ANON_SETTING_KEY, "1"],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn seed_admin_user(conn: &Connection) -> Result<(), String> {
    let user_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM T_Users", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    if user_count == 0 {
        let hash = hash_password(SEEDED_ADMIN_PASSWORD)?;
        conn.execute(
            "INSERT INTO T_Users (Username, Password_Hash, Display_Name, Is_Active, Must_Change_Password)
             VALUES (?, ?, ?, 1, 1)",
            params![SEEDED_ADMIN_USERNAME, hash, "Administrateur"],
        )
        .map_err(|e| e.to_string())?;

        let admin_role_id: i64 = conn
            .query_row(
                "SELECT ID_Role FROM T_Roles WHERE Code_Role = ?",
                params![ADMIN_ROLE_CODE],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT OR IGNORE INTO T_UserRoles (Ref_User, Ref_Role) VALUES (?, ?)",
            params![conn.last_insert_rowid(), admin_role_id],
        )
        .map_err(|e| e.to_string())?;

        return Ok(());
    }

    let legacy_id: Option<i64> = conn
        .query_row(
            "SELECT ID_User FROM T_Users WHERE Username = ?",
            params![LEGACY_ADMIN_USERNAME],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let existing_id: Option<i64> = conn
        .query_row(
            "SELECT ID_User FROM T_Users WHERE Username = ?",
            params![SEEDED_ADMIN_USERNAME],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if user_count == 1 && existing_id.is_none() {
        if let Some(id) = legacy_id {
            conn.execute(
                "UPDATE T_Users
                 SET Username = ?, Display_Name = ?, Must_Change_Password = 1, Updated_At = datetime('now')
                 WHERE ID_User = ?",
                params![SEEDED_ADMIN_USERNAME, "Administrateur", id],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    if count_admin_users_impl(conn)? == 0 {
        recover_admin_access_impl(conn, None)?;
    }

    Ok(())
}

fn count_admin_users_impl(conn: &Connection) -> Result<i64, String> {
    conn.query_row(
        "SELECT COUNT(DISTINCT ur.Ref_User)
         FROM T_UserRoles ur
         INNER JOIN T_Roles r ON r.ID_Role = ur.Ref_Role
         INNER JOIN T_Users u ON u.ID_User = ur.Ref_User
         WHERE r.Code_Role = ? AND u.Is_Active = 1",
        params![ADMIN_ROLE_CODE],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

fn get_admin_role_id(conn: &Connection) -> Result<i64, String> {
    conn.query_row(
        "SELECT ID_Role FROM T_Roles WHERE Code_Role = ?",
        params![ADMIN_ROLE_CODE],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

pub fn recover_admin_access(
    conn: &Connection,
    preferred_username: Option<&str>,
) -> Result<Option<String>, String> {
    recover_admin_access_impl(conn, preferred_username)
}

fn recover_admin_access_impl(
    conn: &Connection,
    preferred_username: Option<&str>,
) -> Result<Option<String>, String> {
    if count_admin_users_impl(conn)? > 0 {
        return Ok(None);
    }

    let admin_role_id = get_admin_role_id(conn)?;

    if let Some(username) = preferred_username
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let preferred_user_id: Option<i64> = conn
            .query_row(
                "SELECT ID_User FROM T_Users WHERE Username = ? AND Is_Active = 1",
                params![username],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        if let Some(user_id) = preferred_user_id {
            conn.execute(
                "INSERT OR IGNORE INTO T_UserRoles (Ref_User, Ref_Role) VALUES (?, ?)",
                params![user_id, admin_role_id],
            )
            .map_err(|e| e.to_string())?;
            return Ok(Some(username.to_string()));
        }
    }

    let seeded_user_id: Option<i64> = conn
        .query_row(
            "SELECT ID_User FROM T_Users WHERE Username = ?",
            params![SEEDED_ADMIN_USERNAME],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let user_id = if let Some(user_id) = seeded_user_id {
        let hash = hash_password(SEEDED_ADMIN_PASSWORD)?;
        conn.execute(
            "UPDATE T_Users
             SET Password_Hash = ?, Display_Name = ?, Is_Active = 1, Must_Change_Password = 1, Updated_At = datetime('now')
             WHERE ID_User = ?",
            params![hash, "Administrateur", user_id],
        )
        .map_err(|e| e.to_string())?;
        user_id
    } else {
        let hash = hash_password(SEEDED_ADMIN_PASSWORD)?;
        conn.execute(
            "INSERT INTO T_Users (Username, Password_Hash, Display_Name, Is_Active, Must_Change_Password)
             VALUES (?, ?, ?, 1, 1)",
            params![SEEDED_ADMIN_USERNAME, hash, "Administrateur"],
        )
        .map_err(|e| e.to_string())?;
        conn.last_insert_rowid()
    };

    conn.execute(
        "INSERT OR IGNORE INTO T_UserRoles (Ref_User, Ref_Role) VALUES (?, ?)",
        params![user_id, admin_role_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(Some(SEEDED_ADMIN_USERNAME.to_string()))
}
