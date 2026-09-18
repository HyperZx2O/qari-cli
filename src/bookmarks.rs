use crate::data::get_data_dir;
use rusqlite::{params, Connection};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Bookmark {
    pub id: i64,
    pub surah_id: u8,
    pub ayah_id: u16,
    pub tag: String,
    pub note: String,
    pub timestamp: i64,
}

const CREATE_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS bookmarks (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        surah_id INTEGER NOT NULL,
        ayah_id INTEGER NOT NULL,
        tag TEXT DEFAULT '',
        note TEXT DEFAULT '',
        timestamp INTEGER NOT NULL
    );
";

pub fn init_db() -> Result<Connection, rusqlite::Error> {
    let path = get_data_dir().join("bookmarks.db");
    let conn = Connection::open(path)?;
    conn.execute(CREATE_TABLE, [])?;
    Ok(conn)
}

pub fn add_bookmark(
    conn: &Connection,
    surah_id: u8,
    ayah_id: u16,
    tag: &str,
    note: &str,
) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO bookmarks (surah_id, ayah_id, tag, note, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![surah_id, ayah_id, tag, note, chrono::Utc::now().timestamp()],
    )?;
    conn.query_row(
        "SELECT id FROM bookmarks WHERE surah_id = ?1 AND ayah_id = ?2",
        params![surah_id, ayah_id],
        |row| row.get(0),
    )
}

pub fn delete_bookmark(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM bookmarks WHERE id = ?1", [id])?;
    Ok(())
}

#[allow(dead_code)]
pub fn get_bookmarks(conn: &Connection) -> Result<Vec<Bookmark>, rusqlite::Error> {
    let mut statement = conn.prepare(
        "SELECT id, surah_id, ayah_id, tag, note, timestamp FROM bookmarks ORDER BY timestamp DESC",
    )?;
    let bookmarks = statement
        .query_map([], |row| {
            Ok(Bookmark {
                id: row.get(0)?,
                surah_id: row.get(1)?,
                ayah_id: row.get(2)?,
                tag: row.get(3)?,
                note: row.get(4)?,
                timestamp: row.get(5)?,
            })
        })?
        .collect();
    bookmarks
}

#[allow(dead_code)]
pub fn is_bookmarked(conn: &Connection, surah_id: u8, ayah_id: u16) -> bool {
    bookmark_id(conn, surah_id, ayah_id).is_some()
}

pub fn bookmark_id(conn: &Connection, surah_id: u8, ayah_id: u16) -> Option<i64> {
    conn.query_row(
        "SELECT id FROM bookmarks WHERE surah_id = ?1 AND ayah_id = ?2",
        params![surah_id, ayah_id],
        |row| row.get(0),
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bookmark_crud_round_trip() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(CREATE_TABLE, []).unwrap();
        let id = add_bookmark(&conn, 2, 255, "favorite", "").unwrap();
        assert!(is_bookmarked(&conn, 2, 255));
        assert_eq!(get_bookmarks(&conn).unwrap()[0].id, id);
        delete_bookmark(&conn, id).unwrap();
        assert!(!is_bookmarked(&conn, 2, 255));
    }
}
