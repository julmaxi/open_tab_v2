use std::collections::{HashSet, HashMap};

use pdf_writer::{Content, PdfWriter, Ref};

use crate::layout::{XObjectRef, FontUseRef};

use super::{ResourceDict, PDFWritingContext};

pub trait ContentWriteable {
    fn write_to_pdf(&self, content: &mut Content, resources: &ResourceDict);
    fn get_xobjects(&self) -> Vec<&XObjectRef>;
    fn get_fonts_and_glyphs(&self) -> Vec<(&FontUseRef, HashSet<u16>)>;
}

pub trait XObjectRenderable {
    fn render_as_xobject(&self, pdf_writer: &mut PdfWriter, context: &mut PDFWritingContext, id: Ref);
    fn get_fonts_and_glyphs(&self) -> HashMap<FontUseRef, HashSet<u16>>;
}