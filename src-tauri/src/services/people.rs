use rusqlite::{params, Connection};

use crate::commands::{ANONYMIZED_LABEL, ANONYMIZED_STATUS};

pub fn delete_person(conn: &Connection, person_id: i64) -> Result<(), String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM T_Affiliations WHERE Ref_Personne = ?",
        params![person_id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE T_Presences SET Ref_Personne = NULL WHERE Ref_Personne = ?",
        params![person_id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM T_Personnes WHERE ID_Personne = ?",
        params![person_id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

pub fn anonymize_person(conn: &Connection, person_id: i64) -> Result<(), String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE T_Personnes SET
            Civilite = NULL,
            Nom = ?,
            Prenom = NULL,
            Email_Prive = NULL,
            Telephone_Prive = NULL,
            Adresse_Privee = NULL,
            Code_Postal_Prive = NULL,
            Commune_Privee = NULL,
            Pays = NULL,
            Consentement_RGPD = 0,
            Date_Consentement = NULL,
            Statut_Compte = ?,
            Notes_Commentaires = NULL,
            Date_Creation = NULL,
            Updated_At = strftime('%Y-%m-%d %H:%M:%f', 'now')
         WHERE ID_Personne = ?",
        params![ANONYMIZED_LABEL, ANONYMIZED_STATUS, person_id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM T_Affiliations WHERE Ref_Personne = ?",
        params![person_id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE T_Presences
         SET Notes_Commentaires = NULL, Souhaite_Rester_En_BDD = 0
         WHERE Ref_Personne = ?",
        params![person_id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open sqlite");
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE T_Personnes (
                ID_Personne INTEGER PRIMARY KEY,
                Civilite TEXT, Nom TEXT, Prenom TEXT, Email_Prive TEXT,
                Telephone_Prive TEXT, Adresse_Privee TEXT, Code_Postal_Prive TEXT,
                Commune_Privee TEXT, Pays TEXT, Consentement_RGPD INTEGER,
                Date_Consentement TEXT, Statut_Compte TEXT, Notes_Commentaires TEXT,
                Date_Creation TEXT, Updated_At TEXT
            );
            CREATE TABLE T_Affiliations (ID_Affiliation INTEGER PRIMARY KEY, Ref_Personne INTEGER);
            CREATE TABLE T_Presences (
                ID_Presence INTEGER PRIMARY KEY, Ref_Personne INTEGER,
                Notes_Commentaires TEXT, Souhaite_Rester_En_BDD INTEGER
            );
            INSERT INTO T_Personnes VALUES (
                1, 'Mme', 'Dupont', 'Alice', 'alice@example.org', '1', 'Rue', '4800',
                'Verviers', 'BE', 1, '2026-01-01', 'Actif', 'note', '2026-01-01', 'v1'
            );
            INSERT INTO T_Affiliations VALUES (1, 1);
            INSERT INTO T_Presences VALUES (1, 1, 'presence note', 1);
            ",
        )
        .expect("schema");
        conn
    }

    #[test]
    fn delete_person_rolls_back_all_changes_when_final_delete_fails() {
        let conn = test_connection();
        conn.execute_batch(
            "CREATE TRIGGER prevent_person_delete BEFORE DELETE ON T_Personnes
             BEGIN SELECT RAISE(ABORT, 'blocked'); END;",
        )
        .expect("trigger");

        assert!(delete_person(&conn, 1).is_err());
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM T_Affiliations", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row("SELECT Ref_Personne FROM T_Presences", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn anonymize_person_rolls_back_when_affiliation_cleanup_fails() {
        let conn = test_connection();
        conn.execute_batch(
            "CREATE TRIGGER prevent_affiliation_delete BEFORE DELETE ON T_Affiliations
             BEGIN SELECT RAISE(ABORT, 'blocked'); END;",
        )
        .expect("trigger");

        assert!(anonymize_person(&conn, 1).is_err());
        let person: (String, String, i64) = conn
            .query_row(
                "SELECT Nom, Email_Prive, Consentement_RGPD FROM T_Personnes WHERE ID_Personne = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(person, ("Dupont".into(), "alice@example.org".into(), 1));
    }
}
