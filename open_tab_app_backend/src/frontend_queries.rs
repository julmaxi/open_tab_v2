use std::error::Error;

use crate::import::{CSVReaderConfig};

pub async fn query_participant_csv_config_proposal(path: String) -> Result<CSVReaderConfig, Box<dyn Error>> {
    let file = std::fs::File::open(path.clone())?;
    Ok(CSVReaderConfig::default_from_file(&file)?)
}
