use super::passwords::hash_password;
use super::seeding::*;
use super::session::SESSION_STATE;
use super::utils::*;
use crate::models::*;
use rusqlite::{params, Connection};

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
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    let user_id = if let Some(id) = user.id_user {
        let existing_role_codes = load_role_codes_for_user(&tx, id)?;
        let is_currently_admin = existing_role_codes
            .iter()
            .any(|code| code == ADMIN_ROLE_CODE);
        if is_currently_admin && !user.is_active && count_admin_users(&tx)? <= 1 {
            return Err("Impossible de désactiver le dernier administrateur actif".into());
        }

        tx.execute(
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
        if password.chars().count() < 8 {
            return Err("Le mot de passe doit contenir au moins 8 caractères".into());
        }
        let hash = hash_password(password)?;
        tx.execute(
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
        tx.last_insert_rowid()
    };

    replace_user_roles_impl(&tx, user_id, &role_ids)?;
    tx.commit().map_err(|e| e.to_string())?;
    get_user_impl(conn, user_id)
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
    if !super::passwords::verify_password(current_password, &password_hash)? {
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

fn get_user_impl(conn: &Connection, user_id: i64) -> Result<UserSummary, String> {
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

fn replace_user_roles_impl(
    conn: &Connection,
    user_id: i64,
    role_ids: &[i64],
) -> Result<(), String> {
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

pub fn count_admin_users(conn: &Connection) -> Result<i64, String> {
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

fn dedupe_i64(values: Vec<i64>) -> Vec<i64> {
    let mut seen = std::collections::BTreeSet::new();
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
