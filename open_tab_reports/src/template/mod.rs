

use std::{collections::HashMap, path::Path};

use serde_json::Value;
use tera::{Context, Tera};
use open_tab_entities::{derived_models::{DrawPresentationInfo, name_to_initials, RegistrationInfo}, tab::{TabView, BreakRelevantTabView}};


use std::io::Write;

use crate::layout::design::{RowInfo, CellInfo, CellWidth, QRCodeLayouter, TextLayouter, DocumentLayouter, TabularLayouter};



#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct AdditionalFilesEntry {
    path: String,
    media_type: String,
}

struct OpenOfficeDocument {
    content: String,
    styles: String,
    additional_files: Vec<AdditionalFilesEntry>,
    additional_files_data: HashMap<String, Vec<u8>>,
    doc_media_type: String,
}

pub struct TemplateContext {
    pub(crate) template_dir: String,
    pub(crate) tera: Tera,
}

fn get_role_letter(val: &str) -> Result<String, anyhow::Error> {
    match val {
        "Government" => Ok("G".into()),
        "Opposition" => Ok("O".into()),
        "NonAligned" => Ok("F".into()),
        _ => Err(anyhow::anyhow!("Invalid role {:?}", val)),
    }
} 


#[allow(dead_code)]
fn role_letter<'a, 'b>(val: &'a Value, _args: &'b HashMap<String, Value>) -> tera::Result<Value> {
    match val {
        Value::String(s) => Ok(Value::String(get_role_letter(s).map_err(
            |e| tera::Error::call_function("role_letter", e)
        )?)),
        _ => Err(tera::Error::call_function("role_letter", anyhow::Error::msg(format!("Invalid role {:?}", val)))),
    }
} 

fn role_letters<'a, 'b>(val: &'a Value, _args: &'b HashMap<String, Value>) -> tera::Result<Value> {
    match val {
        Value::Array(arr) => {
            let mut result = vec![];
            for val in arr {
                if let Value::Object(val) = val {
                        let team_role = val.get("team_role").ok_or(anyhow::anyhow!("Missing team_role")).map_err(|e| tera::Error::call_filter("role_letters", e))?.as_str().ok_or(anyhow::anyhow!("team_role not string")).map_err(|e| tera::Error::call_filter("role_letters", e))?;
                        let role_letter = get_role_letter(team_role).map_err(|e| tera::Error::call_filter("role_letters", e))?;
                        let role_position = val.get("speech_position").ok_or(anyhow::anyhow!("Missing speech_position")).map_err(|e| tera::Error::call_filter("role_letters", e)) ?.as_u64().ok_or(anyhow::anyhow!("speech_position not number")).map_err(|e| tera::Error::call_filter("role_letters", e))?;
                        
                        result.push(Value::String(format!("{}{}", role_letter, role_position + 1)));
                }
            }
            Ok(Value::Array(result))
        },
        _ => Err(tera::Error::call_function("role_letters", anyhow::Error::msg(format!("Invalid argument {:?}. Should be array.", val)))),
    }
}

fn to_2_decimals<'a, 'b>(val: &'a Value, _args: &'b HashMap<String, Value>) -> tera::Result<Value> {
    match val {
        Value::Number(n) => {
            Ok(Value::String(format!("{:.2}", n.as_f64().unwrap_or(0.0))))
        },
        _ => Err(tera::Error::call_function("to_2_decimals", anyhow::Error::msg(format!("Invalid argument {:?}. Should be number.", val)))),
    }
}

fn unwrap_default<'a, 'b>(val: &'a Value, args: &'b HashMap<String, Value>) -> tera::Result<Value> {
    match val {
        Value::Null => {
            match args.get("default") {
                Some(default) => Ok(default.clone()),
                None => Err(tera::Error::call_function("unwrap_default", anyhow::Error::msg(format!("Must set default value.")))),
            }
        },
        _ => Ok(val.clone()),
    }
}


