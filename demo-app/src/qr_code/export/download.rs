//! Asking the browser to save the file, which is the one part of an export that needs the DOM.

use demo_common::util::stringify;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

/// How long the address of the file is kept, in milliseconds, before the browser may forget it. The save has started
/// long before.
const KEEP_MILLISECONDS: i32 = 10_000;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Asks the browser to save `svg` as a file called `file_name`.
///
/// The file is a `Blob` with an address of its own, which a link is pointed at and clicked, and the link is then taken
/// away. Nothing is sent anywhere, and nothing is written to the page. The address is released after a while.
///
/// # Errors
///
/// Returns `Err` if the browser refuses to make the file, the address or the link.
pub(crate) fn download(document: &web_sys::Document, file_name: &str, svg: &str) -> Result<(), String> {
    let parts = js_sys::Array::of1(&JsValue::from_str(svg));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("image/svg+xml");
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &options)
        .map_err(|e| format!("could not make the file: {e:?}"))?;
    let address = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("could not make an address for the file: {e:?}"))?;

    let link = document
        .create_element("a")
        .map_err(|e| format!("could not make a link: {e:?}"))?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| "the link is not an anchor".to_owned())?;
    link.set_href(&address);
    link.set_download(file_name);
    let body = document.body().ok_or_else(|| "the page has no body".to_owned())?;
    body.append_child(&link).map_err(|e| format!("could not add the link: {e:?}"))?;
    link.click();
    link.remove();

    release_later(address)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Lets go of the file's address after [`KEEP_MILLISECONDS`].
///
/// # Errors
///
/// Returns `Err` if there is no window to set the timer on.
fn release_later(address: String) -> Result<(), String> {
    let window = web_sys::window().ok_or_else(|| "no global window".to_owned())?;
    let release = Closure::once_into_js(move || {
        // The browser may already have forgotten the address, and there is nothing to do if it has.
        let _ = web_sys::Url::revoke_object_url(&address);
    });
    window
        .set_timeout_with_callback_and_timeout_and_arguments_0(release.unchecked_ref(), KEEP_MILLISECONDS)
        .map(|_| ())
        .map_err(stringify_js)
}

/// A JavaScript error as text.
fn stringify_js(error: JsValue) -> String {
    stringify(format!("{error:?}"))
}
