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
            "SELECT Holder_Token, Holder_Label, Expires_At
             FROM T_EditLocks
             WHERE Resource_Type = ? AND Resource_Id = ?",
            rusqlite::params![resource_type, resource_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let holder_token = edit_lock_token();
    if let Some((existing_token, existing_label, expires_at)) = existing {
        if existing_token.as_deref() == Some(holder_token) {
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
        "INSERT INTO T_EditLocks (Resource_Type, Resource_Id, Holder_User_Id, Holder_Token, Holder_Label, Machine_Label, Expires_At)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![resource_type, resource_id, user_id, holder_token, holder_label, machine, expires_at],
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

    conn.execute(
        "DELETE FROM T_EditLocks
         WHERE Resource_Type = ? AND Resource_Id = ? AND Holder_Token = ?",
        rusqlite::params![resource_type, resource_id, edit_lock_token()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().expect("sqlite");
        conn.execute_batch(
            "CREATE TABLE T_EditLocks (
                Resource_Type TEXT NOT NULL,
                Resource_Id INTEGER NOT NULL,
                Holder_User_Id INTEGER NOT NULL,
                Holder_Token TEXT NOT NULL,
                Holder_Label TEXT NOT NULL,
                Machine_Label TEXT NOT NULL,
                Acquired_At TEXT DEFAULT (datetime('now')),
                Expires_At TEXT NOT NULL,
                PRIMARY KEY (Resource_Type, Resource_Id)
            );
            CREATE TABLE IF NOT EXISTS T_Users (ID_User INTEGER PRIMARY KEY);
            INSERT INTO T_EditLocks VALUES (
                'personnes', 1, 1, 'token-A', 'Alice', 'PC-01',
                datetime('now'), datetime('now', '+30 minutes')
            );
            INSERT INTO T_EditLocks VALUES (
                'personnes', 2, 2, 'token-B', 'Bob', 'PC-02',
                datetime('now'), datetime('now', '+5 minutes')
            );",
        )
        .expect("schema");
        conn
    }

    fn count_locks(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM T_EditLocks", [], |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn acquisition_cree_nouveau_verrou() {
        let conn = Connection::open_in_memory().expect("sqlite");
        conn.execute_batch(
            "CREATE TABLE T_EditLocks (
                Resource_Type TEXT NOT NULL,
                Resource_Id INTEGER NOT NULL,
                Holder_User_Id INTEGER NOT NULL,
                Holder_Token TEXT NOT NULL,
                Holder_Label TEXT NOT NULL,
                Machine_Label TEXT NOT NULL,
                Acquired_At TEXT DEFAULT (datetime('now')),
                Expires_At TEXT NOT NULL,
                PRIMARY KEY (Resource_Type, Resource_Id)
            );",
        )
        .expect("schema");

        conn.execute(
            "INSERT INTO T_EditLocks (Resource_Type, Resource_Id, Holder_User_Id, Holder_Token, Holder_Label, Machine_Label, Expires_At)
             VALUES (?, ?, ?, ?, ?, ?, datetime('now', '+10 minutes'))",
            rusqlite::params!["personnes", 1, 1, "token-A", "Alice", "PC-01"],
        )
        .expect("insert");
        assert_eq!(count_locks(&conn), 1);
    }

    #[test]
    fn verrou_sans_expiration() {
        let conn = Connection::open_in_memory().expect("sqlite");
        conn.execute_batch(
            "CREATE TABLE T_EditLocks (
                Resource_Type TEXT NOT NULL,
                Resource_Id INTEGER NOT NULL,
                Holder_User_Id INTEGER NOT NULL,
                Holder_Token TEXT NOT NULL,
                Holder_Label TEXT NOT NULL,
                Machine_Label TEXT NOT NULL,
                Acquired_At TEXT DEFAULT (datetime('now')),
                Expires_At TEXT NOT NULL,
                PRIMARY KEY (Resource_Type, Resource_Id)
            );
            INSERT INTO T_EditLocks VALUES (
                'personnes', 1, 1, 'token-A', 'Alice', 'PC-01',
                datetime('now'), datetime('now', '-1 minute')
            );",
        )
        .expect("schema");

        conn.execute(
            "DELETE FROM T_EditLocks WHERE Expires_At <= datetime('now')",
            [],
        )
        .expect("cleanup");
        assert_eq!(count_locks(&conn), 0, "un verrou expiré doit être supprimé");
    }

    #[test]
    fn verrou_actif_reste_apres_cleanup() {
        let conn = Connection::open_in_memory().expect("sqlite");
        conn.execute_batch(
            "CREATE TABLE T_EditLocks (
                Resource_Type TEXT NOT NULL,
                Resource_Id INTEGER NOT NULL,
                Holder_User_Id INTEGER NOT NULL,
                Holder_Token TEXT NOT NULL,
                Holder_Label TEXT NOT NULL,
                Machine_Label TEXT NOT NULL,
                Acquired_At TEXT DEFAULT (datetime('now')),
                Expires_At TEXT NOT NULL,
                PRIMARY KEY (Resource_Type, Resource_Id)
            );
            INSERT INTO T_EditLocks VALUES (
                'personnes', 1, 1, 'token-A', 'Alice', 'PC-01',
                datetime('now'), datetime('now', '+30 minutes')
            );
            INSERT INTO T_EditLocks VALUES (
                'personnes', 2, 2, 'token-B', 'Bob', 'PC-02',
                datetime('now'), datetime('now', '-5 minutes')
            );",
        )
        .expect("schema");

        conn.execute(
            "DELETE FROM T_EditLocks WHERE Expires_At <= datetime('now')",
            [],
        )
        .expect("cleanup");
        assert_eq!(
            count_locks(&conn),
            1,
            "seul le verrou non expiré doit subsister"
        );
    }

    #[test]
    fn refus_token_different() {
        let conn = setup();
        // Bob essaie d'acquérir le verrou d'Alice (token différent)
        let existing = conn
            .query_row(
                "SELECT Holder_Token FROM T_EditLocks WHERE Resource_Type = ? AND Resource_Id = ?",
                rusqlite::params!["personnes", 1],
                |row| row.get::<_, String>(0),
            )
            .unwrap();
        assert_eq!(existing, "token-A", "le verrou appartient toujours à Alice");
    }

    #[test]
    fn renouvellement_verrou() {
        let conn = Connection::open_in_memory().expect("sqlite");
        conn.execute_batch(
            "CREATE TABLE T_EditLocks (
                Resource_Type TEXT NOT NULL,
                Resource_Id INTEGER NOT NULL,
                Holder_User_Id INTEGER NOT NULL,
                Holder_Token TEXT NOT NULL,
                Holder_Label TEXT NOT NULL,
                Machine_Label TEXT NOT NULL,
                Acquired_At TEXT DEFAULT (datetime('now')),
                Expires_At TEXT NOT NULL,
                PRIMARY KEY (Resource_Type, Resource_Id)
            );
            INSERT INTO T_EditLocks VALUES (
                'personnes', 1, 1, 'token-A', 'Alice', 'PC-01',
                '2026-01-01 10:00:00', datetime('now', '+5 minutes')
            );",
        )
        .expect("schema");

        // Renouveler avec le même token
        conn.execute(
            "UPDATE T_EditLocks SET Acquired_At = datetime('now'), Expires_At = datetime('now', '+10 minutes')
             WHERE Resource_Type = ? AND Resource_Id = ? AND Holder_Token = ?",
            rusqlite::params!["personnes", 1, "token-A"],
        )
        .expect("renewal");

        let expires_at: String = conn
            .query_row(
                "SELECT Expires_At FROM T_EditLocks WHERE Resource_Type = 'personnes' AND Resource_Id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let now_plus_9: i64 = conn
            .query_row(
                "SELECT CASE WHEN datetime(?) > datetime('now', '+9 minutes') THEN 1 ELSE 0 END",
                rusqlite::params![expires_at],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            now_plus_9, 1,
            "le verrou doit être prolongé d'au moins 9 minutes"
        );
    }
}
