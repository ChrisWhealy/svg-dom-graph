//! The elements that the panel's own HTML declares: the text panel, the live region and the error line. The controls
//! are not among them, since they are drawn in the scene.

use super::projection::PANEL;
use demo_common::util::required_element;

pub(super) const STAGE: &str = "ek-stage";
pub(super) const SVG: &str = "ek-stage-svg";
const LIVE: &str = "ek-live";
const ERROR: &str = "ek-error";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The elements of the text panel and of the live region and the error line, all of which the page declares.
pub(super) struct Chrome {
    pub(super) live: web_sys::Element,
    pub(super) error: web_sys::Element,
    /// The fields of the text panel, in the order of [`PANEL`].
    pub(super) panel: Vec<web_sys::Element>,
}

impl Chrome {
    pub(super) fn find(document: &web_sys::Document) -> Result<Self, String> {
        Ok(Self {
            live: required_element(document, LIVE)?,
            error: required_element(document, ERROR)?,
            panel: PANEL
                .iter()
                .map(|(id, _)| required_element(document, id))
                .collect::<Result<_, _>>()?,
        })
    }
}
