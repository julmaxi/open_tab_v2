use serde::{Deserialize, Serialize};

use super::{ImageName, GraphicsCollection, XObjectLayout, LayoutContext, layout_trait::{Layoutable, BoundingBox}};


#[derive(Serialize, Deserialize, Clone)]
pub struct FixedImage {
    pub width: f32,
    pub height: f32,
    pub x: f32,
    pub y: f32,
    pub image_name: ImageName
}

impl FixedImage {
    pub fn new(width: f32, height: f32, x: f32, y: f32, image_name: ImageName) -> Self {
        Self {
            width,
            height,
            x,
            y,
            image_name
        }
    }

    pub fn layout(&self, context: &mut LayoutContext) -> XObjectLayout {
        let ref_id = context.graphics_collection.get_image_ref(&self.image_name);

        XObjectLayout {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            obj_ref: ref_id
        }
    }
}

#[typetag::serde]
impl Layoutable for FixedImage {
    fn get_layouter<'a>(&'a self, _value: &'a super::LayoutValue) -> Box<dyn super::layout_trait::Layouter<'a> + 'a> {
        Box::new(ImageLayouter {
            image: self,
            value: _value
        })
    }
}


struct ImageLayouter<'a> {
    image: &'a FixedImage,
    value: &'a super::LayoutValue
}

impl<'a> super::layout_trait::Layouter<'a> for ImageLayouter<'a> {
    fn layout(&mut self, context: &mut LayoutContext, bbox: BoundingBox) -> super::LayoutResult {
        super::LayoutResult {
            bounding_box: super::layout_trait::BoundingBox {
                x: self.image.x,
                y: self.image.y,
                width: self.image.width,
                height: self.image.height
            },
            objects: vec![
                Box::new(self.image.layout(context))
            ],
            is_done: true
        }
    }
}