fn get_public_name<'a, 'b>(val: &'a Value, _args: &'b HashMap<String, Value>) -> tera::Result<Value> {
    match val {
        Value::Object(participant) => {
            let is_anonymous = participant.get("is_anonymous").ok_or(anyhow::anyhow!("Missing is_anonymous")).map_err(|e| tera::Error::call_filter("public_name", e))?.as_bool().ok_or(anyhow::anyhow!("is_anonymous not bool")).map_err(|e| tera::Error::call_filter("public_name", e))?;
            let name = participant.get("speaker_name").ok_or(anyhow::anyhow!("Missing name")).map_err(|e| tera::Error::call_filter("public_name", e))?.as_str().ok_or(anyhow::anyhow!("name not string")).map_err(|e| tera::Error::call_filter("public_name", e))?;
            if is_anonymous {
                Ok(Value::String(name_to_initials(name)))
            } else {
                Ok(Value::String(name.into()))
            }
        },
        _ => Ok(val.clone()),
    }
}

impl TemplateContext {
    pub fn new(template_dir: String) -> Result<Self, anyhow::Error> {
        let mut tera = Tera::new(Path::new(&template_dir).join("**/*.xml").as_os_str().to_str().unwrap_or("templates/**/*.xml"))?;
        tera.register_filter("role_letters", role_letters);
        tera.register_filter("to_2_decimals", to_2_decimals);
        tera.register_filter("unwrap_default", unwrap_default);
        tera.register_filter("public_name", get_public_name);
        
        tera.autoescape_on(vec![".html", ".sql", ".xml"]);
        
        Ok(Self {
            tera,
            template_dir
        })
    }
}

impl OpenOfficeDocument {
    pub fn write<W>(&self, context: &TemplateContext, writer: W) -> Result<(), anyhow::Error> where W: Write + std::io::Seek {
        let _additional_files = serde_json::json!({
            "additional_files": [
                {
                    "path": "Pictures/ballot_background.png",
                    "media_type": "image/png"
                }
            ],
            "doc_media_type": "application/vnd.oasis.opendocument.graphics",
        });

        let manifest_data = serde_json::json!({
            "doc_media_type": self.doc_media_type,
            "additional_files": self.additional_files,
        });

        let manifest = context.tera.render("open_office/manifest.xml", &Context::from_serialize(&manifest_data)?)?;
    
        let meta = context.tera.render("open_office/meta.xml", &Context::new())?;
        let settings = context.tera.render("open_office/settings.xml", &Context::new())?;
    
        let mut zip = zip::ZipWriter::new(writer);
        let options = zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("META-INF/manifest.xml", options)?;
        zip.write_all(manifest.as_bytes())?;
        zip.start_file("meta.xml", options)?;
        zip.write_all(meta.as_bytes())?;
        zip.start_file("settings.xml", options)?;
        zip.write_all(settings.as_bytes())?;
        zip.start_file("content.xml", options)?;
        zip.write_all(self.content.as_bytes())?;
        zip.start_file("styles.xml", options)?;
        zip.write_all(self.styles.as_bytes())?;

        for entry in &self.additional_files {
            let data = self.additional_files_data.get(&entry.path).ok_or(anyhow::anyhow!("Additional file {} not found", entry.path))?;
            zip.start_file(&entry.path, options)?;
            zip.write_all(data)?;
        }
    
        // Apply the changes you've made.
        // Dropping the `ZipWriter` will have the same effect, but may silently fail
        zip.finish()?;
        drop(zip);

        Ok(())
    }
}

pub fn make_open_office_ballots<W>(context: &TemplateContext, writer: W, info: &DrawPresentationInfo) ->
    Result<(), anyhow::Error> where W: Write + std::io::Seek {
    let ballot_xml = context.tera.render("open_office/ballot.xml", &Context::from_serialize(&info)?)?;
    let styles_xml = context.tera.render("open_office/ballot_styles.xml", &Context::new())?;
    let image_file = std::fs::read(Path::new(&context.template_dir).join("open_office/ballot_background.png"))?;

    OpenOfficeDocument {
        content: ballot_xml,
        styles: styles_xml,
        additional_files: vec![
            AdditionalFilesEntry {
                path: "Pictures/ballot_background.png".into(),
                media_type: "image/png".into(),
            }
        ],
        additional_files_data: vec![
            ("Pictures/ballot_background.png".into(), image_file)
        ].into_iter().collect(),
        doc_media_type: "application/vnd.oasis.opendocument.graphics".into(),
    }.write(&context, writer)?;

    return Ok(());
}


