//! The demo gallery's browser, and the address of the gallery.

use headless_chrome::Browser;

/// The demo gallery, built and served once per test run, and the browser that opens its pages.
///
/// Kept apart from [`Shared`](crate::common::shared::Shared), so a run that selects only fixture tests never pays for building
/// the demo.
pub(crate) struct DemoShared {
    pub(crate) browser: Browser,
    pub(crate) base_url: String,
}
