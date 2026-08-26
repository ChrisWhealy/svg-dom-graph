use super::DRAG_EVENT_TYPES;
use svg_dom::SvgNode;

/// Unless [`disarm`](Self::disarm) is called first, this removes every listener [`Scene::make_draggable_with`] may have
/// registered on a `group`.
///
/// A `?` on any fallible step occurring between construction and [`disarm`](Self::disarm) (which may be `set_attr`, or
/// any one of the four listener registrations) drops this `groups`'s still-armed, unwinding installation back to
/// exactly its previous state.
///
/// Since `SvgNode::remove_listeners` is a no-op for an event type for which nothing has been registered,
/// unconditionally removing all four here is always safe, irrespective of progress (including not started).
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
