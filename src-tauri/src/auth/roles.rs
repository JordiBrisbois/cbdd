use rusqlite::{params, Connection, OptionalExtension};
use super::seeding::*;
use super::utils::load_permission_codes_for_role_id;
use crate::models::*;
use std::collections::BTreeSet;

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
    get_role_impl(conn, role_id)
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

fn get_role_impl(conn: &Connection, role_id: i64) -> Result<RoleDetails, String> {
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
