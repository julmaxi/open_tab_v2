use std::collections::HashMap;

use serde::Serialize;

use crate::pdf::writable::ContentWriteable;

use super::{LayoutContext};

#[derive(Debug, Clone)]
pub enum LayoutValue {
    String(String),
    Dict(HashMap<String, LayoutValue>),
    Vec(Vec<LayoutValue>),
    None,
}

pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub struct LayoutResult {
    pub bounding_box: BoundingBox,
    pub objects: Vec<Box<dyn ContentWriteable>>
}

#[typetag::serde(tag = "type")]
pub trait Layoutable  {
    fn layout(&self, context: &mut LayoutContext, value: &LayoutValue) -> LayoutResult;
}