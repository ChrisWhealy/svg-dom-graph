//! The request for `Accessibility.getPartialAXTree` that the step driver tests make.

use crate::step_driver_accessibility::partial_ax_tree_reply::PartialAxTreeReply;
use headless_chrome::protocol::cdp::{DOM, types::Method};
use serde::Serialize;

/// `Accessibility.getPartialAXTree` for one node, as a request and a reply that these modules define themselves.
///
/// The `headless_chrome` bindings have a typed version, but it cannot read a node that Chrome ignores because of
/// `aria-hidden`: the reason `ariaHiddenElement` is not one of the variants it knows, so the whole reply fails to
/// parse. That reason is the very thing this test exists to see, so the reply is kept as raw JSON.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PartialAxTree {
    pub(crate) backend_node_id: DOM::BackendNodeId,
    pub(crate) fetch_relatives: bool,
}

impl Method for PartialAxTree {
    const NAME: &'static str = "Accessibility.getPartialAXTree";
    type ReturnObject = PartialAxTreeReply;
}
