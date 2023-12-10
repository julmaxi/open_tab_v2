use std::{collections::HashMap, rc::{Weak, Rc}, cell::RefCell};

use allsorts::{tag::LATN, gsub::FeatureMask, font::MatchingPresentation};
use image::GenericImageView;
use itertools::Itertools;
use pdf_writer::writers::Page;
use unicode_linebreak::linebreaks;
use unicode_segmentation::UnicodeSegmentation;
use usvg::Text;

use super::{LayoutedElement, LayoutedDocument, font::{Font, FontLoader}, FontRef, LayoutedPage, PageDimensions, TextElement, Instruction};


struct Container {

}

type Result<T> = std::result::Result<T, anyhow::Error>;


enum LayoutDirective {
    Default,
    Absolute{x: f32, y: f32},
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32
}

#[derive(Debug, Clone, Copy)]
enum ContentGenerationOutcome {
    Done,
    Overflow,
    PageBreak,
}

type PageId = usize;

pub struct ContentGenerationResult {
    elements: Vec<(PageId, LayoutedElement)>,
    used_rect: Rect,
    outcome: ContentGenerationOutcome,
}

pub struct ResourceLoader {
    fonts: HashMap<String, FontRef>,
    font_data: Vec<Font>,
    loader: FontLoader
 //   images: HashMap<String, Weak<Image>>,
}

impl ResourceLoader {
    fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            font_data: vec![],
            loader: FontLoader::new()
        }
    }

    fn get_font_ref(&mut self, name: &String) -> Result<FontRef> {
        if let Some(id) = self.fonts.get(name) {
            return Ok(*id);
        }
        else {
            let font = self.loader.load_from_postscript_name(name.clone())?;
            let font_ref = FontRef(self.font_data.len());
            self.font_data.push(font);
            Ok(font_ref)
        }
    }

    fn get_font(&self, font_ref: FontRef) -> &Font {
        &self.font_data[font_ref.0]
    }

    fn get_font_by_name(&mut self, name: &str) -> Result<&Font> {
        let font_ref = self.get_font_ref(&name.to_string())?;
        Ok(self.get_font(font_ref))
    }
}

#[derive(Debug, Clone, Copy)]
struct PageRect {
    page_id: usize,
    rect: Rect,
}

pub trait Layouter {
    fn next_rect(&mut self) -> Option<PageRect>;
}

pub trait ContentGenerator {
    fn next_elements(&self, resources: &mut ResourceLoader, layouter: &mut Box<dyn Layouter>) -> Result<ContentGenerationResult>;
}

struct PageLayouter {
    dimensions: PageDimensions,
    margin_left: f32,
    margin_right: f32,
    margin_top: f32,
    margin_bottom: f32,
    next_page_id: usize
}

impl Layouter for PageLayouter {
    fn next_rect(&mut self) -> Option<PageRect> {
        let page_id = self.next_page_id;
        self.next_page_id += 1;
        Some(PageRect {
            page_id,
            rect: Rect { x: self.margin_left, y: self.margin_right, width: self.dimensions.width - self.margin_right - self.margin_left, height: self.dimensions.height - self.margin_bottom - self.margin_top }
        })
    }
}

struct ColumnLayouter {
    num_columns: u32,
    curr_column: u32,
    parent: Weak<RefCell<Box<dyn Layouter>>>,
    curr_rect: Option<PageRect>
}

impl Layouter for ColumnLayouter {
    fn next_rect(&mut self) -> Option<PageRect> {
        if self.curr_column == self.num_columns {
            self.curr_rect = None;
            self.curr_column = 0;
        }

        if let None = self.curr_rect {
            let parent = self.parent.upgrade().expect("Parent dropped");
            let mut parent = parent.as_ref().borrow_mut();
            let page_rect = parent.next_rect();
            if let Some(page_rect) = page_rect {
                self.curr_rect = Some(page_rect);
            }
            else {
                return None;
            }
        }

        let mut rect = self.curr_rect.as_ref().unwrap().rect.clone();

        let column_width = rect.width / self.num_columns as f32;
        rect.x += column_width * self.curr_column as f32;
        rect.width = column_width;

        self.curr_column += 1;

        Some(PageRect {
            page_id: self.curr_rect.as_ref().unwrap().page_id,
            rect
        })
    }

}


