use std::{collections::HashMap, error::Error, fmt::{Formatter, Display}};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use serde::{Serialize, Deserialize};

use super::{ParticipantFileData, ParticipantData, AdjudicatorData, SpeakerData, TeamData};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CSVReaderConfig {
    name_column: Option<CSVNameCol>,
    role_column: Option<usize>,
    institutions_column: Option<usize>,
    clashes_column: Option<usize>,
    delimiter: Option<u8>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
enum CSVNameCol {
    FirstLast{ first: usize, last: usize },
    Full {column: usize}
}

#[derive(Debug)]
pub enum CSVParserErr {
    ParseError(csv::Error),
    IoError(std::io::Error),
    IndexOutOfBounds{ index: usize },
    BadConfig
}

impl Display for CSVParserErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for CSVParserErr {}


#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum CSVField {
    FullName,
    FirstName,
    LastName,
    Role,
    Institutions,
    Conflicts
}

pub struct ParseResult {
    pub data: ParticipantFileData,
    pub warnings: Vec<ParseWarning>,
}

pub enum ParseWarning {
    TeamHasWrongSize{ name: String, num_members: u32 },
    SkippedRowPartialEntry { index: usize }
}


impl CSVReaderConfig {
    pub fn default_from_file<R>(mut reader: R) -> Result<CSVReaderConfig, CSVParserErr> where R: std::io::Read {
        let delimiter_candidates = [b',', b';', b'\t'];
        let mut delimiter_counts = [0; 3];
        let mut buffer = Vec::new();

        let read_result = reader.read_to_end(&mut buffer);
        if let Err(e) = read_result {
            return Err(CSVParserErr::IoError(e));
        }

        for char in buffer.iter() {
            for (i, delimiter) in delimiter_candidates.iter().enumerate() {
                if char == delimiter {
                    delimiter_counts[i] += 1;
                }
            }
        }

        let delimiter = delimiter_counts.into_iter().enumerate().max_by_key(|(_, c)| *c).map(|(i, _)| delimiter_candidates[i]).unwrap_or(b',');
        let mut reader = csv::ReaderBuilder::new().delimiter(delimiter).from_reader(&buffer[..]);
        let headers = reader.headers().map_err(|e| {
            CSVParserErr::ParseError(e)
        })?;

        let mut config = Self::propose_config_from_headers(headers.into_iter());
        config.delimiter = Some(delimiter);
        Ok(config)
    }

    fn propose_config_from_headers<'a, I>(headers: I) -> CSVReaderConfig where I: Iterator<Item=&'a str> {
        lazy_static! {
            static ref FIELD_HEADER_PATTERNS: HashMap<CSVField, Regex> = {
                let full_name_patterns : Vec<&str> = vec!["name"];
                let first_name_patterns : Vec<&str> = vec!["vorname"];
                let last_name_patterns : Vec<&str> = vec!["(nach)?name"];
                let institutions_patterns: Vec<&str> = vec!["(debattier)?club"];
                let role_patterns: Vec<&str> = vec!["team(name)?", "rolle"];
                let conflicts_patterns: Vec<&str> = vec!["konflikt", "clash(es)?", "nicht.*jurieren"];                

                let mut m = HashMap::new();
                m.insert(CSVField::FullName, full_name_patterns);
                m.insert(CSVField::FirstName, first_name_patterns);
                m.insert(CSVField::LastName, last_name_patterns);
                m.insert(CSVField::Institutions, institutions_patterns);
                m.insert(CSVField::Role, role_patterns);
                m.insert(CSVField::Conflicts, conflicts_patterns);
                
                m.into_iter().map(|(key, patterns)| (key, RegexBuilder::new(&patterns.join("|").to_string()).case_insensitive(true).build().unwrap())).collect()
            };
        }

        let mut proposed_column_assignment = HashMap::new();
        for (header_idx, header) in headers.enumerate() {
            for (field, pattern) in FIELD_HEADER_PATTERNS.iter() {
                if pattern.is_match(header) && proposed_column_assignment.get(field) == None {
                    proposed_column_assignment.insert(*field, header_idx);
                }
            }
        }

        let name_column = match (proposed_column_assignment.get(&CSVField::FirstName), proposed_column_assignment.get(&CSVField::LastName), proposed_column_assignment.get(&CSVField::FullName)) {
            (Some(first), Some(last), _) => Some(CSVNameCol::FirstLast { first: *first, last: *last }),
            (_, _, Some(full)) => Some(CSVNameCol::Full {column: *full}),
            (_, _, _) => None
        };

        CSVReaderConfig {
            name_column,
            role_column: proposed_column_assignment.remove(&CSVField::Role),
            institutions_column: proposed_column_assignment.remove(&CSVField::Institutions),
            clashes_column: proposed_column_assignment.remove(&CSVField::Conflicts),
            delimiter: None
        }
    }

