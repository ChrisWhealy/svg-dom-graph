use crate::scene::connector::connector_type::ConnectorType;
use svg_dom::SvgNode;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The rendered `<path>` for one edge's connector, plus the [`ConnectorType`] it was created with.
///
/// `redraw_edge` has no other way to learn an edge's connector type once a node move forces a reroute. This value must
/// live alongside the rendered handle, not just get used once at creation.
pub struct ConnectorHandle {
    pub path: SvgNode,
    pub connector_type: ConnectorType,
}