pub struct DocumentLayouter {
    pub content: Vec<Box<dyn ContentGenerator>>,
    templates: HashMap<String, Container>
}

impl DocumentLayouter {
    pub fn new() -> Self {
        Self {
            content: vec![],
            templates: HashMap::new()
        }
    }

    pub fn add_element(&mut self, content: Box<dyn ContentGenerator>) {
        self.content.push(content);
    }

    pub fn add_template(&mut self, name: String, template: Container) {
        self.templates.insert(name, template);
    }

    pub fn layout(self) -> Result<LayoutedDocument> {
        let mut doc = LayoutedDocument::new();
        let mut resources = ResourceLoader::new();

        let root_layouter : Rc<RefCell<Box<dyn Layouter>>> = Rc::new(RefCell::new(Box::new(PageLayouter {
            dimensions: PageDimensions::a4(),
            next_page_id: 0,
            margin_left: 20.0,
            margin_right: 20.0,
            margin_top: 20.0,
            margin_bottom: 20.0,
        })));

        let mut page_elements = HashMap::new();

        let mut body_layouter : Box<dyn Layouter> = Box::new(ColumnLayouter {
            num_columns: 1,
            curr_column: 0,
            parent: Rc::downgrade(&root_layouter),
            curr_rect: None
        });

        for mut elem in self.content {
            let result = elem.next_elements(&mut resources, &mut body_layouter)?;

            for elem in result.elements {
                page_elements.entry(elem.0).or_insert_with(|| vec![]).push(elem.1);
            }
        }

        let max_page = page_elements.keys().max().unwrap_or(&0);

        for page_idx in 0..=*max_page {
            let page = LayoutedPage { dimensions: PageDimensions::a4(), elements: page_elements.remove(&page_idx).unwrap_or_default() };
            doc.add_page(page);
        }

        doc.fonts = resources.font_data;

        Ok(doc)
    }
}

pub struct TextLayouter {
    pub text: String,
    pub font: String,
    pub font_size: f32,
}

