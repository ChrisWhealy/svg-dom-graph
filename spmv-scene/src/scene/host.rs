//! An `<svg>` in a `<div>` of its own, which is what a scene is built on and what a teardown removes.

use super::{err, js_err};
use svg_dom::{SvgRoot, root::utils::Size};

/// An `<svg>` in a `<div>` of its own, which is what a scene is built on and what a teardown removes.
pub struct Host {
    pub container_id: String,
    pub svg_id: String,
    /// Whether the host made its container, and so removes it. A host made in a container that was already in the page
    /// removes only its own `<svg>`.
    owns_container: bool,
}

impl Host {
    /// Makes the host in a `<div>` of its own in the document body, with a view box of `extent`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the elements could not be made.
    pub fn create(id: &str, extent: (f64, f64)) -> Result<Self, String> {
        let document = web_sys::window().and_then(|w| w.document()).ok_or("there is no document")?;
        let container_id = format!("{id}-container");
        let div = document.create_element("div").map_err(js_err("div"))?;
        div.set_id(&container_id);
        document.body().ok_or("no body")?.append_child(&div).map_err(js_err("append"))?;
        let mut host = Self::create_in(&container_id, id, extent)?;
        host.owns_container = true;
        Ok(host)
    }

    /// Makes the host's `<svg>` in the element called `container_id`, which is already in the page. Removing the host
    /// removes only the `<svg>`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the container is missing or the `<svg>` could not be made.
    pub fn create_in(container_id: &str, id: &str, extent: (f64, f64)) -> Result<Self, String> {
        // The viewport has the aspect of the layout, so that a wide, short graph is not drawn small in a tall box.
        let height = (900.0 * extent.1 / extent.0.max(1.0)).clamp(240.0, 1400.0);
        let svg = SvgRoot::create_in(container_id, Size::new(900.0, height)).map_err(err("svg"))?;
        svg.root.set_id(id);
        svg.set_view_box(0.0, 0.0, extent.0.max(1.0), extent.1.max(1.0))
            .map_err(err("view box"))?;
        Ok(Self {
            container_id: container_id.to_owned(),
            svg_id: id.to_owned(),
            owns_container: false,
        })
    }

    /// The `<svg>`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if it is no longer in the document.
    pub fn svg(&self) -> Result<web_sys::Element, String> {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id(&self.svg_id))
            .ok_or_else(|| format!("no element #{}", self.svg_id))
    }

    /// Removes what the host made: the container and everything in it, or, for a host made in a container of the
    /// page's, only its `<svg>`.
    pub fn remove(&self) {
        let id = if self.owns_container { &self.container_id } else { &self.svg_id };
        if let Some(element) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id(id))
        {
            element.remove();
        }
    }
}
