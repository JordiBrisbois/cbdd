use super::seeding;
use rusqlite::Connection;

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

    // Seed des données système — ces appels étaient présents dans l'ancien auth.rs
    seeding::seed_permissions(conn)?;
    seeding::seed_system_roles(conn)?;
    seeding::seed_public_permissions(conn)?;
    seeding::seed_statistics_permissions(conn)?;
    seeding::seed_default_settings(conn)?;
    seeding::seed_admin_user(conn)?;
    ensure_concurrency_schema(conn)?;

    Ok(())
}

pub fn ensure_concurrency_schema(conn: &Connection) -> Result<(), String> {
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

pub fn ensure_column(
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
