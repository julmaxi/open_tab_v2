use std::collections::{HashMap, HashSet};


use itertools::Itertools;
use pdf_writer::{PdfWriter, Ref, Content, Name, Str, writers::Resources, Finish, types::{FontFlags, SystemInfo}, Rect};

use crate::layout::{LayoutedDocument, TextElement, FontRef, Instruction, Position, LayoutedPage, LayoutedElement, GraphicsRef, QRCodeElement};

struct Context {
    next_val: i32,
    font_refs: HashMap<FontRef, Ref>
}

impl Context {
    fn new() -> Self {
        Context { next_val: 1, font_refs: HashMap::new() }
    }

    fn next_ref(&mut self) -> Ref {
        let val = self.next_val;
        self.next_val += 1;
        Ref::new(
            val
        )
    }
}

struct LocalContext {
    font_names: HashMap<FontRef, String>
}

impl LocalContext {
    fn new() -> Self {
        Self {
            font_names: HashMap::new()
        }
    }

    fn register_font(&mut self, font: FontRef) {
        let id = self.font_names.len();
        let name = format!("F{}", id);

        self.font_names.insert(
            font,
            name
        );
    }

    fn get_name_for_font<'a>(&'a self, font_ref: FontRef) -> Option<Name<'a>> {
        self.font_names.get(&font_ref).map(|k| Name(k.as_bytes()))
    }

    fn write_to_resources(&self, resources: &mut Resources, context: &mut Context) {
        let mut fonts = resources.fonts();

        for (font, name) in self.font_names.iter() {
            fonts.pair(
                Name(name.as_bytes()),
                context.font_refs.get(font).expect("Font missing")
            );
        }

    }
}

impl TextElement {
    fn write_to_content(&self, content: &mut Content, _context: &Context, local_context: &LocalContext) {
        content.begin_text();

        content.set_font(local_context.get_name_for_font(self.font).expect("Missing resource"), self.font_size);
        
        content.begin_text();

        let mut curr_position = Position::new(0.0, 0.0);

        self.instructions.iter().for_each(
            |i| {
                match i {
                    Instruction::Run { start, stop } => {
                        content.begin_text();
                        content.next_line(curr_position.x, curr_position.y);
                        let s = &self.glyph_ids[*start..*stop].iter().map(|g| g.to_be_bytes()).flatten().collect::<Vec<u8>>();
                        content.show(Str(s));
                        content.end_text();
                    },
                    Instruction::MoveTo { x, y } => {
                        curr_position = Position::new(*x, *y)
                    }
                }
            }
        );
        content.end_text();
    }

    fn get_resources(&self) -> RequiredResources {
        let mut out = RequiredResources::new();
        out.fonts.insert(self.font, HashSet::from_iter(self.glyph_ids.iter().copied()));
        out
    }
}

impl QRCodeElement {
    fn get_resources(&self) -> RequiredResources {
        RequiredResources::new()
    }

    fn write_to_content(&self, content: &mut Content, _context: &Context, _local_context: &LocalContext) {
        content.save_state();
        content.set_fill_color(vec![0.0, 0.0, 0.0]);

        let cell_size = self.size / self.data.len() as f32;
        for (row_idx, row) in self.data.iter().enumerate() {
            for (col_idx, col) in row.iter().enumerate() {
                if *col {
                    content.rect(
                        self.pos.x + col_idx as f32 * cell_size,
                        self.pos.y + row_idx as f32 * cell_size,
                        cell_size,
                        cell_size
                    );
                }
            }
        }
        content.fill_even_odd();
        content.restore_state();
    }
}

#[derive(Debug)]
struct RequiredResources {
    fonts: HashMap<FontRef, HashSet<u16>>,
    graphics: HashSet<GraphicsRef>
}

impl RequiredResources {
    fn new() -> Self {
        RequiredResources { fonts: HashMap::new(), graphics: HashSet::new() }
    }

    fn merge(&mut self, other: &Self) {
        for (key, val) in self.fonts.iter_mut() {
            val.extend(other.fonts.get(key).unwrap_or(&HashSet::new()));
        }
        for (key, other_val) in other.fonts.iter() {
            let val = self.fonts.entry(*key).or_insert_with(|| HashSet::new());
            val.extend(other_val);
        }
        self.graphics.extend(&other.graphics);
    }

    fn into_local_context(self) -> LocalContext {
        let mut context = LocalContext::new();

        for font in self.fonts.keys() {
            context.register_font(*font);
        }

        context
    }
}


