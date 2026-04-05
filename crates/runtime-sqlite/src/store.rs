use rusqlite::Connection;
use sparsion_core::{EventStore, RuntimeError, SalienceScorer};
use sparsion_types::{Event, MemoryTier};
use uuid::Uuid;

use std::sync::Mutex;

pub struct SqliteEventStore {
    conn: Mutex<Connection>,
}

impl SqliteEventStore {
    pub fn new(conn: Connection) -> Result<Self, RuntimeError> {
        crate::init_db(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn open(path: &str) -> Result<Self, RuntimeError> {
        let conn =
            Connection::open(path).map_err(|e| RuntimeError::Storage(e.to_string()))?;
        Self::new(conn)
    }

    pub fn in_memory() -> Result<Self, RuntimeError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| RuntimeError::Storage(e.to_string()))?;
        Self::new(conn)
    }

    /// Insert initial memory state for a newly appended event.
    pub fn init_memory_state(
        &self,
        event: &Event,
        scorer: &dyn SalienceScorer,
    ) -> Result<(), RuntimeError> {
        let conn = self.conn.lock().unwrap();
        let occurrence_count = self.count_occurrences_inner(&conn, event)?;
        let salience = scorer.score(event, occurrence_count);
        let tier = if salience >= 2.0 {
            MemoryTier::Hot
        } else if salience >= 0.5 {
            MemoryTier::Warm
        } else {
            MemoryTier::Cold
        };

        conn.execute(
            "INSERT OR REPLACE INTO memory_state (event_id, salience, tier, occurrence_count, last_accessed)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                event.id.to_string(),
                salience,
                serde_json::to_string(&tier).unwrap().trim_matches('"'),
                occurrence_count,
                event.timestamp.to_rfc3339(),
            ],
        )
        .map_err(|e| RuntimeError::Storage(e.to_string()))?;

        Ok(())
    }

    fn count_occurrences_inner(
        &self,
        conn: &Connection,
        event: &Event,
    ) -> Result<u32, RuntimeError> {
        let count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE content = ?1 AND source = ?2",
                rusqlite::params![event.content, event.source],
                |row| row.get(0),
            )
            .map_err(|e| RuntimeError::Storage(e.to_string()))?;
        Ok(count)
    }
}

impl EventStore for SqliteEventStore {
    fn append(&self, event: &Event) -> Result<(), RuntimeError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO events (id, timestamp, source, kind, content, metadata, importance)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                event.id.to_string(),
                event.timestamp.to_rfc3339(),
                event.source,
                serde_json::to_string(&event.kind).unwrap().trim_matches('"'),
                event.content,
                serde_json::to_string(&event.metadata).unwrap(),
                serde_json::to_string(&event.importance).unwrap().trim_matches('"'),
            ],
        )
        .map_err(|e| RuntimeError::Storage(e.to_string()))?;

        Ok(())
    }

    fn get(&self, id: Uuid) -> Result<Event, RuntimeError> {
        let conn = self.conn.lock().unwrap();
        let event = conn
            .query_row(
                "SELECT id, timestamp, source, kind, content, metadata, importance
                 FROM events WHERE id = ?1",
                rusqlite::params![id.to_string()],
                |row| {
                    let id_str: String = row.get(0)?;
                    let ts_str: String = row.get(1)?;
                    let source: String = row.get(2)?;
                    let kind_str: String = row.get(3)?;
                    let content: String = row.get(4)?;
                    let meta_str: String = row.get(5)?;
                    let imp_str: String = row.get(6)?;

                    Ok((id_str, ts_str, source, kind_str, content, meta_str, imp_str))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => RuntimeError::NotFound(id),
                _ => RuntimeError::Storage(e.to_string()),
            })?;

        let (id_str, ts_str, source, kind_str, content, meta_str, imp_str) = event;

        Ok(Event {
            id: id_str.parse().map_err(|e: uuid::Error| RuntimeError::Storage(e.to_string()))?,
            timestamp: ts_str
                .parse()
                .map_err(|e: chrono::ParseError| RuntimeError::Storage(e.to_string()))?,
            source,
            kind: serde_json::from_str(&format!("\"{}\"", kind_str))
                .map_err(|e| RuntimeError::Storage(e.to_string()))?,
            content,
            metadata: serde_json::from_str(&meta_str)
                .map_err(|e| RuntimeError::Storage(e.to_string()))?,
            importance: serde_json::from_str(&format!("\"{}\"", imp_str))
                .map_err(|e| RuntimeError::Storage(e.to_string()))?,
        })
    }

    fn count(&self) -> Result<u64, RuntimeError> {
        let conn = self.conn.lock().unwrap();
        let count: u64 = conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
            .map_err(|e| RuntimeError::Storage(e.to_string()))?;
        Ok(count)
    }

    fn record_occurrence(&self, event: &Event) -> Result<u32, RuntimeError> {
        let conn = self.conn.lock().unwrap();
        self.count_occurrences_inner(&conn, event)
    }
}
