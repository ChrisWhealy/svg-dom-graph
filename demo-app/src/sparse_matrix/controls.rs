//! Reading the controls, and wiring a control to the action it runs.

use super::{ERROR, SEED};
use demo_common::util::{clear_error, document, report_error, required_element};
use spmv_core::{UpdateOrder, spec::Spec};
use spmv_scene::scene::element_count;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

pub(super) fn selected(id: &str) -> Result<String, String> {
    let element = required_element(&document()?, id)?;
    element
        .dyn_ref::<web_sys::HtmlSelectElement>()
        .map(web_sys::HtmlSelectElement::value)
        .ok_or_else(|| format!("#{id} is not a <select>"))
}

pub(super) fn number(id: &str) -> Result<u64, String> {
    selected(id)?.parse().map_err(|e| format!("#{id}: {e}"))
}

/// The scenario that the controls describe.
pub(super) fn spec_from_controls() -> Result<Spec, String> {
    let entities = number("spmv-size")?;
    let half = (entities / 2).max(1);
    let hub = selected("spmv-adjacency")? == "hub";
    // A data node cannot show more cells than the vector has entries, and a step cannot change more than it shows.
    let cells = number("spmv-cells")?.min(half);
    let updates = number("spmv-updates")?.min(cells);
    Ok(Spec {
        seed: SEED,
        rows: half,
        cols: half,
        per_row: 3,
        hub_cols: if hub { half.min(20) } else { 0 },
        hub_percent: if hub { 60 } else { 0 },
        label_len: number("spmv-labels")?,
        cells,
        updates,
        steps: 0,
    })
}

pub(super) fn body_elements() -> Result<u32, String> {
    element_count(&document()?.body().ok_or("no body")?.into())
}

pub(super) fn set_class(id: &str, class: &str) {
    if let Some(e) = document().ok().and_then(|d| d.get_element_by_id(id)) {
        e.set_class_name(class);
    }
}

pub(super) fn order_of(value: &str) -> UpdateOrder {
    if value == "inorder" { UpdateOrder::InOrder } else { UpdateOrder::Random }
}

/// Sets what a list shows, if it has such a value.
pub(super) fn set_list(id: &str, value: &str) {
    if let Some(list) = document()
        .ok()
        .and_then(|d| d.get_element_by_id(id))
        .and_then(|e| e.dyn_into::<web_sys::HtmlSelectElement>().ok())
    {
        list.set_value(value);
    }
}

pub(super) fn on(id: &str, event: &str, action: fn() -> Result<(), String>) -> Result<(), String> {
    let target = required_element(&document()?, id)?;
    let callback = Closure::<dyn FnMut()>::new(move || match action() {
        Ok(()) => clear_error(ERROR),
        Err(message) => report_error(ERROR, &message),
    });
    target
        .add_event_listener_with_callback(event, callback.as_ref().unchecked_ref())
        .map_err(|e| format!("could not wire #{id}: {e:?}"))?;
    callback.forget();
    Ok(())
}

/// As [`on`], for a box that a person types in. What they typed may not be a node that is there, which is the person's
/// to correct and not a fault of the page, so it is told in the banner and is not logged as an error.
pub(super) fn on_typed(id: &str, event: &str, action: fn() -> Result<(), String>) -> Result<(), String> {
    let target = required_element(&document()?, id)?;
    let callback = Closure::<dyn FnMut()>::new(move || match action() {
        Ok(()) => clear_error(ERROR),
        Err(message) => {
            if let Some(banner) = document().ok().and_then(|d| d.get_element_by_id(ERROR)) {
                banner.set_text_content(Some(&message));
                banner.remove_attribute("hidden").ok();
            }
        },
    });
    target
        .add_event_listener_with_callback(event, callback.as_ref().unchecked_ref())
        .map_err(|e| format!("could not wire #{id}: {e:?}"))?;
    callback.forget();
    Ok(())
}
