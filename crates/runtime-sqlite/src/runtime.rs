use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use sparsion_core::{
    Clock, DecayPolicy, HeuristicScorer, RuntimeError, SalienceScorer, SweepResult,
    TimeDecayPolicy,
};
use sparsion_types::{Event, MemoryQuery, MemoryTier, ScoredMemory};
use uuid::Uuid;

/// Unified Sparsion Runtime backed by SQLite.
///
/// Owns the connection, scorer, decay policy, and clock.
/// This is the single entry point for record → score → sweep → retrieve.
pub struct SqliteRuntime {
    conn: Mutex<Connection>,
    scorer: HeuristicScorer,
    decay: TimeDecayPolicy,
}

impl SqliteRuntime {
    pub fn open(path: &str) -> Result<Self, RuntimeError> {
        let conn = Connection::open(path).map_err(|e| RuntimeError::Storage(e.to_string()))?;
        crate::init_db(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            scorer: HeuristicScorer::default(),
            decay: TimeDecayPolicy::default(),
        })
    }

    pub fn in_memory() -> Result<Self, RuntimeError> {
        let conn =
            Connection::open_in_memory().map_err(|e| RuntimeError::Storage(e.to_string()))?;
        crate::init_db(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            scorer: HeuristicScorer::default(),
            decay: TimeDecayPolicy::default(),
        })
    }

    pub fn with_clock(path: &str, clock: Arc<dyn Clock>) -> Result<Self, RuntimeError> {
        let conn = Connection::open(path).map_err(|e| RuntimeError::Storage(e.to_string()))?;
        crate::init_db(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            scorer: HeuristicScorer::with_clock(168.0, clock.clone()),
            decay: TimeDecayPolicy::with_clock(clock),
        })
    }

    pub fn in_memory_with_clock(clock: Arc<dyn Clock>) -> Result<Self, RuntimeError> {
        let conn =
            Connection::open_in_memory().map_err(|e| RuntimeError::Storage(e.to_string()))?;
        crate::init_db(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            scorer: HeuristicScorer::with_clock(168.0, clock.clone()),
            decay: TimeDecayPolicy::with_clock(clock),
        })
    }

    /// Record an event: append to store, compute initial salience, assign tier.
    pub fn record(&self, event: &Event) -> Result<f64, RuntimeError> {
        let conn = self.conn.lock().unwrap();

        // Append event
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
                serde_json::to_string(&event.importance)
                    .unwrap()
                    .trim_matches('"'),
            ],
        )
        .map_err(|e| RuntimeError::Storage(e.to_string()))?;

        // Count occurrences
        let occurrence_count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE content = ?1 AND source = ?2",
                rusqlite::params![event.content, event.source],
                |row| row.get(0),
            )
            .map_err(|e| RuntimeError::Storage(e.to_string()))?;

        // Score
        let salience = self.scorer.score(event, occurrence_count);
        let tier = self.decay.assign_tier(salience);

        // Insert memory state
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

        Ok(salience)
    }

    /// Query memories ranked by salience.
    pub fn query(&self, query: &MemoryQuery) -> Result<Vec<ScoredMemory>, RuntimeError> {
        let conn = self.conn.lock().unwrap();
        let limit = query.limit.unwrap_or(20);

        let mut sql = String::from(
            "SELECT e.id, e.timestamp, e.source, e.kind, e.content, e.metadata, e.importance,
                    m.salience, m.tier, m.occurrence_count, m.last_accessed
             FROM events e
             JOIN memory_state m ON e.id = m.event_id
             WHERE m.tier != 'forgotten'",
        );

        if let Some(ref source) = query.source {
            sql.push_str(&format!(" AND e.source = '{}'", source));
        }
        if let Some(ref kind) = query.kind {
            let kind_str = serde_json::to_string(kind).unwrap().trim_matches('"').to_string();
            sql.push_str(&format!(" AND e.kind = '{}'", kind_str));
        }
        if let Some(min_sal) = query.min_salience {
            sql.push_str(&format!(" AND m.salience >= {}", min_sal));
        }
        if let Some(ref tier) = query.tier {
            let tier_str = serde_json::to_string(tier).unwrap().trim_matches('"').to_string();
            sql.push_str(&format!(" AND m.tier = '{}'", tier_str));
        }
        if let Some(ref text) = query.text {
            sql.push_str(&format!(
                " AND e.content LIKE '%{}%'",
                text.replace('\'', "''")
            ));
        }

        sql.push_str(&format!(" ORDER BY m.salience DESC LIMIT {}", limit));

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| RuntimeError::Query(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(RawRow {
                    id_str: row.get(0)?,
                    ts_str: row.get(1)?,
                    source: row.get(2)?,
                    kind_str: row.get(3)?,
                    content: row.get(4)?,
                    meta_str: row.get(5)?,
                    imp_str: row.get(6)?,
                    salience: row.get(7)?,
                    tier_str: row.get(8)?,
                    occurrence_count: row.get(9)?,
                    last_accessed_str: row.get(10)?,
                })
            })
            .map_err(|e| RuntimeError::Query(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let r = row.map_err(|e| RuntimeError::Query(e.to_string()))?;
            results.push(r.into_scored_memory()?);
        }

        Ok(results)
    }

    /// Run a full decay sweep: recompute salience for all non-forgotten memories,
    /// update scores in storage, reassign tiers.
    pub fn sweep(&self) -> Result<SweepResult, RuntimeError> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT e.id, e.timestamp, e.source, e.kind, e.content, e.metadata, e.importance,
                        m.salience, m.tier, m.occurrence_count, m.last_accessed
                 FROM events e
                 JOIN memory_state m ON e.id = m.event_id
                 WHERE m.tier != 'forgotten'",
            )
            .map_err(|e| RuntimeError::Storage(e.to_string()))?;

        let rows: Vec<RawRow> = stmt
            .query_map([], |row| {
                Ok(RawRow {
                    id_str: row.get(0)?,
                    ts_str: row.get(1)?,
                    source: row.get(2)?,
                    kind_str: row.get(3)?,
                    content: row.get(4)?,
                    meta_str: row.get(5)?,
                    imp_str: row.get(6)?,
                    salience: row.get(7)?,
                    tier_str: row.get(8)?,
                    occurrence_count: row.get(9)?,
                    last_accessed_str: row.get(10)?,
                })
            })
            .map_err(|e| RuntimeError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| RuntimeError::Storage(e.to_string()))?;

        drop(stmt);

        let mut result = SweepResult {
            total_processed: 0,
            promoted: 0,
            demoted: 0,
            forgotten: 0,
        };

        for r in rows {
            result.total_processed += 1;

            let scored = r.into_scored_memory()?;
            let old_tier = scored.tier;

            // Recompute salience through the full scorer (uses clock for recency)
            let new_salience = self.scorer.score(&scored.event, scored.occurrence_count);
            let new_tier = self.decay.assign_tier(new_salience);

            if new_tier != old_tier {
                match (&old_tier, &new_tier) {
                    (_, MemoryTier::Forgotten) => result.forgotten += 1,
                    (MemoryTier::Warm, MemoryTier::Hot)
                    | (MemoryTier::Cold, MemoryTier::Hot)
                    | (MemoryTier::Cold, MemoryTier::Warm) => {
                        result.promoted += 1;
                    }
                    _ => result.demoted += 1,
                }
            }

            let tier_str = serde_json::to_string(&new_tier)
                .unwrap()
                .trim_matches('"')
                .to_string();
            conn.execute(
                "UPDATE memory_state SET salience = ?1, tier = ?2 WHERE event_id = ?3",
                rusqlite::params![new_salience, tier_str, scored.event.id.to_string()],
            )
            .map_err(|e| RuntimeError::Storage(e.to_string()))?;
        }

        Ok(result)
    }

    /// Get total event count.
    pub fn count(&self) -> Result<u64, RuntimeError> {
        let conn = self.conn.lock().unwrap();
        let count: u64 = conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
            .map_err(|e| RuntimeError::Storage(e.to_string()))?;
        Ok(count)
    }

    /// Get memory state for a specific event by ID.
    pub fn get_memory(&self, id: Uuid) -> Result<ScoredMemory, RuntimeError> {
        let conn = self.conn.lock().unwrap();
        let r = conn
            .query_row(
                "SELECT e.id, e.timestamp, e.source, e.kind, e.content, e.metadata, e.importance,
                        m.salience, m.tier, m.occurrence_count, m.last_accessed
                 FROM events e
                 JOIN memory_state m ON e.id = m.event_id
                 WHERE e.id = ?1",
                rusqlite::params![id.to_string()],
                |row| {
                    Ok(RawRow {
                        id_str: row.get(0)?,
                        ts_str: row.get(1)?,
                        source: row.get(2)?,
                        kind_str: row.get(3)?,
                        content: row.get(4)?,
                        meta_str: row.get(5)?,
                        imp_str: row.get(6)?,
                        salience: row.get(7)?,
                        tier_str: row.get(8)?,
                        occurrence_count: row.get(9)?,
                        last_accessed_str: row.get(10)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => RuntimeError::NotFound(id),
                _ => RuntimeError::Storage(e.to_string()),
            })?;

        r.into_scored_memory()
    }
}

/// Internal helper for row parsing.
struct RawRow {
    id_str: String,
    ts_str: String,
    source: String,
    kind_str: String,
    content: String,
    meta_str: String,
    imp_str: String,
    salience: f64,
    tier_str: String,
    occurrence_count: u32,
    last_accessed_str: String,
}

impl RawRow {
    fn into_scored_memory(self) -> Result<ScoredMemory, RuntimeError> {
        let event = Event {
            id: self
                .id_str
                .parse()
                .map_err(|e: uuid::Error| RuntimeError::Storage(e.to_string()))?,
            timestamp: self
                .ts_str
                .parse()
                .map_err(|e: chrono::ParseError| RuntimeError::Storage(e.to_string()))?,
            source: self.source,
            kind: serde_json::from_str(&format!("\"{}\"", self.kind_str))
                .map_err(|e| RuntimeError::Storage(e.to_string()))?,
            content: self.content,
            metadata: serde_json::from_str(&self.meta_str)
                .map_err(|e| RuntimeError::Storage(e.to_string()))?,
            importance: serde_json::from_str(&format!("\"{}\"", self.imp_str))
                .map_err(|e| RuntimeError::Storage(e.to_string()))?,
        };

        let tier: MemoryTier = serde_json::from_str(&format!("\"{}\"", self.tier_str))
            .map_err(|e| RuntimeError::Storage(e.to_string()))?;

        let last_accessed = self
            .last_accessed_str
            .parse()
            .map_err(|e: chrono::ParseError| RuntimeError::Storage(e.to_string()))?;

        Ok(ScoredMemory {
            event,
            salience: self.salience,
            tier,
            occurrence_count: self.occurrence_count,
            last_accessed,
        })
    }
}
