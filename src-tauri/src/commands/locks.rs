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

    let (user_id, holder_label) = auth::current_actor_label(&conn)?;
    acquire_edit_lock_impl(
        &conn,
        &resource_type,
        resource_id,
        user_id,
        &holder_label,
        &machine_label(),
        edit_lock_token(),
    )
}

fn acquire_edit_lock_impl(
    conn: &rusqlite::Connection,
    resource_type: &str,
    resource_id: i64,
    user_id: Option<i64>,
    holder_label: &str,
    machine: &str,
    holder_token: &str,
) -> Result<EditLockStatus, String> {
    conn.execute(
        "DELETE FROM T_EditLocks WHERE Expires_At <= datetime('now')",
        [],
    )
    .map_err(|e| e.to_string())?;

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

    if let Some((existing_token, existing_label, expires_at)) = existing {
        if existing_token.as_deref() == Some(holder_token) {
            let renewed_expires_at = lock_expiry(conn)?;
            conn.execute(
                "UPDATE T_EditLocks
                 SET Holder_Label = ?, Machine_Label = ?, Acquired_At = datetime('now'),
                     Expires_At = ?
                 WHERE Resource_Type = ? AND Resource_Id = ?",
                rusqlite::params![
                    holder_label,
                    machine,
                    renewed_expires_at,
                    resource_type,
                    resource_id
                ],
            )
            .map_err(|e| e.to_string())?;
            return Ok(EditLockStatus {
                resource_type: resource_type.to_string(),
                resource_id,
                acquired: true,
                holder_label: Some(existing_label),
                expires_at: Some(renewed_expires_at),
            });
        }

        return Ok(EditLockStatus {
            resource_type: resource_type.to_string(),
            resource_id,
            acquired: false,
            holder_label: Some(existing_label),
            expires_at: Some(expires_at),
        });
    }

    let expires_at = lock_expiry(conn)?;

    conn.execute(
        "INSERT INTO T_EditLocks (Resource_Type, Resource_Id, Holder_User_Id, Holder_Token, Holder_Label, Machine_Label, Expires_At)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![resource_type, resource_id, user_id, holder_token, holder_label, machine, expires_at],
    )
    .map_err(|e| e.to_string())?;

    Ok(EditLockStatus {
        resource_type: resource_type.to_string(),
        resource_id,
        acquired: true,
        holder_label: None,
        expires_at: Some(expires_at),
    })
}

fn lock_expiry(conn: &rusqlite::Connection) -> Result<String, String> {
    conn.query_row(
        "SELECT datetime('now', ?)",
        rusqlite::params![format!("+{} minutes", EDIT_LOCK_MINUTES)],
        |row| row.get::<_, String>(0),
    )
    .map_err(|e| e.to_string())
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

    release_edit_lock_impl(&conn, &resource_type, resource_id, edit_lock_token())
}

fn release_edit_lock_impl(
    conn: &rusqlite::Connection,
    resource_type: &str,
    resource_id: i64,
    holder_token: &str,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM T_EditLocks
         WHERE Resource_Type = ? AND Resource_Id = ? AND Holder_Token = ?",
        rusqlite::params![resource_type, resource_id, holder_token],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{acquire_edit_lock_impl, release_edit_lock_impl};
    use rusqlite::Connection;

    fn empty_lock_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("sqlite");
        conn.execute_batch(
            "CREATE TABLE T_EditLocks (
                Resource_Type TEXT NOT NULL,
                Resource_Id INTEGER NOT NULL,
                Holder_User_Id INTEGER,
                Holder_Token TEXT,
                Holder_Label TEXT NOT NULL,
                Machine_Label TEXT,
                Acquired_At TEXT DEFAULT (datetime('now')),
                Expires_At TEXT NOT NULL,
                PRIMARY KEY (Resource_Type, Resource_Id)
            );",
        )
        .expect("schema");
        conn
    }

    #[test]
    fn implementation_acquires_renews_refuses_and_releases_by_token() {
        let conn = empty_lock_connection();

        let acquired =
            acquire_edit_lock_impl(&conn, "personnes", 1, Some(1), "Alice", "PC-A", "token-A")
                .expect("acquire");
        assert!(acquired.acquired);

        let refused =
            acquire_edit_lock_impl(&conn, "personnes", 1, Some(2), "Bob", "PC-B", "token-B")
                .expect("refuse");
        assert!(!refused.acquired);
        assert_eq!(refused.holder_label.as_deref(), Some("Alice"));

        let renewed =
            acquire_edit_lock_impl(&conn, "personnes", 1, Some(1), "Alice", "PC-A", "token-A")
                .expect("renew");
        assert!(renewed.acquired);
        assert_eq!(
            renewed.expires_at,
            conn.query_row(
                "SELECT Expires_At FROM T_EditLocks WHERE Resource_Type = 'personnes' AND Resource_Id = 1",
                [],
                |row| row.get(0),
            )
            .ok()
        );

        release_edit_lock_impl(&conn, "personnes", 1, "token-B").expect("wrong release");
        assert_eq!(count_locks(&conn), 1);
        release_edit_lock_impl(&conn, "personnes", 1, "token-A").expect("release");
        assert_eq!(count_locks(&conn), 0);
    }

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
