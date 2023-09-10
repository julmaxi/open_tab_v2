use swash::shape::ShapeContext;

use super::{FontCollection, GraphicsCollection};

pub struct LayoutContext<'a> {
    pub(crate) shape_context: &'a mut ShapeContext,
    pub(crate) font_collection: &'a mut FontCollection,
    pub(crate) graphics_collection: &'a mut GraphicsCollection,
}

impl<'a> LayoutContext<'a> {
    pub fn new(shape_context: &'a mut ShapeContext, font_collection: &'a mut FontCollection, graphics_collection: &'a mut GraphicsCollection) -> Self {
        Self {
            shape_context,
            font_collection: font_collection,
            graphics_collection: graphics_collection
        }
    }
}