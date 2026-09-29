//! Shared fixture and helpers for every scenario module in this test group.

use svg_dom::{SvgRoot, root::utils::Size};

pub(super) fn document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

/// A fresh `SvgRoot` in its own container, with a unique `id` so parallel tests in this binary do not collide.
pub(super) fn make_svg(id: &str) -> SvgRoot {
    let container_id = format!("{id}-container");
    let el = document().create_element("div").unwrap();
    el.set_id(&container_id);
    document().query_selector("body").unwrap().unwrap().append_child(&el).unwrap();

    let svg = SvgRoot::create_in(&container_id, Size::new(200.0, 200.0)).unwrap();
    svg.root.set_id(id);
    svg
}

pub(super) fn visibility(svg: &SvgRoot) -> Option<String> {
    svg.root.get_attribute("visibility")
}

/// Makes `Element.setAttribute` throw for the named attributes while it is alive, and restores the original when
/// dropped. A small, self-contained copy of `tests/drag/toolbar/failure_injection.rs`'s own `FailingWrites`: that
/// one lives in the external integration-test crate, out of reach from this crate's own internal `#[cfg(test)]`
/// suite. Shared here by [`super::make_enterable`] and [`super::replace_container_child`], the two scenario modules
/// that each need a DOM write to fail on purpose.
pub(super) struct FailingWrites;

impl FailingWrites {
    pub(super) fn start(names: &[&str]) -> Result<Self, String> {
        let list = names.iter().map(|n| format!("{n:?}")).collect::<Vec<_>>().join(", ");
        js_sys::Function::new_no_args(&format!(
            "const proto = Element.prototype;
             if (!proto.__originalSetAttribute) {{ proto.__originalSetAttribute = proto.setAttribute; }}
             const failing = [{list}];
             proto.setAttribute = function (name, value) {{
                 if (failing.includes(name)) {{ throw new Error('injected failure writing ' + name); }}
                 return proto.__originalSetAttribute.apply(this, arguments);
             }};"
        ))
        .call0(&wasm_bindgen::JsValue::NULL)
        .map_err(|e| format!("{e:?}"))?;
        Ok(Self)
    }
}

impl Drop for FailingWrites {
    fn drop(&mut self) {
        let _ = js_sys::Function::new_no_args(
            "const proto = Element.prototype;
             if (proto.__originalSetAttribute) { proto.setAttribute = proto.__originalSetAttribute; }",
        )
        .call0(&wasm_bindgen::JsValue::NULL);
    }
}
