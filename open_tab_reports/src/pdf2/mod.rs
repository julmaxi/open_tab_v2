use std::{sync::Arc, borrow::BorrowMut, collections::HashMap, hash::Hash, fmt::Debug};

use ab_glyph::Font;
use encoding_rs::WINDOWS_1252;
use font_kit::{source::SystemSource, font};
use itertools::{izip, Itertools};
use pdf_writer::{Ref, Rect, Name, Finish, Content, Str, TextStr, Filter, PdfWriter, types::{SystemInfo, FontFlags}, writers::Resources};
use subsetter::{Profile, subset};
use svg2pdf::Options;
use swash::{FontRef, shape::{ShapeContext, Shaper, Direction}, text::{Script, analyze, cluster::{Parser, CharInfo, Token, CharCluster}}, CacheKey};
use tera::Tera;
use usvg::{TreeParsing, TreeTextToPath};

use image::{ColorType, GenericImageView, ImageFormat, DynamicImage, EncodableLayout};
use miniz_oxide::deflate::{compress_to_vec_zlib, CompressionLevel};

use std::collections::HashSet;

use crate::layout::{XObjectLayout, FontUseRef, XObjectRef, LayoutedElements, DynamicTextBox, TextLayoutResult, Instruction, FontCollection, GraphicsCollection, LayoutedDocument, Image, XObjectForm, TransformLayout};

pub(crate) mod writable;

use writable::ContentWriteable;

use self::writable::XObjectRenderable;


impl ContentWriteable for XObjectLayout {
    fn write_to_pdf(&self, content: &mut Content, context: &ResourceDict) {
        content.save_state();
        content.transform([self.width, 0.0, 0.0, self.height, self.x, self.y]);
        content.x_object(context.resolve_ref(&self.obj_ref));
        content.restore_state();
    }

    fn get_xobjects(&self) -> Vec<&XObjectRef> {
        vec![&self.obj_ref]
    }

    fn get_fonts_and_glyphs(&self) -> Vec<(&FontUseRef, HashSet<u16>)> {
        vec![]
    }
}


pub struct PDFWritingContext {
    next_ref: i32,
    xobject_ids: HashMap<XObjectRef, Ref>,
    font_ids: HashMap<FontUseRef, Ref>,
}

impl PDFWritingContext {
    pub fn new() -> Self {
        Self {
            next_ref: 1,
            xobject_ids: HashMap::new(),
            font_ids: HashMap::new(),
        }
    }

    fn next_ref(&mut self) -> Ref {
        let new = Ref::new(self.next_ref);
        self.next_ref += 1;
        new
    }

    fn get_xobject_ref(&mut self, obj_ref: &XObjectRef) -> Ref {
        if let Some(id) = self.xobject_ids.get(obj_ref) {
            id.clone()
        }
        else {
            let id = self.next_ref();
            self.xobject_ids.insert(obj_ref.clone(), id);
            id
        }
    }

    fn get_font_ref(&mut self, obj_ref: &FontUseRef) -> Ref {
        //self.font_ids.get(obj_ref).unwrap().clone()
        if let Some(id) = self.font_ids.get(obj_ref) {
            id.clone()
        }
        else {
            let id = self.next_ref();
            self.font_ids.insert(obj_ref.clone(), id);
            id
        }
    }

    fn register_font(&mut self, obj_ref: FontUseRef) -> Ref {
        let id = self.next_ref();
        self.font_ids.insert(obj_ref, id);
        id
    }

    fn register_xobject(&mut self, obj_ref: XObjectRef) -> Ref {
        let id = self.next_ref();
        self.xobject_ids.insert(obj_ref, id);
        id
    }
}


pub struct ResourceDict {
    image_names: HashMap<XObjectRef, Vec<u8>>,
    font_names: HashMap<FontUseRef, Vec<u8>>,
}