impl ContentGenerator for TextLayouter {
    fn next_elements(&self, resources: &mut ResourceLoader, layouter: &mut Box<dyn Layouter>) -> Result<ContentGenerationResult> {
        let font_ref = resources.get_font_ref(&self.font)?;
        let font = resources.get_font_by_name(&self.font)?;
        let mut allsorts_font = font.as_allsorts();
        let words = self.text.split_word_bounds().collect::<Vec<&str>>();
        //let glyphs = allsorts_font.map_glyphs(&self.text, LATN, MatchingPresentation::NotRequired);

        let mut x_cursor = 0.0;
        let line_height = self.font_size;
        let mut curr_rect = layouter.next_rect();
        if curr_rect.is_none() {
            return Ok(ContentGenerationResult {
                elements: vec![],
                used_rect: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
                outcome: ContentGenerationOutcome::Overflow
            });
        }
        let mut curr_rect = curr_rect.unwrap();

        let mut y_cursor = curr_rect.rect.y + curr_rect.rect.height - line_height;

        let units_per_em = allsorts_font.head_table().unwrap().unwrap().units_per_em as f32;
        let mut all_glyphs = vec![];
        let mut line_starts = vec![(0, 0, y_cursor)];

        let mut line_width = 0.0;

        let mut out_elements = vec![];

        for word in words {
            let glyphs = allsorts_font.map_glyphs(word, LATN, MatchingPresentation::NotRequired);
            let word_width : f32 = glyphs.iter().filter_map(
                |g| {
                    allsorts_font.horizontal_advance(g.glyph_index).map(|a| a as f32 / units_per_em)
                }
            ).sum::<f32>() * self.font_size;

            if line_width + word_width > curr_rect.rect.width || word == "\n" {
                y_cursor -= line_height;

                if word != " " && word != "\n" {
                    line_width = word_width;     
                }
                else {
                    line_width = 0.0;
                }
                line_starts.last_mut().unwrap().1 = all_glyphs.len();               

                if y_cursor < curr_rect.rect.y {
                    let instructions = line_starts.into_iter().flat_map(|(start, stop, pos)| {
                        vec![
                            Instruction::MoveTo { x: curr_rect.rect.x, y: pos },
                            Instruction::Run { start, stop }
                        ]
                    }).collect_vec();

                    out_elements.push(
                        (curr_rect.page_id, LayoutedElement::Text(TextElement {
                            font: font_ref,
                            font_size: self.font_size,
                            glyph_ids: all_glyphs.clone(),
                            instructions
                        }))
                    );

                    let next_rect = layouter.next_rect();
                    if next_rect.is_none() {
                        return Ok(ContentGenerationResult {
                            elements: out_elements,
                            used_rect: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
                            outcome: ContentGenerationOutcome::Overflow
                        });
                    }

                    y_cursor = curr_rect.rect.y + curr_rect.rect.height - line_height;

                    line_starts = vec![(0, 0, y_cursor)];            
                    all_glyphs.clear();
                    if word != " " && word != "\n" {
                        all_glyphs.extend(glyphs.into_iter().map(|g| g.glyph_index));
                    }
                }
                else {
                    line_starts.last_mut().unwrap().1 = all_glyphs.len();
                    line_starts.push((all_glyphs.len(), 0, y_cursor));

                    if word != " " && word != "\n" {
                        all_glyphs.extend(glyphs.into_iter().map(|g| g.glyph_index));
                    }

                }
            }
            else {
                line_width += word_width;
                all_glyphs.extend(glyphs.into_iter().map(|g| g.glyph_index));
            }
        }
        
        line_starts.last_mut().unwrap().1 = all_glyphs.len();
        let instructions = line_starts.into_iter().flat_map(|(start, stop, pos)| {
            vec![
                Instruction::MoveTo { x: curr_rect.rect.x, y: pos },
                Instruction::Run { start, stop }
            ]
        }).collect_vec();

        out_elements.push(
            (curr_rect.page_id, LayoutedElement::Text(TextElement {
                font: font_ref,
                font_size: self.font_size,
                glyph_ids: all_glyphs.clone(),
                instructions
            }))
        );

        let used_rect = Rect { x: curr_rect.rect.x, y: y_cursor, width: curr_rect.rect.width, height: curr_rect.rect.y - y_cursor };

        return Ok(ContentGenerationResult {
            elements: out_elements,
            used_rect,
            outcome: if used_rect.y < curr_rect.rect.y  { ContentGenerationOutcome::Overflow } else {ContentGenerationOutcome::Done}
        });
    }
}


pub struct QRCodeLayouter {
    pub content: String,
    pub size: f32,
}

impl QRCodeLayouter {
    fn content_as_matrix(&self) -> Result<Vec<Vec<bool>>> {
        Ok(qrcode_generator::to_matrix(&self.content, qrcode_generator::QrCodeEcc::Low)?)
    }
}

impl ContentGenerator for QRCodeLayouter {
    fn next_elements(&self, resources: &mut ResourceLoader, layouter: &mut Box<dyn Layouter>) -> Result<ContentGenerationResult> {
        let rect = layouter.next_rect();
        if rect.is_none() {
            return Ok(ContentGenerationResult {
                elements: vec![],
                used_rect: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
                outcome: ContentGenerationOutcome::Overflow
            });
        }
        let rect = rect.unwrap();

        //let center = (rect.rect.x + rect.rect.width / 2.0, rect.rect.y + rect.rect.height / 2.0);
        //let x_pos = center.0 - self.size / 2.0;
        //let y_pos = center.1 - self.size / 2.0;

        let x_pos = rect.rect.x;
        let y_pos = rect.rect.height + rect.rect.y - self.size;

        return Ok(ContentGenerationResult {
            elements: vec![
                (rect.page_id, LayoutedElement::QRCode(
                    super::QRCodeElement { pos: super::Position::new(x_pos, y_pos), data: self.content_as_matrix()?, size: self.size }
                ))
            ],
            used_rect: Rect { x: x_pos, y: y_pos, width: self.size, height: self.size },
            outcome: if rect.rect.width >= self.size && rect.rect.height >= self.size { ContentGenerationOutcome::Done } else { ContentGenerationOutcome::Overflow }
        })
    }
}


#[derive(Debug, Clone)]
pub enum CellWidth {
    Fixed(f32),
    Dynamic
}

pub struct CellInfo {
    pub content: Box<dyn ContentGenerator >,
    pub width: CellWidth
}

