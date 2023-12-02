use allsorts::{tag::LATN, font::MatchingPresentation};
use open_tab_reports::layout::{LayoutedDocument, font::FontLoader, LayoutedPage, PageDimensions, Instruction};



fn main() {
    let loader = FontLoader::new();
    let menlo = loader.load_from_postscript_name("Menlo".to_string()).unwrap();
    

    let mut doc = LayoutedDocument::new();
    let mut allsorts_font = menlo.as_allsorts();

    let glyphs = allsorts_font.map_glyphs("Shaping in a jiffy.", LATN, MatchingPresentation::NotRequired);
    let glyphs = glyphs.into_iter().map(|g| g.glyph_index).collect::<Vec<_>>();

    drop(allsorts_font);

    let font_ref = doc.add_font(menlo);
    let n_glyphs = glyphs.len();

    let mut page: LayoutedPage = LayoutedPage::new(PageDimensions::a4());
    page.add_element(open_tab_reports::layout::LayoutedElement::Text(open_tab_reports::layout::TextElement {
        font: font_ref,
        font_size: 12.0,
        glyph_ids: glyphs,
        instructions: vec![
            Instruction::MoveTo { x: 0.0, y: 10.0 },
            Instruction::Run { start: 0, stop: n_glyphs },
        ],
    }));
    doc.add_page(page);

    let data = doc.write_as_pdf().unwrap();
    //Write to test.pdf
    std::fs::write("test-new.pdf", data).unwrap();
}