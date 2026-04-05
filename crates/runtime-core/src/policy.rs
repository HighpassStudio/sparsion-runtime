/// Named policy presets for different agent domains.
///
/// Each preset configures half-life, tier thresholds, and event-type weights
/// tuned for a specific use case.
#[derive(Debug, Clone)]
pub struct RuntimePolicy {
    pub half_life_hours: f64,
    pub hot_threshold: f64,
    pub warm_threshold: f64,
    pub forget_threshold: f64,
    pub correction_weight: f64,
    pub decision_weight: f64,
    pub error_weight: f64,
    pub action_weight: f64,
    pub observation_weight: f64,
}

impl Default for RuntimePolicy {
    fn default() -> Self {
        Self::balanced()
    }
}

impl RuntimePolicy {
    /// Balanced defaults — 1 week half-life, standard weights.
    pub fn balanced() -> Self {
        Self {
            half_life_hours: 168.0,
            hot_threshold: 2.0,
            warm_threshold: 0.5,
            forget_threshold: 0.05,
            correction_weight: 3.0,
            decision_weight: 2.0,
            error_weight: 1.5,
            action_weight: 1.0,
            observation_weight: 0.7,
        }
    }

    /// Coding agent — shorter memory, high weight on corrections and errors.
    /// Code changes fast; old decisions go stale quickly.
    pub fn coding() -> Self {
        Self {
            half_life_hours: 72.0, // 3 days
            hot_threshold: 2.0,
            warm_threshold: 0.5,
            forget_threshold: 0.1, // forget faster
            correction_weight: 4.0,
            decision_weight: 2.0,
            error_weight: 2.5, // errors matter more in code
            action_weight: 1.0,
            observation_weight: 0.5,
        }
    }

    /// Knowledge base — long memory, strong reinforcement.
    /// Facts persist; only explicit corrections override.
    pub fn knowledge() -> Self {
        Self {
            half_life_hours: 720.0, // 30 days
            hot_threshold: 1.5,
            warm_threshold: 0.3,
            forget_threshold: 0.02, // very slow to forget
            correction_weight: 4.0,
            decision_weight: 2.5,
            error_weight: 1.0,
            action_weight: 0.5,
            observation_weight: 1.0, // observations are facts here
        }
    }

    /// Personal assistant — medium memory, balanced with user action emphasis.
    pub fn assistant() -> Self {
        Self {
            half_life_hours: 336.0, // 2 weeks
            hot_threshold: 2.0,
            warm_threshold: 0.5,
            forget_threshold: 0.05,
            correction_weight: 3.0,
            decision_weight: 2.0,
            error_weight: 1.0,
            action_weight: 1.5, // user actions matter more
            observation_weight: 0.7,
        }
    }

    /// Parse a policy name string into a preset.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "balanced" | "default" => Some(Self::balanced()),
            "coding" | "code" => Some(Self::coding()),
            "knowledge" | "kb" => Some(Self::knowledge()),
            "assistant" | "personal" => Some(Self::assistant()),
            _ => None,
        }
    }
}
