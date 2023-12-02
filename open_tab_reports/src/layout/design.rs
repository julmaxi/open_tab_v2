use std::{collections::HashMap, rc::Weak};

use super::LayoutedElement;


struct Container {

}


enum LayoutDirective {
    Default,
    Absolute{x: f32, y: f32},
}

struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32
}


trait Layouter {
    fn next_rect(&mut self) -> Rect;
}

struct PageLayouter {
    allow_page_break: bool,
}

struct HorizontalLayouter {
    margin: f32,
    parent: Weak<dyn Layouter>,
}

struct DocumentDesign {
    templates: HashMap<String, Container>
}