impl ResourceDict {
    fn resolve_ref(&self, obj_ref: &XObjectRef) -> Name {
        match obj_ref {
            XObjectRef::Image { id } => Name(&self.image_names.get(obj_ref).unwrap()),
            XObjectRef::XObject { id } => Name(&self.image_names.get(obj_ref).unwrap()),
        }
    }

    fn resolve_font_ref(&self, obj_ref: &FontUseRef) -> Name {
        Name(self.font_names.get(obj_ref).unwrap())
    }

    pub fn new() -> Self {
        Self {
            image_names: HashMap::new(),
            font_names: HashMap::new()
        }
    }
    
    fn push_image(&mut self, obj_ref: XObjectRef) {
        let name = format!("Im{}", self.image_names.len() + 1);
        self.image_names.insert(obj_ref, name.as_bytes().into());
    }

    fn push_font(&mut self, obj_ref: FontUseRef) {
        let name = format!("F{}", self.font_names.len() + 1);
        self.font_names.insert(obj_ref, name.as_bytes().into());
    }

    fn write_to_resources(&self, resources: &mut Resources, context: &mut PDFWritingContext) {
        let mut x_objects = resources.x_objects();
        for (obj_ref, name) in self.image_names.iter() {
            x_objects.pair(Name(name), context.get_xobject_ref(obj_ref));
        }
        x_objects.finish();

        let mut fonts = resources.fonts();
        for (obj_ref, name) in self.font_names.iter() {
            fonts.pair(Name(name), context.get_font_ref(obj_ref));
        }
        fonts.finish();

    }
}

impl LayoutedElements {
    fn write_as_xobject(
        &self, writer: &mut PdfWriter, context: &mut PDFWritingContext, id: Ref, bounding_box: Rect
    ) {
        let resources = self.gather_resources(context);
        let content = self.write_content(context, &resources);
        let bytes = content.finish();

        let mut form = writer.form_xobject(id, &bytes);
        form.bbox(bounding_box);
        resources.write_to_resources(&mut form.resources(), context);
    }

    fn gather_resources(
        &self,
        context: &mut PDFWritingContext,
    ) -> ResourceDict {
        let mut resources = ResourceDict::new();

        let mut used_fonts: HashSet<FontUseRef> = HashSet::new();
        for (font, _) in self.get_fonts_and_glyphs() {
            used_fonts.insert(font.clone());
        }
        used_fonts.into_iter().for_each(|f| resources.push_font(f));
        
        let mut used_xobjects = HashSet::new();
        for xobject in self.get_xobjects() {
            used_xobjects.insert(xobject.clone());
        }
        used_xobjects.into_iter().for_each(|f| resources.push_image(f));

        resources
    }


    fn write_content(&self, context: &mut PDFWritingContext, resources: &ResourceDict) -> Content {
        let mut content = Content::new();

        for layout in self.content.iter() {
            layout.write_to_pdf(&mut content, resources);
        }

        content
    }

    fn write_as_page(&self, writer: &mut PdfWriter, context: &mut PDFWritingContext, id: Ref, parent: Ref, format: Rect) {
        let content_id = context.next_ref();
        let mut page = writer.page(id);
        
        page.media_box(format);
        page.parent(parent);
        page.contents(content_id);

        let resources = self.gather_resources(context);
        resources.write_to_resources(&mut page.resources(), context);

        page.finish();

        let content = self.write_content(context, &resources);
        writer.stream(content_id, &content.finish());
    }
}

impl ContentWriteable for TextLayoutResult {
    fn write_to_pdf(&self, content: &mut Content, resources: &ResourceDict) {
        let mut curr_position = (0.0, 0.0);
        let font_name = resources.resolve_font_ref(&self.font);
        content.set_font(font_name, self.font_size);
        //content.set_font(Name(b"F1"), self.font_size);
        self.instructions.iter().for_each(
            |i| {
                match i {
                    Instruction::Run { start, stop } => {
                        content.begin_text();
                        content.next_line(curr_position.0, curr_position.1);
                        let s = &self.glyph_ids[*start..*stop].iter().map(|g| g.to_be_bytes()).flatten().collect::<Vec<u8>>();
                        content.show(Str(s));
                        content.end_text();
                    },
                    Instruction::MoveTo { x, y } => {
                        curr_position = (*x, *y)
                    }
                }
            }
        );
    }

