//! One bar of a sorting demo's diagram, as the page draws it.

/// One bar as the page draws it.
#[derive(serde::Deserialize, Debug)]
pub(crate) struct Bar {
    pub(crate) value: u8,
    pub(crate) fill: String,
    pub(crate) height: f64,
    pub(crate) left: f64,
    pub(crate) bottom: f64,
}
