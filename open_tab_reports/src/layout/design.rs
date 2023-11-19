use std::collections::HashMap;


struct Container {

}


enum LayoutDirective {
    Default,
    Absolute{x: f32, y: f32},
}

struct PageRect {
    page_id: usize,
    x: f32,
    y: f32,
    width: f32,
    height: f32
}

trait Layouter {
    fn next_rect(&mut self, parent: &mut Box<dyn Layouter>) -> PageRect;
}


struct PageLayouter {
    allow_page_break: bool,
}

struct DocumentDesign {
    templates: HashMap<String, Container>
}
