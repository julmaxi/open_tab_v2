use csv::ReaderBuilder;
use std::collections::HashMap;

pub struct InstitutionNormalizer {
    alias_to_name: HashMap<String, String>,
}

impl InstitutionNormalizer {
    pub fn from_csv_file(path: &str) -> anyhow::Result<Self> {
        let mut alias_to_name = HashMap::new();
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_path(path)?;
        for result in reader.records() {
            let record = result?;
            if record.len() != 2 {
                return Err(anyhow::anyhow!("Invalid CSV format"));
            }
            let name = record[0].to_string();
            let institutions = record[1].to_lowercase();
            let aliases = institutions.split(";");
            for alias in aliases {
                let alias = alias.trim().to_string();
                if alias_to_name.insert(alias.clone(), name.clone()) != None {
                    return Err(anyhow::anyhow!("Duplicate alias found: {}", alias));
                }
            }
            if alias_to_name.contains_key(&name) {
                return Err(anyhow::anyhow!("Duplicate name found: {}", name));
            }
            alias_to_name.insert(name.to_lowercase(), name);
        }
        Ok(InstitutionNormalizer { alias_to_name })
    }

    pub fn normalize(&self, institution: &str) -> Option<String> {
        dbg!(&institution, &self.alias_to_name);
        self.alias_to_name.get(&institution.to_lowercase()).cloned()
    }
}