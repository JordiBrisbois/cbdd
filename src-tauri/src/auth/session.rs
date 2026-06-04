use super::passwords::verify_password;
use super::seeding::*;
use super::utils::{
    load_permissions_for_role, load_permissions_for_user, load_role_codes_for_user,
};
use crate::models::CurrentSession;
use once_cell::sync::Lazy;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::Mutex;

pub(crate) static SESSION_STATE: Lazy<Mutex<Option<i64>>> = Lazy::new(|| Mutex::new(None));

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
        all_permission_codes()
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