    fn get_xobjects(&self) -> Vec<&XObjectRef> {
        vec![]
    }

    fn get_fonts_and_glyphs(&self) -> Vec<(&FontUseRef, HashSet<u16>)> {
        let used_glyphs = self.glyph_ids.iter().cloned().collect::<HashSet<_>>();
        vec![(
            &self.font,
            used_glyphs
        )]
    }
}

impl FontCollection {
    pub fn write_fonts_to_pdf(&self, writer: &mut PdfWriter, context: &mut PDFWritingContext, fonts: Vec<(FontUseRef, HashSet<u16>)>) {
        for (font_id, used_glyphs) in fonts.into_iter() {
            let font_ref = context.get_font_ref(&font_id);
            let loaded_font = self.get_font_ref(&font_id);
            let scale = 1000.0 / loaded_font.glyph_metrics(&[]).units_per_em() as f32;
            let glyph_widths = used_glyphs.iter().map(|g| (*g, loaded_font.glyph_metrics(&[]).advance_width(*g) * scale)).collect::<HashMap<_, _>>();

            let font_desc_id = context.next_ref();
            let font_id_2 = context.next_ref();
            let font_file_id = context.next_ref();

            let base_name = self.reverse_font_names.get(&font_id).unwrap();

            writer.type0_font(font_ref).base_font(Name(base_name.as_bytes())).encoding_predefined(Name(b"Identity-H")).descendant_font(font_id_2);
        
            let mut cid_font = writer.cid_font(font_id_2);
            cid_font.base_font(Name(base_name.as_bytes()))
            .subtype(pdf_writer::types::CidFontType::Type2)
            .default_width(1000.0)
            .system_info(SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"Identity"),
                supplement: 0,
            })
            .font_descriptor(font_desc_id)
            ;
        
            let mut widths = cid_font.widths();
            for (g, w) in glyph_widths.into_iter() {
                widths.consecutive(g, [w].into_iter());
            }
            widths.finish();
        
        
            cid_font.finish();
            let metrics = loaded_font.metrics(&[]);
        
            writer.font_descriptor(font_desc_id)
            .font_file2(font_file_id)
            .ascent(
                metrics.ascent
            )
            .cap_height(
                metrics.cap_height
            )
            .descent(
                metrics.descent
            )
            .flags(FontFlags::NON_SYMBOLIC)
            //.bbox(Rect { x1: -664., y1: 324.7, x2: 2028.32, y2: 1037.1 })
            .italic_angle(0.0)
            .stem_v(1000.)
            .name(Name(base_name.as_bytes()))
            .finish(); 
        }
    }
}


impl XObjectRenderable for XObjectForm {
    fn render_as_xobject(&self, pdf_writer: &mut PdfWriter, context: &mut PDFWritingContext, id: Ref) {
        self.elements.write_as_xobject(pdf_writer, context, id, self.bounding_box);
    }

    fn get_fonts_and_glyphs(&self) -> HashMap<FontUseRef, HashSet<u16>> {
        self.elements.get_fonts_and_glyphs()
    }
}

impl Image {
    pub fn from_path(path: &str) -> Self {
        let data = std::fs::read(path).unwrap();
        let format = image::guess_format(&data).unwrap();
        let dynamic = image::load_from_memory(&data).unwrap();    

        Self {
            data: dynamic,
            format
        }
    }
}



