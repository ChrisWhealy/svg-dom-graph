//! A pure function from the engine's state to fixed slots: what is written where, and nothing about how.
//!
//! Every slot has a key that never changes, and the list of slots has the same keys in the same order for a given number of servers.
//! A slot that has nothing to say is blank and hidden from assistive technology, and is never removed. So a draw is a matter of
//! writing the fields that differ, and a full redraw is a matter of writing all of them, and the two reach the same page.
//!
//! Nothing here reads the DOM. [`Projection::canonical`] is the text that the page is compared with, and its hash is the design's
//! `dom_projection_sha`.

use service_sim_engine::{Engine, Model, ServerView, hash::sha256_hex};

#[cfg(test)]
mod unit_tests;

/// The waiting window shows this many jobs, from the head of the queue, and says how many more there are.
pub const QUEUE_SLOTS: usize = 12;

/// The data node has this many cells, and the first is the slice ordinal, so that every displayed slice changes it.
pub const DATA_CELLS: usize = 8;

/// The counters that the overlay lists, with the words that name them. The values are in [`counter_values`].
pub const COUNTER_NAMES: [(&str, &str); 10] = [
    ("arrived", "Arrived"),
    ("admitted", "Admitted"),
    ("rejected", "Rejected"),
    ("completed", "Completed"),
    ("timed-out", "Timed out"),
    ("cancelled", "Cancelled"),
    ("failed", "Failed"),
    ("retrying", "Waiting to retry"),
    ("stale", "Stale pops"),
    ("no-op", "No-op pops"),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a reader has chosen to look at. It is not part of the model, and changing it never touches the engine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct View {
    /// The job that the detail line describes, and that is highlighted wherever it is.
    pub selected: Option<u64>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One element of the overlay that has content: the text it shows, its class, its accessible name and whether it is hidden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub key: String,
    pub text: String,
    pub class: String,
    pub label: String,
    pub hidden: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The four things that a slot can say, which is what a delta writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Text,
    Class,
    Label,
    Hidden,
}

/// One write: the field of a slot, with the value that the new projection holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Write {
    pub slot: usize,
    pub field: Field,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything that is drawn for one snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    pub slots: Vec<Slot>,
    /// The values of the data node, as the engine holds them: ticks and counts are 64 bits wide, and none is narrowed. They change
    /// with every slice, since the first is the slice ordinal.
    pub data: [u64; DATA_CELLS],
}

impl Projection {
    /// The keys of the slots, which depend only on the number of servers.
    #[must_use]
    pub fn keys(servers: usize) -> Vec<String> {
        let mut keys = vec!["clock".to_owned(), "queue".to_owned()];
        keys.extend((0..servers).map(|i| format!("S{i}")));
        keys.extend((0..QUEUE_SLOTS).map(|i| format!("Q{i}")));
        keys.push("more".to_owned());
        keys.extend(COUNTER_NAMES.iter().map(|(key, _)| (*key).to_owned()));
        keys.push("selected".to_owned());
        keys
    }

    /// Whether `other` has the same slots in the same order, which is what a delta needs.
    #[must_use]
    pub fn same_shape(&self, other: &Self) -> bool {
        self.slots.len() == other.slots.len() && self.slots.iter().zip(&other.slots).all(|(a, b)| a.key == b.key)
    }

    /// The text that the page is compared with: one line for each slot, and one for the data. The fields are tab separated, with a
    /// tab or a newline in a field written out, so that no two projections have the same text.
    #[must_use]
    pub fn canonical(&self) -> String {
        let mut text = String::new();
        for slot in &self.slots {
            text.push_str(&canonical_line(&slot.key, &slot.text, &slot.class, &slot.label, slot.hidden));
        }
        text.push_str("data");
        for v in self.data {
            text.push('\t');
            text.push_str(&v.to_string());
        }
        text.push('\n');
        text
    }

