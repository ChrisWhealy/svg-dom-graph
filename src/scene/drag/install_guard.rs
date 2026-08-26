use super::DRAG_EVENT_TYPES;
use svg_dom::SvgNode;

/// Removes every listener [`Scene::make_draggable_with`] may have registered on `group`, unless [`disarm`](Self::disarm)
/// is called first.
///
/// A `?` on any fallible step between construction and [`disarm`](Self::disarm) — `set_attr`, or any one of the
/// four listener registrations — drops this still-armed, unwinding installation back to exactly the state it
/// found `group` in. `SvgNode::remove_listeners` is a no-op for an event type nothing was registered for, so
/// unconditionally removing all four here is always safe, whether registration got partway through or never
/// started.
pub struct InstallGuard {
    pub group: SvgNode,
    pub armed: bool,
}

impl InstallGuard {
    pub fn new(group: SvgNode) -> Self {
        Self { group, armed: true }
    }

    /// Installation finished successfully — do not roll it back on drop.
    pub fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for InstallGuard {
    fn drop(&mut self) {
        if self.armed {
            for event_type in DRAG_EVENT_TYPES {
                self.group.remove_listeners(event_type);
            }
        }
    }
}