impl XObjectRenderable for Image {
    fn render_as_xobject(&self, pdf_writer: &mut PdfWriter, context: &mut PDFWritingContext, id: Ref) {
        let (filter, encoded, mask) = match self.format {
            // A JPEG is already valid DCT-encoded data.
            ImageFormat::Jpeg => {
                assert!(self.data.color() == ColorType::Rgb8);
                (Filter::DctDecode, self.data.as_bytes().into_iter().cloned().collect(), None)
            }
    
            ImageFormat::Png => {
                let level = CompressionLevel::DefaultLevel as u8;
                let encoded = compress_to_vec_zlib(self.data.to_rgb8().as_raw(), level);
    
                // If there's an alpha channel, extract the pixel alpha values.
                let mask = self.data.color().has_alpha().then(|| {
                    let alphas: Vec<_> = self.data.pixels().map(|p| (p.2).0[3]).collect();
                    compress_to_vec_zlib(&alphas, level)
                });
    
                (Filter::FlateDecode, encoded, mask)
            }
            _ => panic!("unsupported image format"),
        };
    
        let image_id = id;
        let s_mask_id = context.next_ref();
        let mut image = pdf_writer.image_xobject(image_id, &encoded);
        image.filter(filter);
        image.width(self.data.width() as i32);
        image.height(self.data.height() as i32);
        image.color_space().device_rgb();
        image.bits_per_component(8);
        if mask.is_some() {
            image.s_mask(s_mask_id);
        }
        image.finish();
    
        if let Some(encoded) = &mask {
            let mut s_mask = pdf_writer.image_xobject(s_mask_id, encoded);
            s_mask.filter(filter);
            s_mask.width(self.data.width() as i32);
            s_mask.height(self.data.height() as i32);
            s_mask.color_space().device_gray();
            s_mask.bits_per_component(8);
        }     
    }

    fn get_fonts_and_glyphs(&self) -> HashMap<FontUseRef, HashSet<u16>> {
        HashMap::new()
    }
}

impl GraphicsCollection {
    pub fn write_to_pdf(&self, pdf_writer: &mut PdfWriter, context: &mut PDFWritingContext) {
        for (obj_ref, obj) in self.objects.iter() {
            let id = context.get_xobject_ref(&obj_ref);
            obj.render_as_xobject(pdf_writer, context, id);
        }
    }
}

impl LayoutedDocument {
    pub fn write_to_pdf(&self, writer: &mut PdfWriter, context: &mut PDFWritingContext) {
        let catalog_id = context.next_ref();
        let page_tree_id = context.next_ref();
        let page_ids = self.pages.iter().map(|_| context.next_ref()).collect_vec();

        // Set up the page tree. For more details see `hello.rs`.
        writer.catalog(catalog_id).pages(page_tree_id);
        writer.pages(page_tree_id).kids(page_ids.clone()).count(self.pages.len() as i32);

        for (page, id) in self.pages.iter().zip(page_ids) {
            let dim = page.format.get_dimensions();
            page.elements.write_as_page(writer, context, id, page_tree_id, Rect { x1: 0.0, y1: 0.0, x2: dim.0, y2: dim.1 });
        }
    }
}

impl ContentWriteable for TransformLayout {
    fn write_to_pdf(&self, content: &mut Content, resources: &ResourceDict) {
        content.save_state();
        content.transform([1.0, 0.0, 0.0, 1.0, self.x, self.y]);
        for element in self.elements.iter() {
            element.write_to_pdf(content, resources);
        }
        content.restore_state();
    }
    
    fn get_xobjects(&self) -> Vec<&XObjectRef> {
        self.elements.iter().map(|e| e.get_xobjects()).flatten().collect::<Vec<_>>()
    }

    fn get_fonts_and_glyphs(&self) -> Vec<(&FontUseRef, HashSet<u16>)> {
        self.elements.iter().map(|e| e.get_fonts_and_glyphs()).flatten().collect::<Vec<_>>()
    }
}