use crate::models::{
    CurrentSession, Permission, RoleDetails, RoleInput, SecuritySettings, UserInput, UserSummary,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use once_cell::sync::Lazy;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::BTreeSet;
use std::sync::Mutex;

static SESSION_STATE: Lazy<Mutex<Option<i64>>> = Lazy::new(|| Mutex::new(None));

const ADMIN_ROLE_CODE: &str = "ADMIN";
const PUBLIC_ROLE_CODE: &str = "PUBLIC";
const ANON_SETTING_KEY: &str = "anonymous_access_enabled";
const SEEDED_ADMIN_USERNAME: &str = "admin@admin.com";
const SEEDED_ADMIN_PASSWORD: &str = "admin";
const LEGACY_ADMIN_USERNAME: &str = "jordi@brisbois.dev";

const PERMISSIONS: &[(&str, &str)] = &[
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

const DEFAULT_PUBLIC_PERMISSIONS: &[&str] = &[
    "personnes.read",
    "structures.read",
    "categories.read",
    "affiliations.read",
    "reunions.read",
    "presences.read",
    "search.read",
    "presets.read",
];

pub fn ensure_security_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS T_Users (
            ID_User INTEGER PRIMARY KEY AUTOINCREMENT,
            Username TEXT NOT NULL UNIQUE,
            Password_Hash TEXT NOT NULL,
            Display_Name TEXT,
            Is_Active INTEGER NOT NULL DEFAULT 1,
            Must_Change_Password INTEGER NOT NULL DEFAULT 0,
            Created_At TEXT NOT NULL DEFAULT (datetime('now')),
            Updated_At TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS T_Roles (
            ID_Role INTEGER PRIMARY KEY AUTOINCREMENT,
            Code_Role TEXT NOT NULL UNIQUE,
            Nom_Role TEXT NOT NULL,
            Is_System INTEGER NOT NULL DEFAULT 0,
            Created_At TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS T_Permissions (
            ID_Permission INTEGER PRIMARY KEY AUTOINCREMENT,
            Code_Permission TEXT NOT NULL UNIQUE,
            Description TEXT
        );

        CREATE TABLE IF NOT EXISTS T_UserRoles (
            Ref_User INTEGER NOT NULL,
            Ref_Role INTEGER NOT NULL,
            PRIMARY KEY (Ref_User, Ref_Role),
            FOREIGN KEY (Ref_User) REFERENCES T_Users(ID_User) ON DELETE CASCADE,
            FOREIGN KEY (Ref_Role) REFERENCES T_Roles(ID_Role) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS T_RolePermissions (
            Ref_Role INTEGER NOT NULL,
            Ref_Permission INTEGER NOT NULL,
            PRIMARY KEY (Ref_Role, Ref_Permission),
            FOREIGN KEY (Ref_Role) REFERENCES T_Roles(ID_Role) ON DELETE CASCADE,
            FOREIGN KEY (Ref_Permission) REFERENCES T_Permissions(ID_Permission) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS T_AppSettings (
            Setting_Key TEXT PRIMARY KEY,
            Setting_Value TEXT NOT NULL
        );
        ",
    )
    .map_err(|e| format!("Erreur création sécurité: {}", e))?;

    seed_permissions(conn)?;
    seed_system_roles(conn)?;
    seed_public_permissions(conn)?;
    seed_statistics_permissions(conn)?;
    seed_default_settings(conn)?;
    seed_admin_user(conn)?;
    ensure_concurrency_schema(conn)?;

    Ok(())
}

fn ensure_concurrency_schema(conn: &Connection) -> Result<(), String> {
    ensure_column(conn, "T_Personnes", "Updated_At", "TEXT")?;
    ensure_column(conn, "T_Structures", "Updated_At", "TEXT")?;
    ensure_column(conn, "T_Affiliations", "Updated_At", "TEXT")?;
    ensure_column(conn, "T_Reunions", "Updated_At", "TEXT")?;

    conn.execute_batch(
        "
        UPDATE T_Personnes SET Updated_At = COALESCE(Updated_At, Date_Creation, datetime('now'));
        UPDATE T_Structures SET Updated_At = COALESCE(Updated_At, Date_Creation, datetime('now'));
        UPDATE T_Affiliations SET Updated_At = COALESCE(Updated_At, datetime('now'));
        UPDATE T_Reunions SET Updated_At = COALESCE(Updated_At, datetime('now'));

        CREATE TABLE IF NOT EXISTS T_EditLocks (
            Resource_Type TEXT NOT NULL,
            Resource_Id INTEGER NOT NULL,
            Holder_User_Id INTEGER,
            Holder_Label TEXT NOT NULL,
            Machine_Label TEXT,
            Acquired_At TEXT NOT NULL DEFAULT (datetime('now')),
            Expires_At TEXT NOT NULL,
            PRIMARY KEY (Resource_Type, Resource_Id)
        );

        CREATE INDEX IF NOT EXISTS idx_edit_locks_expiry ON T_EditLocks(Expires_At);

        CREATE TABLE IF NOT EXISTS T_BackupState (
            Lock_Key TEXT PRIMARY KEY,
            Holder_Label TEXT,
            Machine_Label TEXT,
            Expires_At TEXT,
            Last_Success_At TEXT
        );
        ",
    )
    .map_err(|e| format!("Erreur schéma concurrence: {}", e))?;

    Ok(())
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({})", table))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?;

    for row in rows {
        if row.map_err(|e| e.to_string())? == column {
            return Ok(());
        }
    }

    conn.execute(
        &format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition),
        [],
    )
    .map_err(|e| format!("Erreur ajout colonne {}.{}: {}", table, column, e))?;
    Ok(())
}

fn seed_permissions(conn: &Connection) -> Result<(), String> {
    for (code, description) in PERMISSIONS {
        conn.execute(
            "INSERT OR IGNORE INTO T_Permissions (Code_Permission, Description) VALUES (?, ?)",
            params![code, description],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn seed_system_roles(conn: &Connection) -> Result<(), String> {
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

fn seed_public_permissions(conn: &Connection) -> Result<(), String> {
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

fn seed_statistics_permissions(conn: &Connection) -> Result<(), String> {
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

fn seed_default_settings(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "INSERT OR IGNORE INTO T_AppSettings (Setting_Key, Setting_Value) VALUES (?, ?)",
        params![ANON_SETTING_KEY, "1"],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn seed_admin_user(conn: &Connection) -> Result<(), String> {
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

    if count_admin_users(conn)? == 0 {
        recover_admin_access(conn, None)?;
    }

    Ok(())
}

pub fn recover_admin_access(
    conn: &Connection,
    preferred_username: Option<&str>,
) -> Result<Option<String>, String> {
    if count_admin_users(conn)? > 0 {
        return Ok(None);
    }

    let admin_role_id = get_admin_role_id(conn)?;

    if let Some(username) = preferred_username.map(str::trim).filter(|value| !value.is_empty()) {
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

pub fn login(conn: &Connection, username: &str, password: &str) -> Result<CurrentSession, String> {
    let row = conn
        .query_row(
            "SELECT ID_User, Password_Hash, Is_Active FROM T_Users WHERE Username = ?",
            params![username],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let (user_id, password_hash, is_active) =
        row.ok_or_else(|| "Identifiants invalides".to_string())?;
    if !is_active {
        return Err("Ce compte est désactivé".into());
    }
    if !verify_password(password, &password_hash)? {
        return Err("Identifiants invalides".into());
    }

    *SESSION_STATE.lock().map_err(|e| e.to_string())? = Some(user_id);
    get_current_session(conn)
}

pub fn logout() -> Result<(), String> {
    *SESSION_STATE.lock().map_err(|e| e.to_string())? = None;
    Ok(())
}

pub fn get_current_session(conn: &Connection) -> Result<CurrentSession, String> {
    let anonymous_enabled = get_anonymous_access_enabled(conn)?;
    let current_user_id = *SESSION_STATE.lock().map_err(|e| e.to_string())?;

    if let Some(user_id) = current_user_id {
        if let Some(session) = load_user_session(conn, user_id)? {
            return Ok(session);
        }
        *SESSION_STATE.lock().map_err(|e| e.to_string())? = None;
    }

    let permissions = if anonymous_enabled {
        load_permissions_for_role(conn, PUBLIC_ROLE_CODE)?
    } else {
        Vec::new()
    };

    Ok(CurrentSession {
        is_authenticated: false,
        is_anonymous: true,
        anonymous_access_enabled: anonymous_enabled,
        must_change_password: false,
        user_id: None,
        username: None,
        display_name: Some("Public".into()),
        role_codes: vec![PUBLIC_ROLE_CODE.into()],
        permissions,
    })
}

fn load_user_session(conn: &Connection, user_id: i64) -> Result<Option<CurrentSession>, String> {
    let row = conn
        .query_row(
            "SELECT Username, Display_Name, Is_Active, Must_Change_Password FROM T_Users WHERE ID_User = ?",
            params![user_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, bool>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let Some((username, display_name, is_active, must_change_password)) = row else {
        return Ok(None);
    };

    if !is_active {
        return Ok(None);
    }

    let role_codes = load_role_codes_for_user(conn, user_id)?;
    let is_admin = role_codes.iter().any(|code| code == ADMIN_ROLE_CODE);
    let permissions = if is_admin {
        PERMISSIONS
            .iter()
            .map(|(code, _)| (*code).to_string())
            .collect()
    } else {
        load_permissions_for_user(conn, user_id)?
    };

    Ok(Some(CurrentSession {
        is_authenticated: true,
        is_anonymous: false,
        anonymous_access_enabled: get_anonymous_access_enabled(conn)?,
        must_change_password,
        user_id: Some(user_id),
        username: Some(username),
        display_name,
        role_codes,
        permissions,
    }))
}

pub fn require_permission(conn: &Connection, permission: &str) -> Result<CurrentSession, String> {
    let session = get_current_session(conn)?;
    let is_admin = session
        .role_codes
        .iter()
        .any(|code| code == ADMIN_ROLE_CODE);
    if is_admin || session.permissions.iter().any(|p| p == permission) {
        return Ok(session);
    }
    Err(format!("Permission refusée: {}", permission))
}

pub fn current_actor_label(conn: &Connection) -> Result<(Option<i64>, String), String> {
    let session = get_current_session(conn)?;
    let label = session
        .display_name
        .or(session.username)
        .unwrap_or_else(|| "Utilisateur".to_string());
    Ok((session.user_id, label))
}

pub fn list_permissions() -> Vec<Permission> {
    PERMISSIONS
        .iter()
        .map(|(code, description)| Permission {
            code: (*code).to_string(),
            description: (*description).to_string(),
        })
        .collect()
}

pub fn list_roles(conn: &Connection) -> Result<Vec<RoleDetails>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT ID_Role, Code_Role, Nom_Role, Is_System
             FROM T_Roles
             ORDER BY Is_System DESC, Nom_Role ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, bool>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        let (id_role, code_role, nom_role, is_system) = row.map_err(|e| e.to_string())?;
        result.push(RoleDetails {
            id_role,
            code_role,
            nom_role,
            is_system,
            permission_codes: load_permission_codes_for_role_id(conn, id_role)?,
        });
    }
    Ok(result)
}

pub fn save_role(conn: &Connection, role: RoleInput) -> Result<RoleDetails, String> {
    if role.nom_role.trim().is_empty() {
        return Err("Le nom du rôle est requis".into());
    }

    let permission_codes = dedupe_strings(role.permission_codes);

    let role_id = if let Some(id) = role.id_role {
        let (code_role, nom_role, is_system): (String, String, bool) = conn
            .query_row(
                "SELECT Code_Role, Nom_Role, Is_System FROM T_Roles WHERE ID_Role = ?",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| e.to_string())?;

        if code_role == ADMIN_ROLE_CODE {
            return Err("Le rôle Administrateur a déjà tous les accès".into());
        }

        conn.execute(
            "UPDATE T_Roles SET Nom_Role = ? WHERE ID_Role = ?",
            params![
                if is_system {
                    nom_role.as_str()
                } else {
                    role.nom_role.trim()
                },
                id
            ],
        )
        .map_err(|e| e.to_string())?;
        id
    } else {
        let base_code = sanitize_role_code(&role.nom_role);
        let mut code_role = base_code.clone();
        let mut index = 2;
        while conn
            .query_row(
                "SELECT 1 FROM T_Roles WHERE Code_Role = ?",
                params![code_role],
                |_row| Ok(()),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .is_some()
        {
            code_role = format!("{}_{}", base_code, index);
            index += 1;
        }

        conn.execute(
            "INSERT INTO T_Roles (Code_Role, Nom_Role, Is_System) VALUES (?, ?, 0)",
            params![code_role, role.nom_role.trim()],
        )
        .map_err(|e| e.to_string())?;
        conn.last_insert_rowid()
    };

    replace_role_permissions(conn, role_id, &permission_codes)?;
    get_role(conn, role_id)
}

pub fn delete_role(conn: &Connection, role_id: i64) -> Result<(), String> {
    let (code_role, is_system): (String, bool) = conn
        .query_row(
            "SELECT Code_Role, Is_System FROM T_Roles WHERE ID_Role = ?",
            params![role_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    if is_system || code_role == ADMIN_ROLE_CODE || code_role == PUBLIC_ROLE_CODE {
        return Err("Ce rôle système ne peut pas être supprimé".into());
    }

    conn.execute("DELETE FROM T_Roles WHERE ID_Role = ?", params![role_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_users(conn: &Connection) -> Result<Vec<UserSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT ID_User, Username, Display_Name, Is_Active, Must_Change_Password
             FROM T_Users
             ORDER BY Username ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        let (id_user, username, display_name, is_active, must_change_password) =
            row.map_err(|e| e.to_string())?;
        result.push(UserSummary {
            id_user,
            username,
            display_name,
            is_active,
            must_change_password,
            role_ids: load_role_ids_for_user(conn, id_user)?,
            role_codes: load_role_codes_for_user(conn, id_user)?,
        });
    }
    Ok(result)
}

pub fn save_user(conn: &Connection, user: UserInput) -> Result<UserSummary, String> {
    if user.username.trim().is_empty() {
        return Err("Le nom d'utilisateur est requis".into());
    }
    if user.role_ids.is_empty() {
        return Err("Au moins un rôle est requis".into());
    }

    let role_ids = dedupe_i64(user.role_ids);

    let user_id = if let Some(id) = user.id_user {
        let existing_role_codes = load_role_codes_for_user(conn, id)?;
        let is_currently_admin = existing_role_codes
            .iter()
            .any(|code| code == ADMIN_ROLE_CODE);
        if is_currently_admin && !user.is_active && count_admin_users(conn)? <= 1 {
            return Err("Impossible de désactiver le dernier administrateur actif".into());
        }

        conn.execute(
            "UPDATE T_Users
             SET Username = ?, Display_Name = ?, Is_Active = ?, Must_Change_Password = ?, Updated_At = datetime('now')
             WHERE ID_User = ?",
            params![
                user.username.trim(),
                empty_to_none(user.display_name.as_deref()),
                user.is_active,
                user.must_change_password,
                id
            ],
        )
        .map_err(|e| e.to_string())?;
        id
    } else {
        let password = user
            .password
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "Le mot de passe est requis".to_string())?;
        let hash = hash_password(password)?;
        conn.execute(
            "INSERT INTO T_Users (Username, Password_Hash, Display_Name, Is_Active, Must_Change_Password)
             VALUES (?, ?, ?, ?, ?)",
            params![
                user.username.trim(),
                hash,
                empty_to_none(user.display_name.as_deref()),
                user.is_active,
                user.must_change_password
            ],
        )
        .map_err(|e| e.to_string())?;
        conn.last_insert_rowid()
    };

    replace_user_roles(conn, user_id, &role_ids)?;
    get_user(conn, user_id)
}

pub fn delete_user(conn: &Connection, user_id: i64) -> Result<(), String> {
    let role_codes = load_role_codes_for_user(conn, user_id)?;
    if role_codes.iter().any(|code| code == ADMIN_ROLE_CODE) && count_admin_users(conn)? <= 1 {
        return Err("Impossible de supprimer le dernier administrateur".into());
    }

    conn.execute("DELETE FROM T_Users WHERE ID_User = ?", params![user_id])
        .map_err(|e| e.to_string())?;

    let current_user_id = *SESSION_STATE.lock().map_err(|e| e.to_string())?;
    if current_user_id == Some(user_id) {
        *SESSION_STATE.lock().map_err(|e| e.to_string())? = None;
    }

    Ok(())
}

pub fn admin_set_password(
    conn: &Connection,
    user_id: i64,
    new_password: &str,
) -> Result<(), String> {
    if new_password.trim().len() < 8 {
        return Err("Le mot de passe doit contenir au moins 8 caractères".into());
    }
    let hash = hash_password(new_password)?;
    conn.execute(
        "UPDATE T_Users
         SET Password_Hash = ?, Must_Change_Password = 0, Updated_At = datetime('now')
         WHERE ID_User = ?",
        params![hash, user_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn change_own_password(
    conn: &Connection,
    current_password: &str,
    new_password: &str,
) -> Result<(), String> {
    if new_password.trim().len() < 8 {
        return Err("Le nouveau mot de passe doit contenir au moins 8 caractères".into());
    }

    let user_id = SESSION_STATE
        .lock()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Aucun utilisateur connecté".to_string())?;

    let (password_hash, is_active): (String, bool) = conn
        .query_row(
            "SELECT Password_Hash, Is_Active FROM T_Users WHERE ID_User = ?",
            params![user_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    if !is_active {
        return Err("Ce compte est désactivé".into());
    }
    if !verify_password(current_password, &password_hash)? {
        return Err("Mot de passe actuel invalide".into());
    }
    if current_password.trim() == new_password.trim() {
        return Err("Le nouveau mot de passe doit être différent de l'actuel".into());
    }

    let hash = hash_password(new_password)?;
    conn.execute(
        "UPDATE T_Users
         SET Password_Hash = ?, Must_Change_Password = 0, Updated_At = datetime('now')
         WHERE ID_User = ?",
        params![hash, user_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_security_settings(conn: &Connection) -> Result<SecuritySettings, String> {
    Ok(SecuritySettings {
        anonymous_access_enabled: get_anonymous_access_enabled(conn)?,
    })
}

pub fn save_security_settings(
    conn: &Connection,
    settings: SecuritySettings,
) -> Result<SecuritySettings, String> {
    conn.execute(
        "INSERT INTO T_AppSettings (Setting_Key, Setting_Value) VALUES (?, ?)
         ON CONFLICT(Setting_Key) DO UPDATE SET Setting_Value = excluded.Setting_Value",
        params![
            ANON_SETTING_KEY,
            if settings.anonymous_access_enabled {
                "1"
            } else {
                "0"
            }
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(settings)
}

fn get_anonymous_access_enabled(conn: &Connection) -> Result<bool, String> {
    let value: Option<String> = conn
        .query_row(
            "SELECT Setting_Value FROM T_AppSettings WHERE Setting_Key = ?",
            params![ANON_SETTING_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(value.unwrap_or_else(|| "1".into()) == "1")
}

fn get_role(conn: &Connection, role_id: i64) -> Result<RoleDetails, String> {
    conn.query_row(
        "SELECT ID_Role, Code_Role, Nom_Role, Is_System FROM T_Roles WHERE ID_Role = ?",
        params![role_id],
        |row| {
            Ok(RoleDetails {
                id_role: row.get(0)?,
                code_role: row.get(1)?,
                nom_role: row.get(2)?,
                is_system: row.get(3)?,
                permission_codes: Vec::new(),
            })
        },
    )
    .map_err(|e| e.to_string())
    .and_then(|mut role| {
        role.permission_codes = load_permission_codes_for_role_id(conn, role.id_role)?;
        Ok(role)
    })
}

fn get_user(conn: &Connection, user_id: i64) -> Result<UserSummary, String> {
    conn.query_row(
        "SELECT ID_User, Username, Display_Name, Is_Active, Must_Change_Password
         FROM T_Users WHERE ID_User = ?",
        params![user_id],
        |row| {
            Ok(UserSummary {
                id_user: row.get(0)?,
                username: row.get(1)?,
                display_name: row.get(2)?,
                is_active: row.get(3)?,
                must_change_password: row.get(4)?,
                role_ids: Vec::new(),
                role_codes: Vec::new(),
            })
        },
    )
    .map_err(|e| e.to_string())
    .and_then(|mut user| {
        user.role_ids = load_role_ids_for_user(conn, user_id)?;
        user.role_codes = load_role_codes_for_user(conn, user_id)?;
        Ok(user)
    })
}

fn load_role_ids_for_user(conn: &Connection, user_id: i64) -> Result<Vec<i64>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT r.ID_Role
             FROM T_Roles r
             INNER JOIN T_UserRoles ur ON ur.Ref_Role = r.ID_Role
             WHERE ur.Ref_User = ?
             ORDER BY r.Nom_Role ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![user_id], |row| row.get::<_, i64>(0))
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

fn load_role_codes_for_user(conn: &Connection, user_id: i64) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT r.Code_Role
             FROM T_Roles r
             INNER JOIN T_UserRoles ur ON ur.Ref_Role = r.ID_Role
             WHERE ur.Ref_User = ?
             ORDER BY r.Nom_Role ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![user_id], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

fn load_permissions_for_user(conn: &Connection, user_id: i64) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT p.Code_Permission
             FROM T_Permissions p
             INNER JOIN T_RolePermissions rp ON rp.Ref_Permission = p.ID_Permission
             INNER JOIN T_UserRoles ur ON ur.Ref_Role = rp.Ref_Role
             WHERE ur.Ref_User = ?
             ORDER BY p.Code_Permission ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![user_id], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

fn load_permissions_for_role(conn: &Connection, role_code: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT p.Code_Permission
             FROM T_Permissions p
             INNER JOIN T_RolePermissions rp ON rp.Ref_Permission = p.ID_Permission
             INNER JOIN T_Roles r ON r.ID_Role = rp.Ref_Role
             WHERE r.Code_Role = ?
             ORDER BY p.Code_Permission ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![role_code], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

fn load_permission_codes_for_role_id(
    conn: &Connection,
    role_id: i64,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT p.Code_Permission
             FROM T_Permissions p
             INNER JOIN T_RolePermissions rp ON rp.Ref_Permission = p.ID_Permission
             WHERE rp.Ref_Role = ?
             ORDER BY p.Code_Permission ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![role_id], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

fn replace_role_permissions(
    conn: &Connection,
    role_id: i64,
    permission_codes: &[String],
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM T_RolePermissions WHERE Ref_Role = ?",
        params![role_id],
    )
    .map_err(|e| e.to_string())?;

    for permission_code in permission_codes {
        let permission_id: i64 = conn
            .query_row(
                "SELECT ID_Permission FROM T_Permissions WHERE Code_Permission = ?",
                params![permission_code],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT INTO T_RolePermissions (Ref_Role, Ref_Permission) VALUES (?, ?)",
            params![role_id, permission_id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn replace_user_roles(conn: &Connection, user_id: i64, role_ids: &[i64]) -> Result<(), String> {
    if load_role_codes_for_user(conn, user_id)?
        .iter()
        .any(|code| code == ADMIN_ROLE_CODE)
        && !role_ids.contains(&get_admin_role_id(conn)?)
        && count_admin_users(conn)? <= 1
    {
        return Err("Impossible de retirer le dernier administrateur".into());
    }

    conn.execute(
        "DELETE FROM T_UserRoles WHERE Ref_User = ?",
        params![user_id],
    )
    .map_err(|e| e.to_string())?;

    for role_id in role_ids {
        conn.execute(
            "INSERT INTO T_UserRoles (Ref_User, Ref_Role) VALUES (?, ?)",
            params![user_id, role_id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn get_admin_role_id(conn: &Connection) -> Result<i64, String> {
    conn.query_row(
        "SELECT ID_Role FROM T_Roles WHERE Code_Role = ?",
        params![ADMIN_ROLE_CODE],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

fn count_admin_users(conn: &Connection) -> Result<i64, String> {
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

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| format!("Erreur hash mot de passe: {}", e))
}

fn verify_password(password: &str, stored_hash: &str) -> Result<bool, String> {
    let parsed_hash = PasswordHash::new(stored_hash).map_err(|e| e.to_string())?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

fn sanitize_role_code(name: &str) -> String {
    let mut code = String::new();
    let mut last_was_sep = false;

    for ch in name.trim().chars() {
        let out = match ch {
            'a'..='z' => ch.to_ascii_uppercase(),
            'A'..='Z' | '0'..='9' => ch,
            'à' | 'â' | 'ä' => 'A',
            'ç' => 'C',
            'é' | 'è' | 'ê' | 'ë' => 'E',
            'î' | 'ï' => 'I',
            'ô' | 'ö' => 'O',
            'ù' | 'û' | 'ü' => 'U',
            _ => '_',
        };

        if out == '_' {
            if !last_was_sep && !code.is_empty() {
                code.push('_');
            }
            last_was_sep = true;
        } else {
            code.push(out);
            last_was_sep = false;
        }
    }

    let code = code.trim_matches('_').to_string();
    if code.is_empty() {
        "ROLE".into()
    } else {
        code
    }
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter_map(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() || !seen.insert(trimmed.clone()) {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect()
}

fn dedupe_i64(values: Vec<i64>) -> Vec<i64> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(*value))
        .collect()
}

fn empty_to_none(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}
