use std::{collections::HashMap, alloc::Layout};

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

#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub struct LayoutResult {
    pub bounding_box: BoundingBox,
    pub objects: Vec<Box<dyn ContentWriteable>>,
    pub is_done: bool
}

pub struct PartialLayoutResult {
    pub result: LayoutResult,
    pub num_consumed: usize
}

pub trait Layouter<'a> {
    fn layout(&mut self, context: &mut LayoutContext, bounds: BoundingBox) -> LayoutResult;
}

#[typetag::serde(tag = "type")]
pub trait Layoutable  {
    fn get_layouter<'a>(&'a self, value: &'a LayoutValue) -> Box<dyn Layouter<'a> + 'a>;

    fn layout(&self, context: &mut LayoutContext, value: &LayoutValue) -> LayoutResult {
        let mut layouter = self.get_layouter(value);
        layouter.layout(context, BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0
        })
    }
}

pub trait PartiallyLayoutable {
    fn layout_values(&self, context: &mut LayoutContext, values: &[&LayoutValue], bounding_box: BoundingBox) -> PartialLayoutResult;
}