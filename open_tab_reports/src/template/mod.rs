

use std::{collections::HashMap, path::Path};

use serde_json::Value;
use tera::{Context, Tera};
use open_tab_entities::{derived_models::{name_to_initials, DrawPresentationInfo, RegistrationInfo}, tab::{AugmentedBreakRelevantTabView, AugmentedTabView, BreakRelevantTabView, TabView}};


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
    Tab(AugmentedTabView),
    BreakRelevantTab(AugmentedBreakRelevantTabView)
}

pub fn write_open_office_tab<W>(context: &TemplateContext, writer: W, tab_view: OptionallyBreakRelevantTab, tournament_name: String) -> 
    Result<(), anyhow::Error> where W: Write + std::io::Seek {
    let doc = make_open_office_tab(context, tab_view, tournament_name)?;
    doc.write(&context, writer)?;

    return Ok(());
}

fn make_open_office_tab(context: &TemplateContext, tab_view: OptionallyBreakRelevantTab, tournament_name: String) -> Result<OpenOfficeDocument, anyhow::Error> {
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
    let doc = OpenOfficeDocument {
        content: tab_xml,
        styles: styles_xml,
        additional_files: vec![
        ],
        additional_files_data: vec![
        ].into_iter().collect(),
        doc_media_type: "application/vnd.oasis.opendocument.text".into(),
    };
    Ok(doc)
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
                        width: CellWidth::Fixed(65.0),
                        content: Box::new(
                            QRCodeLayouter {
                                content: p.registration_url.unwrap_or("".into()),
                                size: 55.0,
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

#[cfg(test)]
mod test {
    use open_tab_entities::tab::{AugmentedSpeakerTabEntry, AugmentedTeamTabEntry, SpeakerTabEntry, SpeakerTabEntryDetailedScore, TeamRoundRole, Uuid};
    use super::*;

    #[test]
    fn test_save_tab() {
        let tab_view = AugmentedTabView {
            num_rounds: 3,
            team_tab: vec![
                AugmentedTeamTabEntry {
                    team_name: "Team A".into(),
                    rank: 1,
                    team_uuid: Uuid::from_u128(10),
                    total_score: 100.0,
                    avg_score: Some(90.0),
                    detailed_scores: vec![],
                    member_ranks: vec![1, 4, 5],
                },
                AugmentedTeamTabEntry {
                    team_name: "Team B".into(),
                    rank: 2,
                    team_uuid: Uuid::from_u128(20),
                    total_score: 90.0,
                    avg_score: Some(85.0),
                    detailed_scores: vec![],
                    member_ranks: vec![2, 3, 6],
                },
            ],
            speaker_tab: vec![
                AugmentedSpeakerTabEntry {
                    speaker_name: "Speaker A".into(),
                    team_name: "Team A".into(),
                    rank: 1,
                    speaker_uuid: Uuid::from_u128(300),
                    team_uuid: Uuid::from_u128(10),
                    total_score: 95.0,
                    avg_score: Some(90.0),
                    detailed_scores: vec![
                        Some(SpeakerTabEntryDetailedScore {
                            score: 30.0,
                            team_role: TeamRoundRole::Government,
                            speech_position: 1,
                        }),
                        Some(SpeakerTabEntryDetailedScore {
                            score: 35.0,
                            team_role: TeamRoundRole::Government,
                            speech_position: 2,
                        }),
                        Some(SpeakerTabEntryDetailedScore {
                            score: 30.0,
                            team_role: TeamRoundRole::Government,
                            speech_position: 3,
                        }),
                    ],
                    is_anonymous: false
                },
                AugmentedSpeakerTabEntry {
                    speaker_name: "Speaker B".into(),
                    team_name: "Team B".into(),
                    rank: 2,
                    speaker_uuid: Uuid::from_u128(400),
                    team_uuid: Uuid::from_u128(20),
                    total_score: 85.0,
                    avg_score: Some(80.0),
                    detailed_scores: vec![
                        Some(SpeakerTabEntryDetailedScore {
                            score: 25.0,
                            team_role: TeamRoundRole::Opposition,
                            speech_position: 1,
                        }),
                        Some(SpeakerTabEntryDetailedScore {
                            score: 30.0,
                            team_role: TeamRoundRole::Opposition,
                            speech_position: 2,
                        }),
                        Some(SpeakerTabEntryDetailedScore {
                            score: 30.0,
                            team_role: TeamRoundRole::Opposition,
                            speech_position: 3,
                        }),
                    ],
                    is_anonymous: false
                },
                AugmentedSpeakerTabEntry {
                    rank: 3,
                    speaker_name: "Speaker C".into(),
                    team_name: "Team A".into(),
                    speaker_uuid: Uuid::from_u128(500),
                    team_uuid: Uuid::from_u128(10),
                    total_score: 80.0,
                    avg_score: Some(75.0),
                    detailed_scores: vec![
                        Some(SpeakerTabEntryDetailedScore {
                            score: 20.0,
                            team_role: TeamRoundRole::Government,
                            speech_position: 1,
                        }),
                        Some(SpeakerTabEntryDetailedScore {
                            score: 30.0,
                            team_role: TeamRoundRole::Government,
                            speech_position: 2,
                        }),
                        Some(SpeakerTabEntryDetailedScore {
                            score: 30.0,
                            team_role: TeamRoundRole::Government,
                            speech_position: 3,
                        }),
                    ],
                    is_anonymous: false
                },
            ],
        };
        
        let curr_dir = std::env::current_dir().unwrap();
        if curr_dir.ends_with("open_tab_reports") {
            std::env::set_current_dir(curr_dir.parent().unwrap()).unwrap();
        }
        let template_context = TemplateContext::new("./open_tab_reports/templates/".to_string()).unwrap();
        let result = make_open_office_tab(&template_context, OptionallyBreakRelevantTab::Tab(tab_view), "Test".into()).unwrap();

        let expected = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content office:version="1.2" xmlns:calcext="urn:org:documentfoundation:names:experimental:calc:xmlns:calcext:1.0" xmlns:chart="urn:oasis:names:tc:opendocument:xmlns:chart:1.0" xmlns:css3t="http://www.w3.org/TR/css3-text/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dom="http://www.w3.org/2001/xml-events" xmlns:dr3d="urn:oasis:names:tc:opendocument:xmlns:dr3d:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:drawooo="http://openoffice.org/2010/draw" xmlns:field="urn:openoffice:names:experimental:ooo-ms-interop:xmlns:field:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:form="urn:oasis:names:tc:opendocument:xmlns:form:1.0" xmlns:formx="urn:openoffice:names:experimental:ooxml-odf-interop:xmlns:form:1.0" xmlns:grddl="http://www.w3.org/2003/g/data-view#" xmlns:loext="urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0" xmlns:math="http://www.w3.org/1998/Math/MathML" xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0" xmlns:number="urn:oasis:names:tc:opendocument:xmlns:datastyle:1.0" xmlns:of="urn:oasis:names:tc:opendocument:xmlns:of:1.2" xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:officeooo="http://openoffice.org/2009/office" xmlns:ooo="http://openoffice.org/2004/office" xmlns:oooc="http://openoffice.org/2004/calc" xmlns:ooow="http://openoffice.org/2004/writer" xmlns:rpt="http://openoffice.org/2005/report" xmlns:script="urn:oasis:names:tc:opendocument:xmlns:script:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:tableooo="http://openoffice.org/2009/table" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:xforms="http://www.w3.org/2002/xforms" xmlns:xhtml="http://www.w3.org/1999/xhtml" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
    <office:scripts/>
    <office:font-face-decls>
        <style:font-face style:font-family-generic="roman" style:font-pitch="variable" style:name="Liberation Serif" svg:font-family="'Liberation Serif'"/>
        <style:font-face style:font-family-generic="swiss" style:font-pitch="variable" style:name="Liberation Sans" svg:font-family="'Liberation Sans'"/>
        <style:font-face style:font-family-generic="system" style:font-pitch="variable" style:name="Arial Unicode MS" svg:font-family="'Arial Unicode MS'"/>
    </office:font-face-decls>
    <office:automatic-styles>
        <style:style style:family="table" style:name="Table2">
            <style:table-properties fo:margin-left="0cm" style:shadow="none" style:width="8.29cm" table:align="left"/>
        </style:style>
        <style:style style:family="table-row" style:name="Table2.1">
            <style:table-row-properties fo:keep-together="always"/>
        </style:style>
        <style:style style:family="table-column" style:name="Table2.A">
            <style:table-column-properties style:column-width="0.901cm"/>
        </style:style>
        <style:style style:family="table-column" style:name="Table2.B">
            <style:table-column-properties style:column-width="4.71cm"/>
        </style:style>
        <style:style style:family="table-column" style:name="Table2.C">
            <style:table-column-properties style:column-width="1.339cm"/>
        </style:style>
        <style:style style:family="table-column" style:name="Table2.D">
            <style:table-column-properties style:column-width="1.341cm"/>
        </style:style>
        <style:style style:family="table-cell" style:name="Table2.A1">
            <style:table-cell-properties fo:border="none" fo:padding="0.097cm" style:vertical-align="middle"/>
        </style:style>
        <style:style style:family="table-cell" style:name="Table2.B1">
            <style:table-cell-properties fo:border="none" fo:padding="0.097cm"/>
        </style:style>
        <style:style style:family="table-cell" style:name="Table2.B2">
            <style:table-cell-properties fo:border="none" fo:padding="0.097cm"/>
        </style:style>
        <style:style style:family="table" style:name="Table1">
            <style:table-properties fo:margin-left="0cm" style:shadow="none" style:width="8.29cm" table:align="left"/>
        </style:style>
        <style:style style:family="table-column" style:name="Table1.A">
            <style:table-column-properties style:column-width="0.901cm"/>
        </style:style>
        <style:style style:family="table-column" style:name="Table1.B">
            <style:table-column-properties style:column-width="4.71cm"/>
        </style:style>
        <style:style style:family="table-column" style:name="Table1.C">
            <style:table-column-properties style:column-width="1.341cm"/>
        </style:style>
        <style:style style:family="table-column" style:name="Table1.D">
            <style:table-column-properties style:column-width="1.339cm"/>
        </style:style>
        <style:style style:family="table-cell" style:name="Table1.A1">
            <style:table-cell-properties fo:border="none" fo:padding="0.097cm" style:vertical-align="middle"/>
        </style:style>
        <style:style style:family="table-cell" style:name="Table1.B1">
            <style:table-cell-properties fo:border="none" fo:padding="0.097cm"/>
        </style:style>
        <style:style style:family="paragraph" style:name="P1" style:parent-style-name="Heading_20_1">
            <style:text-properties officeooo:paragraph-rsid="0008cf00" officeooo:rsid="0008cf00" style:font-name="Liberation Sans"/>
        </style:style>
        <style:style style:family="paragraph" style:name="P2" style:parent-style-name="Standard">
            <style:text-properties fo:font-weight="bold" officeooo:paragraph-rsid="000727f1" officeooo:rsid="000727f1" style:font-name="Liberation Sans" style:font-weight-asian="bold" style:font-weight-complex="bold"/>
        </style:style>
        <style:style style:family="paragraph" style:name="P3" style:parent-style-name="Standard">
            <style:text-properties officeooo:paragraph-rsid="0008cf00" style:font-name="Liberation Sans"/>
        </style:style>
        <style:style style:family="paragraph" style:name="P4" style:parent-style-name="Table_20_Contents">
            <style:paragraph-properties fo:text-align="center" style:justify-single-word="false"/>
            <style:text-properties officeooo:paragraph-rsid="000727f1" officeooo:rsid="000727f1" style:font-name="Liberation Sans"/>
        </style:style>
        <style:style style:family="paragraph" style:name="P5" style:parent-style-name="Table_20_Contents">
            <style:text-properties officeooo:paragraph-rsid="0008cf00" officeooo:rsid="0008cf00" style:font-name="Liberation Sans"/>
        </style:style>
        <style:style style:family="paragraph" style:name="P6" style:parent-style-name="Table_20_Contents">
            <style:text-properties fo:font-size="6pt" fo:font-style="italic" officeooo:paragraph-rsid="0008cf00" officeooo:rsid="0008cf00" style:font-name="Liberation Sans" style:font-size-asian="6pt" style:font-size-complex="6pt" style:font-style-asian="italic" style:font-style-complex="italic"/>
        </style:style>
        <style:style style:family="paragraph" style:name="P7" style:parent-style-name="Table_20_Contents">
            <style:text-properties fo:font-size="6pt" fo:font-style="normal" officeooo:paragraph-rsid="0008cf00" officeooo:rsid="0008cf00" style:font-name="Liberation Sans" style:font-size-asian="6pt" style:font-size-complex="6pt" style:font-style-asian="normal" style:font-style-complex="normal"/>
        </style:style>
        <style:style style:family="paragraph" style:name="P8" style:parent-style-name="Table_20_Contents">
            <style:paragraph-properties fo:text-align="center" style:justify-single-word="false"/>
            <style:text-properties fo:font-size="9pt" fo:font-style="normal" officeooo:paragraph-rsid="0008cf00" officeooo:rsid="0008cf00" style:font-name="Liberation Sans" style:font-size-asian="9pt" style:font-size-complex="9pt" style:font-style-asian="normal" style:font-style-complex="normal"/>
        </style:style>
        <style:style style:family="paragraph" style:name="P9" style:parent-style-name="Table_20_Contents">
            <style:text-properties fo:font-size="8pt" officeooo:paragraph-rsid="0008cf00" officeooo:rsid="0008cf00" style:font-name="Liberation Sans" style:font-size-asian="8pt" style:font-size-complex="8pt"/>
        </style:style>
        <style:style style:family="paragraph" style:name="P10" style:parent-style-name="Table_20_Contents">
            <style:paragraph-properties fo:text-align="center" style:justify-single-word="false"/>
            <style:text-properties fo:font-size="10pt" officeooo:paragraph-rsid="0008cf00" officeooo:rsid="0008cf00" style:font-name="Liberation Sans" style:font-size-asian="10pt" style:font-size-complex="10pt"/>
        </style:style>
        <style:style style:family="paragraph" style:name="P11" style:parent-style-name="Table_20_Contents">
            <style:paragraph-properties fo:text-align="center" style:justify-single-word="false"/>
            <style:text-properties fo:font-size="10pt" officeooo:paragraph-rsid="000727f1" officeooo:rsid="000727f1" style:font-name="Liberation Sans" style:font-size-asian="10pt" style:font-size-complex="10pt"/>
        </style:style>
        <style:style style:family="paragraph" style:name="P12" style:parent-style-name="Table_20_Contents">
            <style:paragraph-properties fo:text-align="center" style:justify-single-word="false"/>
            <style:text-properties fo:font-size="10pt" officeooo:paragraph-rsid="0008cf00" officeooo:rsid="000727f1" style:font-name="Liberation Sans" style:font-size-asian="10pt" style:font-size-complex="10pt"/>
        </style:style>
        <style:style style:family="paragraph" style:name="P13" style:parent-style-name="Table_20_Contents">
            <style:text-properties fo:font-size="8pt" officeooo:paragraph-rsid="0009b361" officeooo:rsid="000727f1" style:font-name="Liberation Sans" style:font-size-asian="8pt" style:font-size-complex="8pt"/>
        </style:style>
        <style:style style:family="paragraph" style:name="P14" style:parent-style-name="Table_20_Contents">
            <style:paragraph-properties fo:text-align="center" style:justify-single-word="false"/>
            <style:text-properties fo:font-size="8pt" fo:font-style="normal" officeooo:paragraph-rsid="0008cf00" officeooo:rsid="0008cf00" style:font-name="Liberation Sans" style:font-size-asian="8pt" style:font-size-complex="8pt" style:font-style-asian="normal" style:font-style-complex="normal"/>
        </style:style>
        <style:style style:family="text" style:name="T1">
            <style:text-properties officeooo:rsid="0008cf00"/>
        </style:style>
        <style:style style:family="text" style:name="T2">
            <style:text-properties style:text-position="super 58%"/>
        </style:style>
        <style:style style:family="text" style:name="T3">
            <style:text-properties fo:font-style="normal" style:font-style-asian="normal" style:font-style-complex="normal" style:text-position="super 58%"/>
        </style:style>
        <style:style style:family="text" style:name="T4">
            <style:text-properties style:text-position="sub 58%"/>
        </style:style>
        <style:style style:family="text" style:name="T5">
            <style:text-properties fo:font-style="normal" style:font-style-asian="normal" style:font-style-complex="normal" style:text-position="sub 58%"/>
        </style:style>
        <style:style style:family="text" style:name="T6">
            <style:text-properties style:text-position="0% 100%"/>
        </style:style>
        <style:style style:family="text" style:name="T7">
            <style:text-properties fo:font-style="normal" style:font-style-asian="normal" style:font-style-complex="normal" style:text-position="0% 100%"/>
        </style:style>
        <style:style style:family="text" style:name="T8">
            <style:text-properties fo:font-size="10pt" officeooo:rsid="0008cf00" style:font-size-asian="10pt" style:font-size-complex="10pt"/>
        </style:style>
        <style:style style:family="text" style:name="T9">
            <style:text-properties fo:font-size="10pt" fo:font-style="italic" officeooo:rsid="0008cf00" style:font-size-asian="10pt" style:font-size-complex="10pt" style:font-style-asian="italic" style:font-style-complex="italic"/>
        </style:style>
        <style:style style:family="text" style:name="T10">
            <style:text-properties fo:font-style="italic" officeooo:rsid="0008cf00" style:font-style-asian="italic" style:font-style-complex="italic"/>
        </style:style>
        <style:style style:family="text" style:name="T11">
            <style:text-properties fo:font-style="italic" officeooo:rsid="0009b361" style:font-style-asian="italic" style:font-style-complex="italic"/>
        </style:style>
        <style:style style:family="text" style:name="T12">
            <style:text-properties officeooo:rsid="0009b361"/>
        </style:style>
        <style:style style:family="text" style:name="T13">
            <style:text-properties fo:font-size="7pt" fo:font-style="italic" style:font-size-asian="7pt" style:font-size-complex="7pt" style:font-style-asian="italic" style:font-style-complex="italic"/>
        </style:style>
        <style:style style:family="section" style:name="Sect1">
            <style:section-properties style:editable="false" text:dont-balance-text-columns="true">
                <style:columns fo:column-count="2" fo:column-gap="0.497cm">
                    <style:column fo:end-indent="0.249cm" fo:start-indent="0cm" style:rel-width="4818*"/>
                    <style:column fo:end-indent="0cm" fo:start-indent="0.249cm" style:rel-width="4820*"/>
                </style:columns>
            </style:section-properties>
        </style:style>
    </office:automatic-styles>
    <office:body>
        <office:text>
            <text:sequence-decls>
                <text:sequence-decl text:display-outline-level="0" text:name="Illustration"/>
                <text:sequence-decl text:display-outline-level="0" text:name="Table"/>
                <text:sequence-decl text:display-outline-level="0" text:name="Text"/>
                <text:sequence-decl text:display-outline-level="0" text:name="Drawing"/>
            </text:sequence-decls>
            <text:h text:outline-level="1" text:style-name="P1">Test - Tab</text:h>
            <text:section text:name="Section1" text:style-name="Sect1">
                <text:p text:style-name="P2">Teams</text:p>
                <table:table table:name="Table2" table:style-name="Table2">
                    <table:table-column table:style-name="Table2.A"/>
                    <table:table-column table:style-name="Table2.B"/>
                    <table:table-column table:style-name="Table2.C"/>
                    <table:table-column table:style-name="Table2.D"/>
                    
                    <table:table-row style="Table2.1">
                        <table:table-cell office:value-type="string" table:style-name="Table2.A1">
                            <text:p text:style-name="P10">
                                <text:span text:style-name="T1">2.</text:span></text:p>
                        </table:table-cell>
                        <table:table-cell office:value-type="string" table:style-name="Table2.B1">
                            <text:p text:style-name="P9">Team A
                                <text:span text:style-name="T11">
                                    
                                </text:span>
                            </text:p>
                            <text:p text:style-name="P6">2+5+6</text:p>
                            <text:p text:style-name="P6">
                
                            </text:p>
                        </table:table-cell>
                        <table:table-cell office:value-type="string" table:style-name="Table2.A1">
                            <text:p text:style-name="P14">
                                100.00
                            </text:p>
                        </table:table-cell>
                        <table:table-cell office:value-type="string" table:style-name="Table2.A1">
                            <text:p text:style-name="P14">
                                
                                    90.00
                                
                            </text:p>
                        </table:table-cell>
                    </table:table-row>
                    
                    <table:table-row style="Table2.1">
                        <table:table-cell office:value-type="string" table:style-name="Table2.A1">
                            <text:p text:style-name="P10">
                                <text:span text:style-name="T1">3.</text:span></text:p>
                        </table:table-cell>
                        <table:table-cell office:value-type="string" table:style-name="Table2.B1">
                            <text:p text:style-name="P9">Team B
                                <text:span text:style-name="T11">
                                    
                                </text:span>
                            </text:p>
                            <text:p text:style-name="P6">3+4+7</text:p>
                            <text:p text:style-name="P6">
                
                            </text:p>
                        </table:table-cell>
                        <table:table-cell office:value-type="string" table:style-name="Table2.A1">
                            <text:p text:style-name="P14">
                                90.00
                            </text:p>
                        </table:table-cell>
                        <table:table-cell office:value-type="string" table:style-name="Table2.A1">
                            <text:p text:style-name="P14">
                                
                                    85.00
                                
                            </text:p>
                        </table:table-cell>
                    </table:table-row>
                    
                </table:table>
                <text:p text:style-name="P2"/>
                <text:p text:style-name="P2">Redner</text:p>
                <table:table table:name="Table1" table:style-name="Table1">
                    <table:table-column table:style-name="Table1.A"/>
                    <table:table-column table:style-name="Table1.B"/>
                    <table:table-column table:style-name="Table1.C"/>
                    <table:table-column table:style-name="Table1.D"/>
                    

                    <table:table-row table:style-name="Table2.1">
                        <table:table-cell office:value-type="string" table:style-name="Table1.A1">
                            <text:p text:style-name="P12">
                                <text:span text:style-name="T1">2</text:span>.</text:p>
                        </table:table-cell>
                        <table:table-cell office:value-type="string" table:style-name="Table1.B1">
                            <text:p text:style-name="P13">Speaker A
                                <text:span text:style-name="T11">
                                    
                                </text:span>
                            </text:p>
                            <text:p text:style-name="P6">Team A (G2+G3+G4)</text:p>
                            <text:p text:style-name="P7">
                                
                                    
                                        
                                        30.00
                                    
                                
                                    
                                        
                                        +
                                        
                                        35.00
                                    
                                
                                    
                                        
                                        +
                                        
                                        30.00
                                    
                                
                            </text:p>
                        </table:table-cell>
                        <table:table-cell office:value-type="string" table:style-name="Table2.A1">
                            <text:p text:style-name="P14">
                                95.00
                            </text:p>
                        </table:table-cell>
                        <table:table-cell office:value-type="string" table:style-name="Table2.A1">
                            <text:p text:style-name="P14">
                                
                                    90.00
                                
                            </text:p>
                        </table:table-cell>
                    </table:table-row>
                    

                    <table:table-row table:style-name="Table2.1">
                        <table:table-cell office:value-type="string" table:style-name="Table1.A1">
                            <text:p text:style-name="P12">
                                <text:span text:style-name="T1">3</text:span>.</text:p>
                        </table:table-cell>
                        <table:table-cell office:value-type="string" table:style-name="Table1.B1">
                            <text:p text:style-name="P13">Speaker B
                                <text:span text:style-name="T11">
                                    
                                </text:span>
                            </text:p>
                            <text:p text:style-name="P6">Team B (O2+O3+O4)</text:p>
                            <text:p text:style-name="P7">
                                
                                    
                                        
                                        25.00
                                    
                                
                                    
                                        
                                        +
                                        
                                        30.00
                                    
                                
                                    
                                        
                                        +
                                        
                                        30.00
                                    
                                
                            </text:p>
                        </table:table-cell>
                        <table:table-cell office:value-type="string" table:style-name="Table2.A1">
                            <text:p text:style-name="P14">
                                85.00
                            </text:p>
                        </table:table-cell>
                        <table:table-cell office:value-type="string" table:style-name="Table2.A1">
                            <text:p text:style-name="P14">
                                
                                    80.00
                                
                            </text:p>
                        </table:table-cell>
                    </table:table-row>
                    

                    <table:table-row table:style-name="Table2.1">
                        <table:table-cell office:value-type="string" table:style-name="Table1.A1">
                            <text:p text:style-name="P12">
                                <text:span text:style-name="T1">4</text:span>.</text:p>
                        </table:table-cell>
                        <table:table-cell office:value-type="string" table:style-name="Table1.B1">
                            <text:p text:style-name="P13">Speaker C
                                <text:span text:style-name="T11">
                                    
                                </text:span>
                            </text:p>
                            <text:p text:style-name="P6">Team A (G2+G3+G4)</text:p>
                            <text:p text:style-name="P7">
                                
                                    
                                        
                                        20.00
                                    
                                
                                    
                                        
                                        +
                                        
                                        30.00
                                    
                                
                                    
                                        
                                        +
                                        
                                        30.00
                                    
                                
                            </text:p>
                        </table:table-cell>
                        <table:table-cell office:value-type="string" table:style-name="Table2.A1">
                            <text:p text:style-name="P14">
                                80.00
                            </text:p>
                        </table:table-cell>
                        <table:table-cell office:value-type="string" table:style-name="Table2.A1">
                            <text:p text:style-name="P14">
                                
                                    75.00
                                
                            </text:p>
                        </table:table-cell>
                    </table:table-row>
                    
                </table:table>
                <text:p text:style-name="P3"/>

                
            </text:section>
        </office:text>
    </office:body>
</office:document-content>"#.trim();

        assert_eq!(result.content, expected);
    }
}