    pub fn parse<R>(&self, reader: R) -> Result<ParseResult, CSVParserErr> where R: std::io::Read {
        let delimiter = self.delimiter.ok_or(CSVParserErr::BadConfig)?;
        let mut reader = csv::ReaderBuilder::new().delimiter(delimiter).flexible(true).from_reader(reader);

        let role_idx = self.role_column.ok_or(CSVParserErr::BadConfig)?;

        let mut teams : HashMap<String, Vec<SpeakerData>> = HashMap::new();
        let mut adjudicators = vec![];
        let mut warnings = vec![];

        for (row_idx, row) in reader.records().into_iter().enumerate() {
            let row = row.map_err(|e| CSVParserErr::ParseError(e))?;

            let name = match self.name_column {
                Some(CSVNameCol::Full {column: index}) => row.get(index).ok_or(CSVParserErr::IndexOutOfBounds { index })?.to_string(),
                Some(CSVNameCol::FirstLast{first, last }) => {
                    let first_name = row.get(first).ok_or(CSVParserErr::IndexOutOfBounds { index: first })?;
                    let last_name = row.get(last).ok_or(CSVParserErr::IndexOutOfBounds { index: last })?;

                    format!("{} {}", first_name, last_name)
                }
                None => {Err(CSVParserErr::BadConfig)?},
            };

            if name.len() == 0 {
                warnings.push(ParseWarning::SkippedRowPartialEntry { index: row_idx });
                continue;
            }

            let institutions = match self.institutions_column {
                Some(index) => row.get(
                    index
                ).map(
                    |i| i.split(";").map(|i| i.trim().to_string()).collect()
                ).unwrap_or(vec![]),
                None => vec![]
            };

            let clashes = match self.clashes_column {
                Some(index) => row.get(
                    index
                ).map(
                    |i| i.split(";").map(|i| i.trim().to_string()).collect()
                ).unwrap_or(vec![]),
                None => vec![]
            };

            let participant_data = ParticipantData {
                name,
                institutions,
                clashes
            };

            let role = row.get(role_idx).ok_or(CSVParserErr::IndexOutOfBounds { index: role_idx })?.to_string();

            if role.len() == 0 || role.starts_with("#") {
                let (chair_skill, panel_skill) = if role.starts_with("#") && role.len() == 3 {
                    let chair_skill = role.chars().nth(1).unwrap().to_digit(10);
                    let panel_skill = role.chars().nth(2).unwrap().to_digit(10);

                    match (chair_skill, panel_skill) {
                        (Some(chair), Some(panel)) => (chair * 10, panel * 10),
                        _ => (50, 50)
                    }
                } else {
                    (50, 50)
                };
                adjudicators.push(AdjudicatorData { participant_data, chair_skill: chair_skill as i16, panel_skill: panel_skill as i16});
            }
            else {
                let speaker = SpeakerData { participant_data };
                match teams.get_mut(&role) {
                    Some(members) => {members.push(speaker);},
                    None => {teams.insert(role, vec![speaker]);}
                }
            }
        }

        for (team, members) in teams.iter() {
            if members.len() != 3 {
                warnings.push(ParseWarning::TeamHasWrongSize { name: team.clone(), num_members: members.len() as u32 })
            }
        }

        Ok(ParseResult {
            warnings,
            data: ParticipantFileData {
                teams: teams.into_iter().map(|(name, members)| TeamData {name, members}).collect(),
                adjudicators,
            }
        })
    }
}

