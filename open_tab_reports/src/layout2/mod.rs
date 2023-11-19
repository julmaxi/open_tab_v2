use std::{sync::Arc, borrow::BorrowMut, collections::HashMap, hash::Hash, fmt::Debug};

use ab_glyph::Font;
use encoding_rs::WINDOWS_1252;
use font_kit::{source::SystemSource, font};
use itertools::{izip, Itertools};
use pdf_writer::{Ref, Rect, Name, Finish, Content, Str, TextStr, Filter, PdfWriter, types::{SystemInfo, FontFlags}, writers::Resources};
use serde::{Deserialize, Serialize};
use subsetter::{Profile, subset};
use svg2pdf::Options;
use swash::{FontRef, shape::{ShapeContext, Shaper, Direction}, text::{Script, analyze, cluster::{Parser, CharInfo, Token, CharCluster}}, CacheKey};
use tera::Tera;
use usvg::{TreeParsing, TreeTextToPath};

use ::image::{ColorType, GenericImageView, ImageFormat, DynamicImage, EncodableLayout};
use miniz_oxide::deflate::{compress_to_vec_zlib, CompressionLevel};

use std::collections::HashSet;

use crate::pdf::writable::{ContentWriteable, XObjectRenderable};

mod text;
mod image;
mod context;
mod layout_trait;
mod page;
mod grid;

pub use layout_trait::{LayoutValue, LayoutResult};
pub use page::{SinglePageTemplate, DocumentTemplate};
pub use grid::{TransformLayout, DynamicRowLayout};

pub use context::LayoutContext;
pub use text::{DynamicTextBox, LayoutDirection, TextLayoutResult, Instruction};
pub use self::image::FixedImage;
use self::page::PageLayoutInfo;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImageName {
    Path(String),
    Form(FormName)
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum XObjectRef {
    Image { id: u32 },
    XObject { id: u32 },
}


#[derive(Debug, Clone)]
pub struct XObjectLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub obj_ref: XObjectRef,
}

pub struct LayoutedElements {
    pub content: Vec<Box<dyn ContentWriteable>>
}

impl LayoutedElements {
    pub fn new() -> Self {
        Self {
            content: vec![]
        }
    }

    pub fn push(&mut self, element: Box<dyn ContentWriteable>) {
        self.content.push(element);
    }
}

impl From<Vec<Box<dyn ContentWriteable>>> for LayoutedElements {
    fn from(vec: Vec<Box<dyn ContentWriteable>>) -> Self {
        Self {
            content: vec
        }
    }
}

impl Extend<Box<dyn ContentWriteable>> for LayoutedElements {
    fn extend<T: IntoIterator<Item = Box<dyn ContentWriteable>>>(&mut self, iter: T) {
        self.content.extend(iter);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontUseRef {
    id: u32,
}


pub struct FontCollection {
    pub(crate) fonts: HashMap<FontUseRef, Arc<Vec<u8>>>,

    pub(crate) font_names: HashMap<String, FontUseRef>,
    pub(crate) reverse_font_names: HashMap<FontUseRef, String>,

    pub(crate) offsets: HashMap<FontUseRef, u32>,

    next_id: u32,

    keys: HashMap<FontUseRef, CacheKey>,
}

impl FontCollection {
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            font_names: HashMap::new(),
            reverse_font_names: HashMap::new(),
            next_id: 1,
            keys: HashMap::new(),
            offsets: HashMap::new()
        }
    }

