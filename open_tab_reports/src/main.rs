use std::{sync::Arc, borrow::BorrowMut, collections::HashMap, hash::Hash, fmt::Debug};

use ab_glyph::Font;
use allsorts::{tables::Fixed, layout};
use encoding_rs::WINDOWS_1252;
use font_kit::{source::SystemSource, font};
use itertools::{izip, Itertools};
use open_tab_reports::{layout::{FontCollection, DynamicTextBox, LayoutDirection, LayoutContext, XObjectLayout, XObjectRef, LayoutedElements, Image, GraphicsCollection, LayoutedDocument, XObjectForm, FixedImage, ImageName, FormName, SinglePageTemplate, LayoutValue, DocumentTemplate}, pdf::PDFWritingContext};
use pdf_writer::{Ref, Rect, Name, Finish, Content, Str, TextStr, Filter, PdfWriter, types::{SystemInfo, FontFlags}, writers::Resources};
use subsetter::{Profile, subset};
use svg2pdf::Options;
use swash::{FontRef, shape::{ShapeContext, Shaper, Direction}, text::{Script, analyze, cluster::{Parser, CharInfo, Token, CharCluster}}, CacheKey};
use tera::Tera;
use usvg::{TreeParsing, TreeTextToPath};

use image::{ColorType, GenericImageView, ImageFormat, DynamicImage, EncodableLayout};
use miniz_oxide::deflate::{compress_to_vec_zlib, CompressionLevel};

use std::collections::HashSet;