impl LayoutedPage {
    fn write_to_content(&self, content: &mut Content, context: &Context, local_context: &LocalContext) {
        for element in self.elements.iter() {
            element.write_to_content(content, context, local_context);
        }
    }

    fn get_resources(&self) -> RequiredResources {
        let mut out = RequiredResources::new();
        for r in self.elements.iter().map(|e| e.get_resources()) {
            out.merge(&r);
        }

        out
    }
}

impl LayoutedElement {
    fn write_to_content(&self, content: &mut Content, context: &Context, local_context: &LocalContext) {
        match self {
            LayoutedElement::Text(e) => e.write_to_content(content, context, local_context),
            LayoutedElement::QRCode(e) => e.write_to_content(content, context, local_context),
            LayoutedElement::Image(_) => todo!(),
            LayoutedElement::Group(_) => todo!(),
        }
    }

    fn get_resources(&self) -> RequiredResources {
        match self {
            LayoutedElement::Text(e) => e.get_resources(),
            LayoutedElement::QRCode(e) => e.get_resources(),
            LayoutedElement::Image(_) => todo!(),
            LayoutedElement::Group(_) => todo!(),
        }
    }
}


impl LayoutedDocument {
    pub fn write_as_pdf(&self) -> Result<Vec<u8>, anyhow::Error> {
        let mut writer = PdfWriter::new();

        let mut context = Context::new();

        for idx in 0..self.fonts.len() {
            let next_ref = context.next_ref();
            context.font_refs.insert(
                FontRef::new(idx),
                next_ref
            );
        }

        let catalog_id = context.next_ref();
        let page_tree_id = context.next_ref();
        let page_ids = self.pages.iter().map(|_| context.next_ref()).collect_vec();

        let meta_data_id = context.next_ref();

        writer.catalog(catalog_id).pages(page_tree_id).metadata(meta_data_id);
        writer.pages(page_tree_id).kids(page_ids.clone()).count(self.pages.len() as i32);

        let meta = "
        <?xpacket begin=\"?\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>
   <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"Adobe XMP Core 5.4-c002 1.000000, 0000/00/00-00:00:00        \">
           <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">
            <rdf:Description rdf:about=\"\"
                  xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">
               <xmp:CreatorTool>OpenTab</xmp:CreatorTool>
            </rdf:Description>
      </rdf:RDF>
</x:xmpmeta>";
        writer.metadata(meta_data_id, meta.as_bytes());

        let mut global_required_resources = RequiredResources::new();

        for (page, page_id) in self.pages.iter().zip(page_ids) {
            let content_id = context.next_ref();

            let required_resources = page.get_resources();
            global_required_resources.merge(&required_resources);
            let local_context = required_resources.into_local_context();

            let mut pdf_page = writer.page(page_id);
            pdf_page.media_box(
                page.dimensions.into()
            ).parent(page_tree_id).contents(content_id);
            let mut resources = pdf_page.resources();
            local_context.write_to_resources(&mut resources, &mut context);
            resources.finish();
            pdf_page.finish();

            let mut content = Content::new();
            page.write_to_content(&mut content, &context, &local_context);

            writer.stream(content_id, &content.finish());
        }

        for (font_id, used_glyphs) in global_required_resources.fonts.iter() {
            let font_ref = *context.font_refs.get(font_id).expect("Missing font");
            let font = &self.fonts[font_id.0];
            let loaded_font = font.as_swash();

            let mut allsorts_font = font.as_allsorts();

            let table = allsorts_font.head_table().unwrap().unwrap();

            let glyph_widths = used_glyphs.iter().map(|g|
                (
                    *g,
                    //loaded_font.glyph_metrics(&[]).advance_width(*g) * scale
                    allsorts_font.horizontal_advance(*g).unwrap() as f32 / table.units_per_em as f32 * 1000.0
                    //1.0
                )
            ).collect::<HashMap<_, _>>();

            let font_desc_id = context.next_ref();
            let font_id_2 = context.next_ref();
            let font_file_id = context.next_ref();

            let base_name = font.name.clone();

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
            .font_descriptor(font_desc_id);
        
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
            .bbox(Rect { x1: table.x_min as f32, y1: table.y_min as f32, x2: table.x_max as f32, y2: table.y_max  as f32 })
            .italic_angle(0.0)
            .stem_v(1.)
            .name(Name(base_name.as_bytes()))
            .finish();

            writer.stream(font_file_id, &font.data.as_slice());
        }

        Ok(writer.finish())
    }
}
