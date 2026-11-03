//! The elements that the panel's own HTML declares: the status and the live region. The controls are not among them,
//! since they are drawn in the scene.

use demo_common::util::required_element;

/// The ids of the elements that the page declares.
const LIVE: &str = "n-queens-live";
const ERROR: &str = "n-queens-error";
pub(super) const INFO: &str = "n-queens-board-info";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The elements of the status and of the live region, all of which the page declares.
pub(super) struct Chrome {
    pub(super) live: web_sys::Element,
    pub(super) error: web_sys::Element,
    pub(super) info: web_sys::Element,
    /// The status fields, in the order of [`projection::Status`], then the field for the playback.
    pub(super) fields: [web_sys::Element; 8],
}

impl Chrome {
    pub(super) fn find(document: &web_sys::Document) -> Result<Self, String> {
        let get = |id: &str| required_element(document, id);
        Ok(Self {
            live: get(LIVE)?,
            error: get(ERROR)?,
            info: get(INFO)?,
            fields: [
                get("n-queens-st-position")?,
                get("n-queens-st-kind")?,
                get("n-queens-st-cell")?,
                get("n-queens-st-depth")?,
                get("n-queens-st-reason")?,
                get("n-queens-st-solutions")?,
                get("n-queens-st-vector")?,
                get("n-queens-st-playback")?,
            ],
        })
    }
}
