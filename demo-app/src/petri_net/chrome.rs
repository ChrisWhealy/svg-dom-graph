//! The elements that the panel's own HTML declares: the text panel, the notice, the live region and the error line. The
//! controls are not among them, since they are drawn in the scene.

use super::projection::PANEL;
use demo_common::util::required_element;

pub(super) const STAGE: &str = "pn-stage";
pub(super) const SVG: &str = "pn-stage-svg";
const LIVE: &str = "pn-live";
const ERROR: &str = "pn-error";
const NOTICE: &str = "pn-notice";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The elements of the text panel and of the notice, the live region and the error line, all of which the page
/// declares.
pub(super) struct Chrome {
    pub(super) live: web_sys::Element,
    pub(super) error: web_sys::Element,
    pub(super) notice: web_sys::Element,
    /// The fields of the text panel, in the order of [`PANEL`].
    pub(super) panel: Vec<web_sys::Element>,
}

impl Chrome {
    pub(super) fn find(document: &web_sys::Document) -> Result<Self, String> {
        Ok(Self {
            live: required_element(document, LIVE)?,
            error: required_element(document, ERROR)?,
            notice: required_element(document, NOTICE)?,
            panel: PANEL
                .iter()
                .map(|(id, _)| required_element(document, id))
                .collect::<Result<_, _>>()?,
        })
    }
}
