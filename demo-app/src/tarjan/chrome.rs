//! The elements that the window's own HTML declares: the place of the scene, the text panel, the
//! live region and the error line. The graph, the stack, the roster and the condensation are all in the scene.

use super::projection::PANEL;
use demo_common::util::required_element;

/// The layer of controls, which the scene holds, and the place that the scene fills.
pub(super) const CONTROLS: &str = "tj-controls-svg";
pub(super) const SCENE: &str = "tj-scene";
const LIVE: &str = "tj-live";
const ERROR: &str = "tj-error";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The elements of the window that its HTML declares.
pub(super) struct Chrome {
    pub(super) live: web_sys::Element,
    pub(super) error: web_sys::Element,
    /// The place that the scene's `<svg>` is made in, which fills what is left of the window.
    pub(super) scene: web_sys::Element,
    /// The fields of the text panel, in the order of [`PANEL`].
    pub(super) panel: Vec<web_sys::Element>,
}

impl Chrome {
    pub(super) fn find(document: &web_sys::Document) -> Result<Self, String> {
        Ok(Self {
            live: required_element(document, LIVE)?,
            error: required_element(document, ERROR)?,
            scene: required_element(document, SCENE)?,
            panel: PANEL
                .iter()
                .map(|(id, _)| required_element(document, id))
                .collect::<Result<_, _>>()?,
        })
    }
}
