// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `EdgeAnchors` defines the number of evenly spaced connector fixing points available on each of a node's four sides.
/// This can be configured instead of the default single anchor point every connector uses by default.
///
/// The wrapped value must be `>= 1`.
///
/// `Scene::add_node_with`/`Scene::set_edge_anchors` reject `0` with [`Error::InvalidEdgeAnchors`] — a side with no
/// candidate point cannot anchor a connector, so `0` has no meaning here.
///
/// Consequently, you must use `None` rather than `Some(EdgeAnchors(0))` to keep a connector's own default anchor rule.
///
/// # How a connector picks one of the `n` candidates
///
/// A connector still picks *which side* to leave from exactly as it always does: by the ray from this node's own centre
/// toward the other endpoint's centre, and whichever side that ray crosses first. `EdgeAnchors` only changes *where on
/// that side* the connector lands.
///
/// That side is divided into `n + 1` equal segments, giving `n` internal division points — the two corners bounding the
/// side are never candidates. The connector then snaps to whichever of those `n` points sits closest to where the
/// unsnapped ray would have crossed.
///
/// If the two nodes' centres exactly coincide, that ray has no direction to pick a side from. This is the same
/// pre-existing degenerate case ordinary, unconfigured routing already has to handle, and `EdgeAnchors` resolves
/// it the same way: falling back to this node's own centre and `Side::East`, rather than an actual fixing point.
///
/// # `EdgeAnchors(1)` does not always match `None`
///
/// With `n = 1`, `n + 1 = 2` and this makes the segment's internal division point identical to the side's midpoint.
/// For an elbow connector this is no change at all — the default (`None`) elbow rule already always anchors at the same
/// midpoint, so `Some(EdgeAnchors(1))` and `None` render identically.
///
/// For a straight connector however, this does *not* generally hold: its default (`None`) is the ray's own exact
/// boundary crossing, which would only land on the midpoint by coincidence. `Some(EdgeAnchors(1))` forces a straight
/// connector onto the midpoint regardless, so the two will often render at visibly different points.
///
/// Every connector touching this node makes this choice independently, from its own other endpoint's position alone.
/// Fixing points are not reserved or assigned: nothing stops two, or all, of a node's incident connectors from landing
/// on the same point — there is no occupancy tracking or one-connector-per-point allocation.
///
/// `EdgeAnchors(5)` means "five candidate positions per side," not "capacity for five edges."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeAnchors(pub u8);