fn main() -> std::io::Result<()> {
    let motion = "Lorem ipsum dolor sit amet?";  

    let font_size = 11.0;  

    let mut swash_context: ShapeContext = ShapeContext::new();
    let mut font_collection = FontCollection::new();
    let mut graphic: GraphicsCollection = GraphicsCollection::new();
    let mut layout_context = LayoutContext::new(&mut swash_context, &mut font_collection, &mut graphic);

    let template_value_dicts = LayoutValue::Vec(vec![
        LayoutValue::Dict(HashMap::from_iter(
            vec![
                ("gov.members.0".into(), LayoutValue::String("John Smith".into())),
                ("gov.members.1".into(), LayoutValue::String("Jane Doe".into())),
                ("gov.members.2".into(), LayoutValue::String("Robert Johnson".into())),
                ("gov.name".into(), LayoutValue::String("Government".into())),
                ("opp.members.0".into(), LayoutValue::String("Alice Johnson".into())),
                ("opp.members.1".into(), LayoutValue::String("Michael Brownington the first of his weirdly long name".into())),
                ("opp.members.2".into(), LayoutValue::String("Sophia Lee".into())),
                ("opp.name".into(), LayoutValue::String("Opposition".into())),
                ("non_aligned.members.0".into(), LayoutValue::String("Emily Davis".into())),
                ("non_aligned.members.1".into(), LayoutValue::String("Daniel Wilson".into())),
                ("non_aligned.members.2".into(), LayoutValue::String("Olivia White".into())),
                ("adj.0".into(), LayoutValue::String("Alice Johnson".into())),
                ("adj.1".into(), LayoutValue::String("Michael Brown".into())),
                ("adj.2".into(), LayoutValue::String("Sophia Lee".into())),
                ("adj.3".into(), LayoutValue::String("Daniel Wilson".into())),
                ("adj.4".into(), LayoutValue::String("Olivia White".into()))
            ]
        )),
        LayoutValue::Dict(HashMap::from_iter(
            vec![
                ("gov.members.0".into(), LayoutValue::String("John Smith".into())),
                ("gov.members.1".into(), LayoutValue::String("Jane Doe".into())),
                ("gov.members.2".into(), LayoutValue::String("Robert Johnson".into())),
                ("gov.name".into(), LayoutValue::String("Government".into())),
                ("opp.members.0".into(), LayoutValue::String("Alice Johnson".into())),
                ("opp.members.1".into(), LayoutValue::String("Michael Brownington the first of his weirdly long name".into())),
                ("opp.members.2".into(), LayoutValue::String("Sophia Lee".into())),
                ("opp.name".into(), LayoutValue::String("Opposition".into())),
                ("non_aligned.members.0".into(), LayoutValue::String("XXX Davis".into())),
                ("non_aligned.members.1".into(), LayoutValue::String("Daniel Wilson".into())),
                ("non_aligned.members.2".into(), LayoutValue::String("Olivia White".into())),
                ("adj.0".into(), LayoutValue::String("Alice Johnson".into())),
                ("adj.1".into(), LayoutValue::String("Michael Brown".into())),
                ("adj.2".into(), LayoutValue::String("Sophia Lee".into())),
                ("adj.3".into(), LayoutValue::String("Daniel Wilson".into())),
                ("adj.4".into(), LayoutValue::String("Olivia White".into()))
            ]
        )),

    ]);

    let values = LayoutValue::Dict(
        HashMap::from_iter(
            vec![(
                "background".into(), 
                LayoutValue::Dict(
                    HashMap::from_iter(
                        vec![
                            ("motion".into(), LayoutValue::String(motion.into()))
                        ]
                    )
                )
            )]
        )
    );

    let doc_values = LayoutValue::Dict(
        HashMap::from_iter(
            vec![
                ("forms".into(), values),
                ("pages".into(), LayoutValue::Vec(vec![template_value_dicts]))
            ]
        )
    );

    let mut context = PDFWritingContext::new();
    let mut writer = PdfWriter::new();

    let document_template = serde_json::from_str::<DocumentTemplate>(std::fs::read_to_string("template.json")?.as_str())?;

    let doc = document_template.layout(&mut layout_context, &doc_values);
    
    doc.write_to_pdf(&mut writer, &mut context);

    let mut fonts = doc.get_fonts_and_glyphs();

    graphic.get_fonts_and_glyphs().into_iter().for_each(|(f, g)| {
        fonts.entry(f).or_insert_with(|| HashSet::new()).extend(g);
    });

    graphic.write_to_pdf(&mut writer, &mut context);
    font_collection.write_fonts_to_pdf(&mut writer, &mut context, fonts.into_iter().collect_vec());

    std::fs::write("image.pdf", writer.finish());


    let template_value_dicts = LayoutValue::Dict(HashMap::from_iter(
            vec![
                (
                    "team_tab".into(), LayoutValue::Vec(
                        (0..120).map(|i| 
                            LayoutValue::Dict(
                                HashMap::from_iter(vec![("name".into(), LayoutValue::String(format!("Entry {}", i).into()))].into_iter())
                            )).collect_vec()
                    )
                )
            ]
        ));
    

    let doc_values = LayoutValue::Dict(
        HashMap::from_iter(
            vec![
                ("pages".into(), LayoutValue::Vec(vec![template_value_dicts]))
            ]
        )
    );

    let mut context = PDFWritingContext::new();
    let mut writer = PdfWriter::new();
    let mut graphic: GraphicsCollection = GraphicsCollection::new();
    let mut layout_context = LayoutContext::new(&mut swash_context, &mut font_collection, &mut graphic);

    let document_template = serde_json::from_str::<DocumentTemplate>(std::fs::read_to_string("template2.json")?.as_str())?;

    let doc = document_template.layout(&mut layout_context, &doc_values);
    
    doc.write_to_pdf(&mut writer, &mut context);

    let mut fonts = doc.get_fonts_and_glyphs();

    graphic.get_fonts_and_glyphs().into_iter().for_each(|(f, g)| {
        fonts.entry(f).or_insert_with(|| HashSet::new()).extend(g);
    });

    graphic.write_to_pdf(&mut writer, &mut context);
    font_collection.write_fonts_to_pdf(&mut writer, &mut context, fonts.into_iter().collect_vec());


    std::fs::write("image2.pdf", writer.finish());

    Ok(())
}
