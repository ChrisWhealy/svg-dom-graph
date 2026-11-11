//! The browser that opens the fixture page, and the address of that page.

use headless_chrome::Browser;

pub(crate) struct Shared {
    // This is never read after construction, but it must outlive every test. Dropping `Browser` closes the Chrome
    // process and with it, every `Tab` opened from it.
    pub(crate) browser: Browser,
    pub(crate) base_url: String,
}
