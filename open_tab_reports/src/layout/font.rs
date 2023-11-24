use std::{sync::Arc};

use allsorts::{binary::read::ReadScope, font_data::{FontData, DynamicFontTableProvider}};
use font_kit::source::SystemSource;


pub struct Font {
    pub(crate) data: Arc<Vec<u8>>,
    pub(crate) name: String,
}


impl Font {
    pub fn as_swash<'a>(&'a self) -> swash::FontRef<'a> {
        swash::FontRef {
            data: &self.data,
            offset: 0,
            key: swash::CacheKey::new(),
        }
    }

    pub fn as_allsorts<'a>(&'a self) -> allsorts::Font<DynamicFontTableProvider<'a>> {
        let scope = ReadScope::new(&self.data);
        let font_file = scope.read::<FontData<'_>>().expect("unable to parse font");
        // Use a different index to access other fonts in a font collection (E.g. TTC)
        let provider = font_file
            .table_provider(0)
            .expect("unable to create table provider");
        let font = allsorts::Font::new(provider)
            .expect("unable to load font tables")
            .expect("unable to find suitable cmap sub-table");
        
        font
    }
}

pub struct FontLoader {
    source: SystemSource,
}

impl FontLoader {
    pub fn new() -> Self {
        Self {
            source: SystemSource::new()
        }
    }

    pub fn load_from_postscript_name(&self, name: String) -> Result<Font, anyhow::Error> {
        let font = self.source
        .select_by_postscript_name(&name).unwrap();

        let (buf, _font_index)= match font.clone() {
            font_kit::handle::Handle::Path { path, .. } => {
                println!("Path: {:?}", path);
                return Err(anyhow::anyhow!("Path fonts not supported"))
            },
            font_kit::handle::Handle::Memory { bytes, font_index } => {
                (bytes, font_index as usize)
            }
        };

        let font = Font {
            data: buf,
            name
        };

        Ok(font)
    }
}
