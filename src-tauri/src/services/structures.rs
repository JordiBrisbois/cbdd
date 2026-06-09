use rusqlite::{params, Connection};

pub fn delete_structure(conn: &Connection, structure_id: i64) -> Result<(), String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM T_Affiliations WHERE Ref_Structure = ?",
        params![structure_id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE T_Reunions SET Ref_Structure = NULL WHERE Ref_Structure = ?",
        params![structure_id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM T_Structures WHERE ID_Structure = ?",
        params![structure_id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_structure_rolls_back_links_when_final_delete_fails() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        conn.execute_batch(
            "
            CREATE TABLE T_Structures (ID_Structure INTEGER PRIMARY KEY);
            CREATE TABLE T_Affiliations (ID_Affiliation INTEGER PRIMARY KEY, Ref_Structure INTEGER);
            CREATE TABLE T_Reunions (ID_Reunion INTEGER PRIMARY KEY, Ref_Structure INTEGER);
            INSERT INTO T_Structures VALUES (1);
            INSERT INTO T_Affiliations VALUES (1, 1);
            INSERT INTO T_Reunions VALUES (1, 1);
            CREATE TRIGGER prevent_structure_delete BEFORE DELETE ON T_Structures
            BEGIN SELECT RAISE(ABORT, 'blocked'); END;
            ",
        )
        .expect("schema");

        assert!(delete_structure(&conn, 1).is_err());
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM T_Affiliations", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row("SELECT Ref_Structure FROM T_Reunions", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}