pub fn make_open_office_presentation<W>(context: &TemplateContext, writer: W, info: &DrawPresentationInfo) ->
    Result<(), anyhow::Error> where W: Write + std::io::Seek {
    let presentation_xml = context.tera.render("open_office/presentation.xml", &Context::from_serialize(&info)?)?;
    let styles_xml = context.tera.render("open_office/presentation_styles.xml", &Context::new())?;

    OpenOfficeDocument {
        content: presentation_xml,
        styles: styles_xml,
        additional_files: vec![
        ],
        additional_files_data: vec![
        ].into_iter().collect(),
        doc_media_type: "application/vnd.oasis.opendocument.presentation".into(),
    }.write(&context, writer)?;

    return Ok(());
}

#[derive(Debug)]
pub enum OptionallyBreakRelevantTab {
    Tab(TabView),
    BreakRelevantTab(BreakRelevantTabView)
}

pub fn make_open_office_tab<W>(context: &TemplateContext, writer: W, tab_view: OptionallyBreakRelevantTab, tournament_name: String) -> 
    Result<(), anyhow::Error> where W: Write + std::io::Seek {
    let mut break_marks = HashMap::new();
    let mut breaking_adjudicators = vec![];

    if let OptionallyBreakRelevantTab::BreakRelevantTab(tab) = &tab_view {
        for breaking_team in tab.breaking_teams.iter() {
            break_marks.entry(breaking_team.clone()).or_insert(vec![]).push("Break");

            for member in tab.team_members.get(breaking_team).unwrap_or(&vec![]).iter() {
                break_marks.entry(member.clone()).or_insert(vec![]).push("Break in Team");
            }
        }
        
        for breaking_speaker in tab.breaking_speakers.iter() {
            break_marks.entry(breaking_speaker.clone()).or_insert(vec![]).push("Break");
        }

        breaking_adjudicators = tab.breaking_adjudicators.clone();
    }

    let tab = match tab_view {
        OptionallyBreakRelevantTab::Tab(tab) => tab,
        OptionallyBreakRelevantTab::BreakRelevantTab(tab) => tab.tab,
    };
    let mut values = Context::from_serialize(&tab)?;
    values.insert("tournament_name", &Value::String(tournament_name));
    values.insert("break_marks", &serde_json::json!(break_marks));
    values.insert("breaking_adjudicators", &serde_json::json!(breaking_adjudicators));

    let tab_xml = context.tera.render("open_office/tab.xml", &values)?;
    let styles_xml = context.tera.render("open_office/tab_styles.xml", &Context::new())?;

    OpenOfficeDocument {
        content: tab_xml,
        styles: styles_xml,
        additional_files: vec![
        ],
        additional_files_data: vec![
        ].into_iter().collect(),
        doc_media_type: "application/vnd.oasis.opendocument.text".into(),
    }.write(&context, writer)?;

    return Ok(());
}

pub fn make_pdf_registration_items<W>(
    _context: &TemplateContext,
    mut writer: W,
    registration_info: RegistrationInfo,
) -> Result<(), anyhow::Error> where W: Write + std::io::Seek {
    let table_rows = registration_info.participant_info.into_iter().map(
        |p| {
            RowInfo {
                cells: vec![
                    CellInfo {
                        width: CellWidth::Fixed(45.0),
                        content: Box::new(
                            QRCodeLayouter {
                                content: p.registration_url.unwrap_or("".into()),
                                size: 35.0,
                            }
                        )
                    },
                    CellInfo {
                        width: CellWidth::Dynamic,
                        content: Box::new(
                            TextLayouter {
                                text: format!("{}\n{}", p.name, p.role),
                                font_size: 12.0,
                                font: "Helvetica".into(),
                            }
                        )
                    },
                ]
            }
        }
    ).collect();

    let mut doc = DocumentLayouter::new();
    doc.add_element(Box::new(
        TabularLayouter {
            rows: table_rows,
            row_margin: 10.0,
        }
    ));

    let layouted_doc = doc.layout()?;
    writer.write(&layouted_doc.write_as_pdf()?)?;

    return Ok(());
}