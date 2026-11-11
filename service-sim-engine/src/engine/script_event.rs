//! An event of a script, which is known before the run.

/// An event of a script, which is known before the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptEvent {
    Arrive {
        at: u64,
        key: String,
        priority: u8,
        durations: Vec<u64>,
        deadline: u64,
    },
    Cancel {
        at: u64,
        key: String,
    },
    Fail {
        at: u64,
        server: u64,
    },
    Recover {
        at: u64,
        server: u64,
    },
}