    pub fn get_font_ref_from_postscript_name<'a>(&'a mut self, name: &str) -> FontRef<'a> {
        let font_id = if let Some(font_id) = self.font_names.get(name) {
            *font_id
        }
        else {
            self.load_font_from_postscript_name(name)
        };

        self.get_font_ref(&font_id)
    }

    pub fn get_id_from_postscript_name(&self, name: &str) -> Option<FontUseRef> {
        self.font_names.get(name).cloned()
    }

    pub fn get_font_ref<'a>(&'a self, font_id: &FontUseRef) -> FontRef<'a> {
        FontRef {
            data: self.fonts.get(&font_id).unwrap(),
            offset: self.offsets.get(&font_id).unwrap().clone(),
            key: self.keys.get(&font_id).unwrap().clone()
        }
    }

    pub fn load_font_from_postscript_name(
        &mut self,
        name: &str,
    ) -> FontUseRef {
        let font = SystemSource::new()
        .select_by_postscript_name(name).unwrap();
        let font_id = FontUseRef { id: self.next_id };
        self.next_id += 1;

        let (buf, font_index)= match font.clone() {
            font_kit::handle::Handle::Path { path, .. } => {
                println!("Path: {:?}", path);
                panic!("Path not supported")
            },
            font_kit::handle::Handle::Memory { bytes, font_index } => {
                (bytes, font_index as usize)
            }
        };
        self.fonts.insert(font_id, buf);
        let buf = self.fonts.get(&font_id).unwrap();

        let font = FontRef::from_index(&buf, font_index).unwrap();
        let (offset, key) = (font.offset, font.key);
        self.keys.insert(font_id, key);
        self.offsets.insert(font_id, offset);

        self.font_names.insert(name.to_string(), font_id);
        self.reverse_font_names.insert(font_id, name.to_string());

        font_id
    }
}


pub struct GraphicsCollection {
    pub objects: HashMap<XObjectRef, Box<dyn XObjectRenderable>>,

    pub loaded_images: HashMap<ImageName, XObjectRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FormName(pub String);

impl GraphicsCollection {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            loaded_images: HashMap::new(),
        }
    }

    pub fn get_fonts_and_glyphs(&self) -> HashMap<FontUseRef, HashSet<u16>> {
        let mut used_fonts = HashMap::new();
        for obj in self.objects.values() {
            for (font, used_glyphs) in obj.get_fonts_and_glyphs() {
                used_fonts.entry(font).or_insert_with(|| HashSet::new()).extend(used_glyphs);
            }
        }
        used_fonts
    }
    
    pub fn register_form(
        &mut self,
        name: FormName,
        elements: LayoutedElements,
    ) {
        let id = self.objects.len() as u32;
        self.objects.insert(XObjectRef::XObject { id }, Box::new(XObjectForm { elements, bounding_box: Rect::new(0.0, 0.0, 842.0, 595.0) }));
        self.loaded_images.insert(ImageName::Form(name.clone()), XObjectRef::XObject { id });
        
    }

    pub fn get_image_ref(&mut self, image_name: &ImageName) -> XObjectRef {
        if let Some(id) = self.loaded_images.get(image_name) {
            return id.clone();
        }
        match image_name {
            ImageName::Path(path) => {
                let image = Image::from_path(&path);
                let id = self.objects.len() as u32;
                self.objects.insert(XObjectRef::Image { id }, Box::new(image));
                XObjectRef::Image { id }
            }
            ImageName::Form(template) => {
                panic!("Unregistered Template!")
            }
        }
    }
}

pub struct LayoutedDocument {
    pub pages: Vec<PageLayoutInfo>,
}

impl LayoutedDocument {
    pub fn get_fonts_and_glyphs(&self) -> HashMap<FontUseRef, HashSet<u16>> {
        let mut used_fonts = HashMap::new();
        for page in self.pages.iter() {
            for (font, used_glyphs) in page.elements.get_fonts_and_glyphs() {
                used_fonts.entry(font).or_insert_with(|| HashSet::new()).extend(used_glyphs);
            }
        }
        used_fonts
    }
}

impl LayoutedElements {
    pub(crate) fn get_fonts_and_glyphs(&self) -> HashMap<FontUseRef, HashSet<u16>> {
        let mut used_fonts = HashMap::new();
        for (font, glyphs) in self.content.iter().flat_map(|c| c.get_fonts_and_glyphs()) {
            used_fonts.entry(*font).or_insert_with(|| HashSet::new()).extend(glyphs);
        }
        used_fonts
    }

    pub(crate) fn get_xobjects(&self) -> Vec<&XObjectRef> {
        self.content.iter().flat_map(|c| c.get_xobjects()).collect_vec()
    }
}

pub struct Image {
    pub(crate) data: DynamicImage,
    pub(crate) format: ImageFormat,
}

pub struct XObjectForm {
    pub elements: LayoutedElements,
    pub bounding_box: Rect
}