    /// The design's `dom_projection_sha`, when it is taken of the page's own content, and the hash of what should be there when it
    /// is taken of the projection.
    #[must_use]
    pub fn sha256(&self) -> String {
        sha256_hex(self.canonical().as_bytes())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One line of the canonical text, which the page's read-back builds in the same way.
#[must_use]
pub fn canonical_line(key: &str, text: &str, class: &str, label: &str, hidden: bool) -> String {
    let escape = |s: &str| s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n");
    format!(
        "{}\t{}\t{}\t{}\t{}\n",
        escape(key),
        escape(text),
        escape(class),
        escape(label),
        u8::from(hidden)
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The writes that turn `old` into `new`. With no `old` (nothing has been drawn, or what was drawn cannot be trusted) it is every
/// field of every slot, which is a full redraw.
///
/// `new` has to have the shape of `old`, and the caller checks that with [`Projection::same_shape`].
#[must_use]
pub fn writes(old: Option<&Projection>, new: &Projection) -> Vec<Write> {
    let mut out = Vec::new();
    for (slot, now) in new.slots.iter().enumerate() {
        let before = old.and_then(|o| o.slots.get(slot));
        let differs = |same: fn(&Slot, &Slot) -> bool| before.is_none_or(|b| !same(b, now));
        if differs(|a, b| a.text == b.text) {
            out.push(Write { slot, field: Field::Text });
        }
        if differs(|a, b| a.class == b.class) {
            out.push(Write { slot, field: Field::Class });
        }
        if differs(|a, b| a.label == b.label) {
            out.push(Write { slot, field: Field::Label });
        }
        if differs(|a, b| a.hidden == b.hidden) {
            out.push(Write { slot, field: Field::Hidden });
        }
    }
    out
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The values of the ten counters, in the order of [`COUNTER_NAMES`].
#[must_use]
pub fn counter_values(model: &Model) -> [u64; 10] {
    let c = &model.counters;
    [
        c.arrivals,
        c.admitted,
        c.rejected,
        c.completed,
        c.timed_out,
        c.cancelled,
        c.failed_attempts + c.failed_deferrals,
        model.retry_wait as u64,
        c.stale,
        c.noops,
    ]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn slot(key: &str, text: String, class: &str, label: String, hidden: bool) -> Slot {
    Slot {
        key: key.to_owned(),
        text,
        class: class.to_owned(),
        label,
        hidden,
    }
}

fn priority_word(priority: u8) -> &'static str {
    if priority == 0 { "low" } else { "high" }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The model that [`project`] needs for a reader who has chosen `view`: the window of the queue, the running jobs and the selected
/// one, and nothing of the jobs that nobody can see. Its cost does not grow with the length of the run.
#[must_use]
pub fn extract(engine: &Engine, view: &View) -> Model {
    engine.model(QUEUE_SLOTS, view.selected)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the model looks like to a reader who has chosen `view`.
///
/// The model has to be the one that [`extract`] made for the same view. A model extracted for another selection may not hold the
/// job that this view selects, and that is a caller's defect, which a debug build reports.
#[must_use]
pub fn project(model: &Model, view: &View) -> Projection {
    debug_assert_eq!(
        model.selected, view.selected,
        "the model was extracted for another selection than the view's"
    );
    let selected = view.selected.and_then(|id| model.job(id));
    let chosen = |id: u64| selected.is_some_and(|j| j.id == id);
    let mut slots = Vec::with_capacity(Projection::keys(model.servers.len()).len());

    slots.push(slot(
        "clock",
        format!("tick {} · slice {}", model.tick, model.slice),
        "ss-clock",
        format!("Simulated tick {}, snapshot {}", model.tick, model.slice),
        false,
    ));
    slots.push(slot(
        "queue",
        format!("{}/{}", model.waiting_total, model.queue_capacity),
        "ss-queue-count",
        format!("The waiting queue holds {} of {}", model.waiting_total, model.queue_capacity),
        false,
    ));

    for (i, server) in model.servers.iter().enumerate() {
        let key = format!("S{i}");
        slots.push(match *server {
            ServerView::Idle => slot(&key, "idle".into(), "ss-server ss-idle", format!("Server {key}: idle"), false),
            ServerView::Down => slot(&key, "down".into(), "ss-server ss-down", format!("Server {key}: down"), false),
            ServerView::Busy { job, due } => {
                let (name, attempt) = model.job(job).map_or((format!("#{job}"), 0), |j| (j.key.clone(), j.attempt));
                let class = if chosen(job) { "ss-server ss-busy ss-selected" } else { "ss-server ss-busy" };
                slot(
                    &key,
                    name.clone(),
                    class,
                    format!("Server {key}: running {name}, attempt {attempt}, due tick {due}"),
                    false,
                )
            },
        });
    }

    for position in 0..QUEUE_SLOTS {
        let key = format!("Q{position}");
        let job = model.waiting.get(position).and_then(|id| model.job(*id));
        slots.push(match job {
            Some(j) => {
                let priority = priority_word(j.priority);
                let mut class = format!("ss-q ss-filled ss-{priority}");
                if chosen(j.id) {
                    class.push_str(" ss-selected");
                }
                slot(
                    &key,
                    format!("{} {}", j.key, if j.priority == 0 { "L" } else { "H" }),
                    &class,
                    format!("Position {}: {}, {priority} priority", position + 1, j.key),
                    false,
                )
            },
            None => slot(&key, String::new(), "ss-q ss-empty", String::new(), true),
        });
    }

    let more = model.waiting_total.saturating_sub(QUEUE_SLOTS);
    slots.push(if more > 0 {
        slot(
            "more",
            format!("+{more} more"),
            "ss-more",
            format!("{more} more waiting jobs"),
            false,
        )
    } else {
        slot("more", String::new(), "ss-more ss-empty", String::new(), true)
    });

    for ((key, name), value) in COUNTER_NAMES.iter().zip(counter_values(model)) {
        slots.push(slot(key, value.to_string(), "ss-counter", format!("{name}: {value}"), false));
    }

    slots.push(match selected {
        Some(j) => {
            let attempt = if j.attempt > 0 { format!(", attempt {}", j.attempt) } else { String::new() };
            slot(
                "selected",
                format!("{}: {}", j.key, j.state.word()),
                "ss-selected-job",
                format!(
                    "Selected job {}, {} priority, {}{attempt}",
                    j.key,
                    priority_word(j.priority),
                    j.state.word()
                ),
                false,
            )
        },
        None => slot(
            "selected",
            "none".into(),
            "ss-selected-job ss-none",
            "No job is selected".into(),
            false,
        ),
    });

    let c = &model.counters;
    let busy = model.servers.iter().filter(|s| matches!(s, ServerView::Busy { .. })).count();
    let data = [
        model.slice,
        model.tick,
        model.waiting_total as u64,
        busy as u64,
        c.completed,
        c.timed_out,
        c.cancelled,
        c.rejected,
    ];
    Projection { slots, data }
}
