use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::types::PyDict;

use sparsion_sqlite::SqliteRuntime;
use sparsion_types::{Event, EventKind, Importance, MemoryQuery, MemoryTier};

/// The main Sparsion Runtime instance.
#[pyclass]
struct Runtime {
    inner: SqliteRuntime,
}

#[pymethods]
impl Runtime {
    #[new]
    #[pyo3(signature = (path))]
    fn new(path: &str) -> PyResult<Self> {
        let inner = SqliteRuntime::open(path)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Record an event into the runtime. Returns the event ID.
    #[pyo3(signature = (source, kind, content, importance="normal", overrides=None))]
    fn record(&self, source: &str, kind: &str, content: &str, importance: &str, overrides: Option<String>) -> PyResult<String> {
        let event_kind = parse_kind(kind)?;
        let imp = parse_importance(importance)?;

        let mut event = Event::new(source, event_kind, content).with_importance(imp);
        if let Some(ref target) = overrides {
            let target_id: uuid::Uuid = target.parse()
                .map_err(|e: uuid::Error| PyRuntimeError::new_err(format!("invalid overrides UUID: {}", e)))?;
            event = event.with_overrides(target_id);
        }
        let id = event.id.to_string();

        self.inner
            .record(&event)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(id)
    }

    /// Query memories by optional filters. Returns list of dicts.
    #[pyo3(signature = (text=None, source=None, min_salience=None, tier=None, limit=20))]
    fn query(
        &self,
        py: Python<'_>,
        text: Option<String>,
        source: Option<String>,
        min_salience: Option<f64>,
        tier: Option<String>,
        limit: usize,
    ) -> PyResult<Vec<PyObject>> {
        let mut mq = MemoryQuery::new().limit(limit);
        if let Some(t) = text {
            mq = mq.text(t);
        }
        if let Some(s) = source {
            mq.source = Some(s);
        }
        if let Some(ms) = min_salience {
            mq = mq.min_salience(ms);
        }
        if let Some(t) = tier {
            mq = mq.tier(parse_tier(&t)?);
        }

        let memories = self.inner
            .query(&mq)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let results: Vec<PyObject> = memories
            .iter()
            .map(|m| {
                let dict = PyDict::new_bound(py);
                dict.set_item("id", m.event.id.to_string()).unwrap();
                dict.set_item("content", &m.event.content).unwrap();
                dict.set_item("source", &m.event.source).unwrap();
                dict.set_item("salience", m.salience).unwrap();
                dict.set_item("tier", format!("{:?}", m.tier)).unwrap();
                dict.set_item("occurrence_count", m.occurrence_count).unwrap();
                dict.set_item("timestamp", m.event.timestamp.to_rfc3339()).unwrap();
                dict.set_item("is_overridden", m.is_overridden).unwrap();
                dict.into()
            })
            .collect();

        Ok(results)
    }

    /// Run a decay sweep across all memories.
    fn sweep(&self, py: Python<'_>) -> PyResult<PyObject> {
        let result = self.inner
            .sweep()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let dict = PyDict::new_bound(py);
        dict.set_item("total_processed", result.total_processed)?;
        dict.set_item("promoted", result.promoted)?;
        dict.set_item("demoted", result.demoted)?;
        dict.set_item("forgotten", result.forgotten)?;
        Ok(dict.into())
    }

    /// Get total event count.
    fn count(&self) -> PyResult<u64> {
        self.inner
            .count()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Inspect memory state: counts per tier.
    fn inspect(&self, py: Python<'_>) -> PyResult<PyObject> {
        let result = self.inner
            .inspect()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let dict = PyDict::new_bound(py);
        dict.set_item("total_events", result.total_events)?;
        dict.set_item("hot", result.hot)?;
        dict.set_item("warm", result.warm)?;
        dict.set_item("cold", result.cold)?;
        dict.set_item("forgotten", result.forgotten)?;
        Ok(dict.into())
    }
}

fn parse_kind(s: &str) -> PyResult<EventKind> {
    match s {
        "user_action" => Ok(EventKind::UserAction),
        "observation" => Ok(EventKind::Observation),
        "decision" => Ok(EventKind::Decision),
        "error" => Ok(EventKind::Error),
        "correction" => Ok(EventKind::Correction),
        _ => Err(PyRuntimeError::new_err(format!(
            "unknown event kind: '{}'. Expected: user_action, observation, decision, error, correction",
            s
        ))),
    }
}

fn parse_importance(s: &str) -> PyResult<Importance> {
    match s {
        "low" => Ok(Importance::Low),
        "normal" => Ok(Importance::Normal),
        "high" => Ok(Importance::High),
        "critical" => Ok(Importance::Critical),
        _ => Err(PyRuntimeError::new_err(format!(
            "unknown importance: '{}'. Expected: low, normal, high, critical",
            s
        ))),
    }
}

fn parse_tier(s: &str) -> PyResult<MemoryTier> {
    match s {
        "hot" => Ok(MemoryTier::Hot),
        "warm" => Ok(MemoryTier::Warm),
        "cold" => Ok(MemoryTier::Cold),
        "forgotten" => Ok(MemoryTier::Forgotten),
        _ => Err(PyRuntimeError::new_err(format!(
            "unknown tier: '{}'. Expected: hot, warm, cold, forgotten",
            s
        ))),
    }
}

#[pymodule]
fn sparsion_runtime(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Runtime>()?;
    Ok(())
}
