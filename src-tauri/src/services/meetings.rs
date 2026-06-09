use rusqlite::{params, Connection};

pub fn delete_meeting(conn: &Connection, meeting_id: i64) -> Result<(), String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM T_Presences WHERE Ref_Reunion = ?",
        params![meeting_id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM T_Reunions WHERE ID_Reunion = ?",
        params![meeting_id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_meeting_rolls_back_presences_when_final_delete_fails() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        conn.execute_batch(
            "
            CREATE TABLE T_Reunions (ID_Reunion INTEGER PRIMARY KEY);
            CREATE TABLE T_Presences (ID_Presence INTEGER PRIMARY KEY, Ref_Reunion INTEGER);
            INSERT INTO T_Reunions VALUES (1);
            INSERT INTO T_Presences VALUES (1, 1);
            CREATE TRIGGER prevent_meeting_delete BEFORE DELETE ON T_Reunions
            BEGIN SELECT RAISE(ABORT, 'blocked'); END;
            ",
        )
        .expect("schema");

        assert!(delete_meeting(&conn, 1).is_err());
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM T_Presences", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}