#[test]
fn test_propose_from_empty_header() {
    let headers = vec![];

    let config = CSVReaderConfig::propose_config_from_headers(headers.into_iter());

    assert_eq!(config.name_column, None);
    assert_eq!(config.role_column, None);
    assert_eq!(config.institutions_column, None);
    assert_eq!(config.clashes_column, None);
}

#[test]
fn test_propose_with_full_name_header() {
    let headers = vec!["Name"];

    let headers = CSVReaderConfig::propose_config_from_headers(headers.into_iter());

    assert_eq!(headers.name_column, Some(CSVNameCol::Full {column: 0}));
    assert_eq!(headers.role_column, None);
    assert_eq!(headers.institutions_column, None);
    assert_eq!(headers.clashes_column, None);
}

#[test]
fn test_propose_with_first_last_name_header() {
    let headers = vec!["Name", "Vorname"];

    let headers = CSVReaderConfig::propose_config_from_headers(headers.into_iter());

    assert_eq!(headers.name_column, Some(CSVNameCol::FirstLast { first: 1, last: 0 }));
    assert_eq!(headers.role_column, None);
    assert_eq!(headers.institutions_column, None);
    assert_eq!(headers.clashes_column, None);
}


#[test]
fn test_propose_full_header() {
    let headers = vec!["Club", "Name", "Vorname", "Team", "Clashes"];

    let headers = CSVReaderConfig::propose_config_from_headers(headers.into_iter());

    assert_eq!(headers.name_column, Some(CSVNameCol::FirstLast { first: 2, last: 1 }));
    assert_eq!(headers.role_column, Some(3));
    assert_eq!(headers.institutions_column, Some(0));
    assert_eq!(headers.clashes_column, Some(4));
}

#[test]
fn test_read_valid_data_with_full_name() -> Result<(), anyhow::Error> {
    let config = CSVReaderConfig {
        name_column: Some(CSVNameCol::Full {column: 0}),
        role_column: Some(1),
        institutions_column: Some(2),
        clashes_column: Some(3),
        delimiter: Some(b',')
    };

    let test_file = "Name,Team,Club,Clashes
Pers. A,A,Club A;Club B,
Pers. B,A,Club A,Pers. A
Pers. C,A,Club A,
Pers. D,,Club C,
";
    let parsed = config.parse(test_file.as_bytes())?;

    assert_eq!(parsed.data.teams.len(), 1);
    assert_eq!(parsed.data.teams[0].members.iter().map(|m| m.participant_data.name.clone()).sorted().collect_vec(), vec!["Pers. A", "Pers. B", "Pers. C"]);
    assert_eq!(parsed.data.adjudicators.iter().map(|a| a.participant_data.name.clone()).collect_vec(), vec!["Pers. D"]);

    Ok(())
}

#[test]
fn test_read_valid_data_with_first_and_last_name() -> Result<(), anyhow::Error> {
    let config = CSVReaderConfig {
        name_column: Some(CSVNameCol::FirstLast{first: 0, last: 1}),
        role_column: Some(2),
        institutions_column: Some(3),
        clashes_column: Some(4),
        delimiter: Some(b',')
    };

    let test_file = "Vorname,Name,Team,Club,Clashes
Pers.,A,A,Club A;Club B,
Pers.,B,A,Club A,Pers. A
Pers.,C,A,Club A,
Pers.,D,,Club C,
";
    let parsed = config.parse(test_file.as_bytes())?;

    assert_eq!(parsed.data.teams.len(), 1);
    assert_eq!(parsed.data.teams[0].members.iter().map(|m| m.participant_data.name.clone()).sorted().collect_vec(), vec!["Pers. A", "Pers. B", "Pers. C"]);
    assert_eq!(parsed.data.adjudicators.iter().map(|a| a.participant_data.name.clone()).collect_vec(), vec!["Pers. D"]);

    Ok(())
}