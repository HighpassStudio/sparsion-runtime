mod store;
mod retriever;
mod runtime;

pub use store::SqliteEventStore;
pub use retriever::SqliteRetriever;
pub use runtime::SqliteRuntime;

use rusqlite::Connection;
use sparsion_core::RuntimeError;

/// Initialize the SQLite schema.
pub fn init_db(conn: &Connection) -> Result<(), RuntimeError> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            source TEXT NOT NULL,
            kind TEXT NOT NULL,
            content TEXT NOT NULL,
            metadata TEXT NOT NULL DEFAULT '{}',
            importance TEXT NOT NULL DEFAULT 'normal'
        );

        CREATE TABLE IF NOT EXISTS memory_state (
            event_id TEXT PRIMARY KEY REFERENCES events(id),
            salience REAL NOT NULL,
            tier TEXT NOT NULL DEFAULT 'hot',
            occurrence_count INTEGER NOT NULL DEFAULT 1,
            last_accessed TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
        CREATE INDEX IF NOT EXISTS idx_events_source ON events(source);
        CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind);
        CREATE INDEX IF NOT EXISTS idx_memory_salience ON memory_state(salience);
        CREATE INDEX IF NOT EXISTS idx_memory_tier ON memory_state(tier);

        CREATE TABLE IF NOT EXISTS overrides (
            source_id TEXT NOT NULL REFERENCES events(id),
            target_id TEXT NOT NULL REFERENCES events(id),
            PRIMARY KEY (source_id, target_id)
        );

        CREATE INDEX IF NOT EXISTS idx_overrides_target ON overrides(target_id);

        CREATE TABLE IF NOT EXISTS snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            hot INTEGER NOT NULL,
            warm INTEGER NOT NULL,
            cold INTEGER NOT NULL,
            forgotten INTEGER NOT NULL
        );
        ",
    )
    .map_err(|e| RuntimeError::Storage(e.to_string()))?;

    Ok(())
}
