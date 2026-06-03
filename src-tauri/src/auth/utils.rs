use rusqlite::{params, Connection, OptionalExtension};
use crate::models::{CurrentSession, SecuritySettings};
use super::seeding::*;

pub fn require_permission(conn: &Connection, permission: &str) -> Result<CurrentSession, String> {
    let session = super::session::get_current_session(conn)?;
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
    let session = super::session::get_current_session(conn)?;
    let label = session
        .display_name
        .or(session.username)
        .unwrap_or_else(|| "Utilisateur".to_string());
    Ok((session.user_id, label))
}

pub fn get_security_settings(conn: &Connection) -> Result<SecuritySettings, String> {
    Ok(SecuritySettings {
        anonymous_access_enabled: get_anonymous_enabled(conn)?,
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

fn get_anonymous_enabled(conn: &Connection) -> Result<bool, String> {
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

// ── ROLE helpers ──

pub fn load_permission_codes_for_role_id(
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

pub fn load_role_ids_for_user(conn: &Connection, user_id: i64) -> Result<Vec<i64>, String> {
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

pub fn load_role_codes_for_user(conn: &Connection, user_id: i64) -> Result<Vec<String>, String> {
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

pub fn load_permissions_for_user(conn: &Connection, user_id: i64) -> Result<Vec<String>, String> {
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

pub fn load_permissions_for_role(conn: &Connection, role_code: &str) -> Result<Vec<String>, String> {
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
