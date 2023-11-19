

use std::{collections::HashMap, path::Path};

use tera::{Context, Tera};
use open_tab_entities::{domain::ballot::Ballot, info::TournamentParticipantsInfo, derived_models::{DisplayBallot, ResultDebate, DrawPresentationInfo}};

use lazy_static::lazy_static;
use std::io::Write;
use zip::write::FileOptions;


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

impl TemplateContext {
    pub fn new(template_dir: String) -> Result<Self, anyhow::Error> {
        let mut tera = Tera::new(Path::new(&template_dir).join("**/*.xml").as_os_str().to_str().unwrap_or("templates/**/*.xml"))?;
        tera.autoescape_on(vec![".html", ".sql", ".xml"]);
        
        Ok(Self {
            tera,
            template_dir
        })
    }
}

impl OpenOfficeDocument {
    pub fn write<W>(&self, context: &TemplateContext, writer: W) -> Result<(), anyhow::Error> where W: Write + std::io::Seek {
        let additional_files = serde_json::json!({
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

pub fn make_open_office_ballots<W>(context: &TemplateContext, writer: W, info: DrawPresentationInfo) ->
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