pub struct RowInfo {
    pub cells: Vec<CellInfo>,
}


pub struct TabularLayouter {
    pub rows: Vec<RowInfo>,
    pub row_margin: f32,
}

pub struct FixedRectLayouter {
    rect: PageRect,
    has_fired: bool
}

impl FixedRectLayouter {
    pub fn new(rect: PageRect) -> Self {
        Self {
            rect,
            has_fired: false
        }
    }
}

impl Layouter for FixedRectLayouter {
    fn next_rect(&mut self) -> Option<PageRect> {
        if self.has_fired {
            return None;
        }
        self.has_fired = true;
        Some(self.rect)
    }
}

struct RowLayoutResult {
    elements: Vec<(PageId, LayoutedElement)>,
    remaining_rect: Rect,
}

impl TabularLayouter {
    fn layout_row(&self, row: &RowInfo, mut remaining_rect: Rect, page_id: PageId, break_on_overflow: bool, resources: &mut ResourceLoader) -> Result<Option<RowLayoutResult>> {
        let mut out_elements = vec![];

        let fixed_cell_width_sum = row.cells.iter().filter_map(|c| {
            match c.width {
                CellWidth::Fixed(w) => Some(w),
                CellWidth::Dynamic => None
            }
        }).sum::<f32>();
        
        let dynamic_cell_count = row.cells.iter().filter(|c| {
            match c.width {
                CellWidth::Fixed(_) => false,
                CellWidth::Dynamic => true
            }
        }).count();

        let dynamic_cell_width = ((remaining_rect.width - fixed_cell_width_sum) / dynamic_cell_count as f32).max(0.0);

        let mut min_y = remaining_rect.y + remaining_rect.height;
        let mut offset = 0.0;

        for cell in row.cells.iter() {
            let width = match cell.width {
                CellWidth::Fixed(w) => w,
                CellWidth::Dynamic => dynamic_cell_width
            };

            let mut layouter : Box<dyn Layouter> = Box::new(FixedRectLayouter::new(PageRect {
                page_id: page_id,
                rect: Rect {
                    x: remaining_rect.x + offset,
                    y: remaining_rect.y,
                    width,
                    height: remaining_rect.height
                }
            }));

            let result = cell.content.next_elements(resources, &mut layouter)?;
            match result.outcome {
                ContentGenerationOutcome::Overflow => {
                    if break_on_overflow {
                        return Ok(None);
                    }
                },
                _ => {},
            }
            out_elements.extend(result.elements);

            min_y = f32::min(min_y, result.used_rect.y);

            offset += width;
        }

        remaining_rect.height = min_y - remaining_rect.y - self.row_margin;

        Ok(Some(RowLayoutResult {
            elements: out_elements,
            remaining_rect
        }))
    }
}

impl ContentGenerator for TabularLayouter {
    fn next_elements(&self, resources: &mut ResourceLoader, layouter: &mut Box<dyn Layouter>) -> Result<ContentGenerationResult> {
        let rect = layouter.next_rect();
        if rect.is_none() {
            return Ok(ContentGenerationResult {
                elements: vec![],
                used_rect: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
                outcome: ContentGenerationOutcome::Overflow
            });
        }

        let mut rect = rect.unwrap();

        let mut out_elements = vec![];

        let mut remaining_rect = rect.rect.clone();

        for row in self.rows.iter() {
            let result = self.layout_row(row, remaining_rect, rect.page_id, true, resources)?;

            let result = if result.is_none() {
                let next_rect = layouter.next_rect();
                if next_rect.is_none() {
                    return Ok(ContentGenerationResult {
                        elements: out_elements,
                        used_rect: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
                        outcome: ContentGenerationOutcome::Overflow
                    });
                }
                rect = next_rect.unwrap();

                remaining_rect = rect.rect.clone();

                let result = self.layout_row(row, remaining_rect, rect.page_id, false, resources)?;
                result.unwrap()
            } else {
                result.unwrap()
            };

            out_elements.extend(result.elements);
            remaining_rect = result.remaining_rect;
        }

        Ok(
            ContentGenerationResult {
                elements: out_elements,
                used_rect: rect.rect,
                outcome: ContentGenerationOutcome::Done
            }
        )
    }
}