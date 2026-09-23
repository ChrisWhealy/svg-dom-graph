//! Every colour this crate's own rendering code uses, defined exactly once and named for what it means, rather
//! than repeated as a raw hex literal at each call site. `model::content` and `scene::node`/`scene::connector`
//! both draw from this one module, so a shade never drifts out of sync between two places that are supposed to
//! match — for example, [`BOX_STROKE`] on every box kind this crate ever draws, or [`CONNECTOR_STROKE`] shared by
//! a connector's own `<path>` and a non-commutative operator's own port marker.
//!
//! Every constant here is a plain SVG colour string — a hex triplet — passed straight into
//! [`SvgNode::set_fill`](svg_dom::SvgNode::set_fill)/[`SvgNode::set_stroke`](svg_dom::SvgNode::set_stroke). None of
//! them is computed; picking a new shade means editing exactly one line here, not hunting down every call site
//! that happens to share it.

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The border stroke colour every rendered box shares — a plain node's own box, a data node's own outer and/or
/// content box, an operator node's own outer and value-cell boxes, or a named data node's own outer box. One
/// shared border colour reads as "this crate's own box," regardless of which node kind or fill colour it wraps.
pub(crate) const BOX_STROKE: &str = "#2a5db0";

/// The label/value text fill colour every rendered `<text>` element shares — a plain node's own label, a data
/// node's own cell value(s), an operator node's own label and result, or a named data node's own name label. A
/// near-black rather than pure black, so text reads clearly against every pastel fill this module defines without
/// looking harsh.
pub(crate) const TEXT_FILL: &str = "#1b1b1b";

/// The default (unnamed) box background — a light pastel blue. Shared by a plain label node's own box, an
/// operator node's own outer box, and an unnamed data node's own outer box — single-value or multi-value grid
/// alike.
pub(crate) const PLAIN_BOX_FILL: &str = "#eef4ff";

/// A named data node's own outer box background — a pastel teal, deliberately distinct from [`PLAIN_BOX_FILL`]
/// (an operator node's own outer box, and an unnamed data node's own outer box, both already use it) and from
/// every [`TYPE_COLOUR_U8`]/[`TYPE_COLOUR_U16`]/[`TYPE_COLOUR_U32`]/[`TYPE_COLOUR_U64`] pastel the content box it
/// wraps could show. So the outer "this is named" box, an operator's own "this is computed" box, and the inner
/// "this value's own type" box each read as a distinct kind of box, never blending into one another.
pub(crate) const NAMED_BOX_FILL: &str = "#d6f2ee";

/// A connector's own `<path>` stroke colour — a muted grey, deliberately quiet next to the pastel boxes it
/// connects. Also the fill colour for a non-commutative operator's own "L"/"R" port marker, so the marker reads as
/// part of the connector it names, not as a separate accent colour competing for attention.
pub(crate) const CONNECTOR_STROKE: &str = "#555";

/// [`Scene`](crate::scene::Scene)`::set_selection`'s own row/column-level highlight colour — a warm yellow, chosen
/// to read clearly against every [`TYPE_COLOUR_U8`]/[`TYPE_COLOUR_U16`]/[`TYPE_COLOUR_U32`]/[`TYPE_COLOUR_U64`]
/// pastel and against [`PLAIN_BOX_FILL`] alike. Marks "we are now processing this row/column" in a
/// [`Selection::Row`](crate::scene::Selection::Row)/[`Selection::Column`](crate::scene::Selection::Column) walk.
pub(crate) const SELECTION_BAND: &str = "#ffe066";

/// [`Scene`](crate::scene::Scene)`::set_selection`'s own cell-level highlight colour — a stronger orange-red,
/// overriding [`SELECTION_BAND`] for the one cell a [`Selection::Cell`](crate::scene::Selection::Cell), or a
/// `Row`/`Column`'s own optional cell, names. Marks "and specifically this element."
pub(crate) const SELECTION_FOCUS: &str = "#ff6b4a";

/// [`NodeValues::type_colour`](crate::model::content::NodeValues::type_colour)'s own `u8` shade — pastel apricot.
pub(crate) const TYPE_COLOUR_U8: &str = "#fdebd3";

/// [`NodeValues::type_colour`](crate::model::content::NodeValues::type_colour)'s own `u16` shade — pastel mint.
pub(crate) const TYPE_COLOUR_U16: &str = "#dcefdc";

/// [`NodeValues::type_colour`](crate::model::content::NodeValues::type_colour)'s own `u32` shade — pastel lavender.
pub(crate) const TYPE_COLOUR_U32: &str = "#e6dcf5";

/// [`NodeValues::type_colour`](crate::model::content::NodeValues::type_colour)'s own `u64` shade — pastel rose.
pub(crate) const TYPE_COLOUR_U64: &str = "#f5dce4";
