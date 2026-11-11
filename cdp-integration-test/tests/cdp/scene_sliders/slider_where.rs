//! Where a slider of a scene is.

/// A slider of a scene: where its svg is, and where its track is in the scene.
pub(crate) struct Where {
    pub(crate) svg: &'static str,
    pub(crate) id: &'static str,
    pub(crate) left: f64,
    pub(crate) middle: f64,
    pub(crate) width: f64,
}
