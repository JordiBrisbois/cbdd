use super::*;

#[tauri::command]
pub fn acquire_edit_lock(
    app: AppHandle,
    resource_type: String,
    resource_id: i64,
) -> Result<EditLockStatus, String> {
    let conn = db::get_conn(&app)?;
    let permission = permission_for_resource(&resource_type)
        .ok_or_else(|| "Type de ressource de verrou inconnu".to_string())?;
    auth::require_permission(&conn, permission)?;

    conn.execute(
        "DELETE FROM T_EditLocks WHERE Expires_At <= datetime('now')",
        [],
    )
    .map_err(|e| e.to_string())?;

    let (user_id, holder_label) = auth::current_actor_label(&conn)?;
    let machine = machine_label();

    let existing = conn
        .query_row(
            "SELECT Holder_User_Id, Holder_Label, Expires_At
             FROM T_EditLocks
             WHERE Resource_Type = ? AND Resource_Id = ?",
            rusqlite::params![resource_type, resource_id],
            |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if let Some((existing_user_id, existing_label, expires_at)) = existing {
        if existing_user_id == user_id {
            conn.execute(
                "UPDATE T_EditLocks
                 SET Holder_Label = ?, Machine_Label = ?, Acquired_At = datetime('now'),
                     Expires_At = datetime('now', ?)
                 WHERE Resource_Type = ? AND Resource_Id = ?",
                rusqlite::params![
                    holder_label,
                    machine,
                    format!("+{} minutes", EDIT_LOCK_MINUTES),
                    resource_type,
                    resource_id
                ],
            )
            .map_err(|e| e.to_string())?;
            return Ok(EditLockStatus {
                resource_type,
                resource_id,
                acquired: true,
                holder_label: Some(existing_label),
                expires_at: Some(expires_at),
            });
        }

        return Ok(EditLockStatus {
            resource_type,
            resource_id,
            acquired: false,
            holder_label: Some(existing_label),
            expires_at: Some(expires_at),
        });
    }

    let expires_at = conn
        .query_row(
            "SELECT datetime('now', ?)",
            rusqlite::params![format!("+{} minutes", EDIT_LOCK_MINUTES)],
            |row| row.get::<_, String>(0),
        )
        .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO T_EditLocks (Resource_Type, Resource_Id, Holder_User_Id, Holder_Label, Machine_Label, Expires_At)
         VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params![resource_type, resource_id, user_id, holder_label, machine, expires_at],
    )
    .map_err(|e| e.to_string())?;

    Ok(EditLockStatus {
        resource_type,
        resource_id,
        acquired: true,
        holder_label: None,
        expires_at: Some(expires_at),
    })
}

#[tauri::command]
pub fn release_edit_lock(
    app: AppHandle,
    resource_type: String,
    resource_id: i64,
) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    let permission = permission_for_resource(&resource_type)
        .ok_or_else(|| "Type de ressource de verrou inconnu".to_string())?;
    auth::require_permission(&conn, permission)?;

    let (user_id, _) = auth::current_actor_label(&conn)?;
    conn.execute(
        "DELETE FROM T_EditLocks
         WHERE Resource_Type = ? AND Resource_Id = ? AND Holder_User_Id IS ?",
        rusqlite::params![resource_type, resource_id, user_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
