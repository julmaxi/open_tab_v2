use std::ops::Bound;

use itertools::Itertools;
use serde::{Serialize, Deserialize};

use crate::pdf::writable::ContentWriteable;

use super::{layout_trait::{Layoutable, PartiallyLayoutable, Layouter, BoundingBox}, LayoutContext, LayoutValue, LayoutResult, LayoutedElements, page::TemplateEntry};



#[derive(Serialize, Deserialize)]
pub struct DynamicRowLayout {
    min_row_size: Option<f32>,
    x: f32,
    y: f32,
    elements: Vec<TemplateEntry>,
}


pub struct TransformLayout {
    pub x: f32,
    pub y: f32,
    pub elements: Vec<Box<dyn ContentWriteable>>
}


impl TransformLayout {
    pub fn new(x: f32, y: f32, elements: Vec<Box<dyn ContentWriteable>>) -> Self {
        Self {
            x,
            y,
            elements
        }
    }
}

#[typetag::serde]
impl Layoutable for DynamicRowLayout {
    fn get_layouter<'a>(&'a self, value: &'a LayoutValue) -> Box<dyn Layouter<'a> + 'a> {
        let values = match value {
            LayoutValue::Vec(vec) => vec,
            _ => panic!("Expected Vec, got {:?}", value)
        };
        Box::new(RowLayouter {
            values,
            grid: self,
            curr_idx: 0
        })
    }
}

struct RowLayouter<'a> {
    values: &'a Vec<LayoutValue>,
    grid: &'a DynamicRowLayout,
    curr_idx: usize
}

impl<'a> Layouter<'a> for RowLayouter<'a> {
    fn layout(&mut self, context: &mut LayoutContext, bbox: BoundingBox) -> LayoutResult {
        let mut elements = vec![];
        let mut cursor_y = if self.curr_idx == 0 {
            self.grid.y
        }
        else {
            bbox.height
        };
        let mut max_width = 0.0;

        'outer: for val in self.values[self.curr_idx..].iter() {
            let mut local_elements = vec![];
            let mut min_y = 0.0;
            let mut max_y = 0.0;
            let remaining_height = cursor_y;
            for elem in self.grid.elements.iter() {
                let element_value = match (&elem.key, val) {
                    (Some(key), LayoutValue::Dict(dict)) => dict.get(key).unwrap_or(&LayoutValue::None),
                    _ => &LayoutValue::None
                };
                //let result = elem.element.layout(context, element_value);
                let mut layouter = elem.element.get_layouter(element_value);
                let result = layouter.layout(
                    context,
                    BoundingBox { x: 0.0, y: 0.0, width: bbox.width, height: remaining_height }
                );
                let width = result.bounding_box.width + result.bounding_box.x;
                max_width = f32::max(max_width, width);

                min_y = f32::min(min_y, result.bounding_box.y);
                max_y = f32::max(max_y, result.bounding_box.y + result.bounding_box.height);

                local_elements.extend(
                    result.objects
                );
            }
            cursor_y -= max_y;
            if cursor_y < bbox.y {
                break 'outer;
            }
            self.curr_idx += 1;

            elements.push(
                TransformLayout::new(
                    self.grid.x,
                    cursor_y,
                    local_elements
                )
            );
        }

        let objects : Vec<Box<dyn ContentWriteable>> = elements.into_iter().map(|e| {
            let x : Box<dyn ContentWriteable> = Box::new(e);
            x
        }).collect();
        LayoutResult {
            bounding_box: super::layout_trait::BoundingBox {
                x: self.grid.x,
                y: self.grid.y,
                width: max_width,
                height: self.grid.y - cursor_y
            },
            objects,
            is_done: self.curr_idx == self.values.len()
        }
    }
}



#[derive(Serialize, Deserialize)]
pub struct ColumnLayout {
    n_columns: u32,
    element: Box<dyn Layoutable>,
}

#[typetag::serde]
impl Layoutable for ColumnLayout {
    fn get_layouter<'a>(&'a self, value: &'a LayoutValue) -> Box<dyn Layouter<'a> + 'a> {
        let inner_layouter = self.element.get_layouter(value);
        Box::new(ColumnLayouter {
            value,
            grid: self,
            inner_layouter
        })
    }
}

struct ColumnLayouter<'a> {
    value: &'a LayoutValue,
    grid: &'a ColumnLayout,
    inner_layouter: Box<dyn Layouter<'a> + 'a>
}

impl<'a> Layouter<'a> for ColumnLayouter<'a> {
    fn layout(&mut self, context: &mut LayoutContext, bounds: BoundingBox) -> LayoutResult {
        let col_width = bounds.width / self.grid.n_columns as f32;

        let mut is_done = false;

        let mut elements = vec![];

        for col_idx in 0..self.grid.n_columns {
            dbg!(col_idx, col_idx as f32 * col_width);
            let col_x = bounds.x + col_idx as f32 * col_width;
            let col_height = bounds.height;

            let result = self.inner_layouter.layout(
                context,
                BoundingBox {
                    x: 0.0,
                    y: 0.0,
                    width: col_width,
                    height: col_height
                }
            );

            let column_transform = TransformLayout::new(
                col_x,
                0.0,
                result.objects
            );

            let b : Box<dyn ContentWriteable> = Box::new(column_transform);

            elements.push(
                b
            );

            dbg!(elements.len());

            if result.is_done {
                is_done = true;
                break;
            }
        };

        LayoutResult { bounding_box: bounds, objects: elements, is_done }

    }
}