//! The reply to a `PartialAxTree` request, with the nodes left as raw JSON.

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(crate) struct PartialAxTreeReply {
    pub(crate) nodes: Vec<Value>,
}
