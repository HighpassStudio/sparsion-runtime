use rusqlite::Connection;
use sparsion_core::{DecayPolicy, MemoryRetriever, RuntimeError, SweepResult};
use sparsion_types::{Event, MemoryQuery, MemoryTier, ScoredMemory};

use std::sync::Mutex;

pub struct SqliteRetriever {
    conn: Mutex<Connection>,
    decay_policy: Box<dyn DecayPolicy + Send>,
}

impl SqliteRetriever {
    pub fn new(conn: Connection, decay_policy: Box<dyn DecayPolicy + Send>) -> Result<Self, RuntimeError> {
        crate::init_db(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            decay_policy,
        })
    }
}

impl MemoryRetriever for SqliteRetriever {
    fn retrieve(&self, query: &MemoryQuery) -> Result<Vec<ScoredMemory>, RuntimeError> {
        let conn = self.conn.lock().unwrap();
        let limit = query.limit.unwrap_or(20);

        let mut sql = String::from(
            "SELECT e.id, e.timestamp, e.source, e.kind, e.content, e.metadata, e.importance,
                    m.salience, m.tier, m.occurrence_count, m.last_accessed
             FROM events e
             JOIN memory_state m ON e.id = m.event_id
             WHERE m.tier != 'forgotten'"
        );

        if let Some(ref source) = query.source {
            sql.push_str(&format!(" AND e.source = '{}'", source));
        }
        if let Some(ref kind) = query.kind {
            let kind_str = serde_json::to_string(kind).unwrap();
            sql.push_str(&format!(" AND e.kind = {}", kind_str));
        }
        if let Some(min_sal) = query.min_salience {
            sql.push_str(&format!(" AND m.salience >= {}", min_sal));
        }
        if let Some(ref tier) = query.tier {
            let tier_str = serde_json::to_string(tier).unwrap().replace('"', "");
            sql.push_str(&format!(" AND m.tier = '{}'", tier_str));
        }
        if let Some(ref text) = query.text {
            sql.push_str(&format!(" AND e.content LIKE '%{}%'", text.replace('\'', "''")));
        }

        sql.push_str(&format!(" ORDER BY m.salience DESC LIMIT {}", limit));

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| RuntimeError::Query(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let ts_str: String = row.get(1)?;
                let source: String = row.get(2)?;
                let kind_str: String = row.get(3)?;
                let content: String = row.get(4)?;
                let meta_str: String = row.get(5)?;
                let imp_str: String = row.get(6)?;
                let salience: f64 = row.get(7)?;
                let tier_str: String = row.get(8)?;
                let occ: u32 = row.get(9)?;
                let la_str: String = row.get(10)?;

                Ok((
                    id_str, ts_str, source, kind_str, content, meta_str, imp_str,
                    salience, tier_str, occ, la_str,
                ))
            })
            .map_err(|e| RuntimeError::Query(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let (id_str, ts_str, source, kind_str, content, meta_str, imp_str, salience, tier_str, occ, la_str) =
                row.map_err(|e| RuntimeError::Query(e.to_string()))?;

            let event = Event {
                id: id_str.parse().map_err(|e: uuid::Error| RuntimeError::Query(e.to_string()))?,
                timestamp: ts_str.parse().map_err(|e: chrono::ParseError| RuntimeError::Query(e.to_string()))?,
                source,
                kind: serde_json::from_str(&format!("\"{}\"", kind_str))
                    .map_err(|e| RuntimeError::Query(e.to_string()))?,
                content,
                metadata: serde_json::from_str(&meta_str)
                    .map_err(|e| RuntimeError::Query(e.to_string()))?,
                importance: serde_json::from_str(&format!("\"{}\"", imp_str))
                    .map_err(|e| RuntimeError::Query(e.to_string()))?,
                overrides: None,
            };

            let tier: MemoryTier = serde_json::from_str(&format!("\"{}\"", tier_str))
                .map_err(|e| RuntimeError::Query(e.to_string()))?;

            let last_accessed = la_str
                .parse()
                .map_err(|e: chrono::ParseError| RuntimeError::Query(e.to_string()))?;

            results.push(ScoredMemory {
                event,
                salience,
                tier,
                occurrence_count: occ,
                last_accessed,
                is_overridden: false,
            });
        }

        Ok(results)
    }

    fn sweep(&self) -> Result<SweepResult, RuntimeError> {
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

        let rows: Vec<_> = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let salience: f64 = row.get(7)?;
                let tier_str: String = row.get(8)?;
                let occ: u32 = row.get(9)?;
                let la_str: String = row.get(10)?;
                Ok((id_str, salience, tier_str, occ, la_str))
            })
            .map_err(|e| RuntimeError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| RuntimeError::Storage(e.to_string()))?;

        let mut result = SweepResult {
            total_processed: 0,
            promoted: 0,
            demoted: 0,
            forgotten: 0,
        };

        for (id_str, salience, tier_str, occ, la_str) in rows {
            result.total_processed += 1;

            let last_accessed: chrono::DateTime<chrono::Utc> = la_str
                .parse()
                .map_err(|e: chrono::ParseError| RuntimeError::Storage(e.to_string()))?;

            let memory = ScoredMemory {
                event: Event::new("", sparsion_types::EventKind::Observation, ""), // placeholder
                salience,
                tier: serde_json::from_str(&format!("\"{}\"", tier_str))
                    .unwrap_or(MemoryTier::Cold),
                occurrence_count: occ,
                last_accessed,
                is_overridden: false,
            };

            let new_salience = self.decay_policy.decay(&memory);
            let new_tier = self.decay_policy.assign_tier(new_salience);
            let old_tier = memory.tier;

            if new_tier != old_tier {
                match (&old_tier, &new_tier) {
                    (_, MemoryTier::Forgotten) => result.forgotten += 1,
                    (MemoryTier::Warm, MemoryTier::Hot) | (MemoryTier::Cold, MemoryTier::Hot) | (MemoryTier::Cold, MemoryTier::Warm) => {
                        result.promoted += 1;
                    }
                    _ => result.demoted += 1,
                }
            }

            let tier_str = serde_json::to_string(&new_tier).unwrap().replace('"', "");
            conn.execute(
                "UPDATE memory_state SET salience = ?1, tier = ?2 WHERE event_id = ?3",
                rusqlite::params![new_salience, tier_str, id_str],
            )
            .map_err(|e| RuntimeError::Storage(e.to_string()))?;
        }

        Ok(result)
    }
}
