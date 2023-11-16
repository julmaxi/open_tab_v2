use std::{collections::HashMap, cell::RefCell};

use pdf_writer::Rect;

pub mod font;
pub mod design;

use font::Font;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontRef(pub(crate) usize);

impl FontRef {
    pub fn new(id: usize) -> Self {
        return Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum GraphicsRef {
    Template(usize),
    Image(usize)
}

pub(crate) struct Image {
    pub(crate) data: Vec<u8>
}

#[derive(Debug, Clone, Copy)]
pub struct PageDimensions {
    pub width: f32,
    pub height: f32
}

impl PageDimensions {
    pub fn a4() -> Self {
        Self {
            width: 595.0,
            height: 842.0
        }
    }
}

impl Into<Rect> for PageDimensions {
    fn into(self) -> Rect {
        Rect { x1: 0.0, y1: 0.0, x2: self.width, y2: self.height }
    }
}

pub struct LayoutedPage {
    pub(crate) dimensions: PageDimensions,
    pub(crate) elements: Vec<LayoutedElement>
}

impl LayoutedPage {
    pub fn new(dimensions: PageDimensions) -> Self {
        Self {
            dimensions,
            elements: vec![]
        }
    }

    pub fn add_element(&mut self, element: LayoutedElement) {
        self.elements.push(element);
    }

    pub fn add_text(&mut self, element: TextElement) {
        self.add_element(LayoutedElement::Text(element));
    }
}

pub enum LayoutedElement {
    Text(TextElement),
    Image(GraphicElement),
    Group(GroupElement)
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub(crate) x: f32,
    pub(crate) y: f32
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Position { x, y }
    }
}

#[derive(Debug)]
pub enum Instruction {
    Run{start: usize, stop: usize},
    MoveTo{ x: f32, y: f32 },
}


pub struct TextElement {
    pub glyph_ids: Vec<u16>,
    pub instructions: Vec<Instruction>,
    pub font_size: f32,
    pub font: FontRef
}

pub struct GraphicElement {
    pub(crate) pos: Position,
    pub(crate) image: GraphicsRef
}

pub struct GroupElement {
    pub(crate) children: Vec<Box<LayoutedElement>>
}

pub struct LayoutedDocument {
    pub(crate) fonts: Vec<Font>,
    pub(crate) graphics: Vec<Image>,
    pub(crate) pages: Vec<LayoutedPage>,
    pub(crate) templates: HashMap<usize, LayoutedPage>
}

impl LayoutedDocument {
    pub fn new() -> Self {
        Self {
            fonts: vec![],
            graphics: vec![],
            pages: vec![],
            templates: HashMap::new()
        }
    }

    pub fn add_font(&mut self, font: Font) -> FontRef {
        let id = self.fonts.len();
        self.fonts.push(font);
        FontRef::new(id)
    }

    pub fn add_page(&mut self, page: LayoutedPage) {
        self.pages.push(page);
    }
}