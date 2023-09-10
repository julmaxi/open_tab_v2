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

use image::{ColorType, GenericImageView, ImageFormat, DynamicImage, EncodableLayout};
use miniz_oxide::deflate::{compress_to_vec_zlib, CompressionLevel};

use std::collections::HashSet;

use crate::pdf::writable::{ContentWriteable, XObjectRenderable};

use super::{FontCollection, FontUseRef, context::LayoutContext, layout_trait::Layoutable};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicTextBox {
    pub width: f32,
    pub max_height: Option<f32>,
    pub default_font_size: f32,
    pub x: f32,
    pub y: f32,
    pub layout_direction: LayoutDirection,
    pub font: String
}

impl DynamicTextBox {
    fn run_layout_trial(&self,
        context: &mut LayoutContext,
        clusters: &Vec<CharCluster>,
        font_size: f32
    ) -> TextLayoutResult {
        let font_ref = context.font_collection.get_font_ref_from_postscript_name(&self.font);
        let mut shaper = context.shape_context.builder(font_ref)
        .script(Script::Latin)
        .size(font_size)
        .build();

        for cluster in clusters.iter() {
            shaper.add_cluster(cluster);
        }

        let mut glyph_ids = vec![];
        let mut instructions = vec![];
        let mut curr_advance = 0.0;
        let mut last_break_opportunity = 0;
        let mut cursor = 0;
        let mut n_lines = 1;
        let mut total_width = 0.0;

        instructions.push(Instruction::MoveTo { x: 0.0, y: 0.0 });

        let mut y_advance = 0.0;
    
        shaper.shape_with(|cluster| {
            let new_advance = cluster.glyphs.iter().map(|g| g.advance).sum::<f32>();
            // This is not exactly correct, since we ignore whitespaces at the beginning
            total_width += new_advance;
            let cluster_start = glyph_ids.len();
            glyph_ids.extend(cluster.glyphs.iter().map(|g| g.id).collect_vec());
    
            if curr_advance + new_advance > self.width  {
                if last_break_opportunity > cursor {
                    instructions.push(Instruction::Run { start: cursor, stop: last_break_opportunity});            
                    curr_advance = new_advance;
                    // Ignore the space at the beginning of the text
                    cursor = last_break_opportunity + 1;
                }
                else {
                    instructions.push(Instruction::Run { start: cursor, stop: glyph_ids.len()});
                    curr_advance = 0.0;
                    cursor = glyph_ids.len();
                }
                n_lines += 1;
                y_advance -= font_size;
                instructions.push(Instruction::MoveTo { x: 0.0, y: y_advance });
            }
            else {
                curr_advance += new_advance;
            }
            
            if cluster.info.is_boundary() && cluster.info.is_whitespace() {
                last_break_opportunity = cluster_start;
            }
        });
    
        if cursor < glyph_ids.len() {
            instructions.push(Instruction::Run { start: cursor, stop: glyph_ids.len()});
        }

        let instructions = instructions.into_iter().map(|i| match i {
            Instruction::MoveTo { x, y } => {
                match self.layout_direction {
                    LayoutDirection::TopToBottom => Instruction::MoveTo { x: x + self.x, y: y + self.y },
                    LayoutDirection::BottomToTop => Instruction::MoveTo { x: x + self.x, y: y + self.y - y_advance},
                }
            }
            i => i
        }).collect_vec();

        let font_id = context.font_collection.get_id_from_postscript_name(&self.font).unwrap();

        TextLayoutResult {
            instructions,
            height: -y_advance,
            n_lines,
            glyph_ids,
            total_width,
            font_size,
            font: font_id
        }    
    }

    pub fn layout_text(
        &self,
        context: &mut LayoutContext,
        text: &str,
    ) -> TextLayoutResult {
        let mut parser = Parser::new(
            Script::Latin,
            text.char_indices()
                // Call analyze passing the same text and zip
                // the results
                .zip(analyze(text.chars()))
                // Analyze yields the tuple (Properties, Boundary)
                .map(|((i, ch), (props, boundary))|
                
                {
                    Token {
                    ch,
                    offset: i as u32,
                    len: ch.len_utf8() as u8,
                    // Create character information from properties and boundary
                    info: CharInfo::new(props, boundary),
                    data: 0
                }}),
        );
        let mut cluster = CharCluster::new();
        let font = context.font_collection.get_font_ref_from_postscript_name(&self.font);
        let charmap = font.charmap();
        let mut parsed_text = vec![];
    
        while parser.next(&mut cluster) {
            cluster.map(|ch| charmap.map(ch));
            parsed_text.push(cluster);
        }

        let result = self.run_layout_trial(context, &parsed_text, self.default_font_size);

        if let Some(max_height) = self.max_height {
            if result.height > max_height {
                //let new_font_size = self.default_font_size * max_height / result.height;
                let factor = ((self.width * max_height) / (self.default_font_size * result.total_width)).sqrt();
                return self.run_layout_trial(context, &parsed_text, (self.default_font_size * factor).floor());
            }
            else {
                result
            }
        }
        else {
            result
        }    
    }
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LayoutDirection {
    TopToBottom,
    BottomToTop,
}

#[derive(Debug)]
pub enum Instruction {
    Run{start: usize, stop: usize},
    MoveTo{ x: f32, y: f32 },
}


pub struct TextLayoutResult {
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) height: f32,
    pub(crate) n_lines: usize,
    pub(crate) glyph_ids: Vec<u16>,
    pub(crate) total_width: f32,
    pub(crate) font_size: f32,
    pub(crate) font: FontUseRef
}


#[typetag::serde]
impl Layoutable for DynamicTextBox {
    fn layout(&self, context: &mut LayoutContext, value: &super::LayoutValue) -> super::LayoutResult {
        let text = match value {
            super::LayoutValue::String(s) => s,
            _ => panic!("Expected string")
        };

        let result = self.layout_text(context, text);

        return super::LayoutResult {
            bounding_box: super::layout_trait::BoundingBox {
                x: self.x,
                y: self.y,
                width: self.width,
                height: result.height
            },
            objects: vec![
                Box::new(result)
            ]
        };
    }
}