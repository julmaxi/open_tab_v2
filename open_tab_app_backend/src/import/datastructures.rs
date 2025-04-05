pub struct ParticipantFileData {
    pub teams: Vec<TeamData>,
    pub adjudicators: Vec<AdjudicatorData>
}

pub enum NameData {
    Full(String),
    FirstLast{ first: String, last: String },
}

pub struct TeamData {
    pub members: Vec<SpeakerData>,
    pub name: String
}

pub struct ParticipantData {
    pub name: String,
    pub institutions: Vec<String>,
    pub clashes: Vec<String>,
    pub is_anonymous: Option<bool>,
    pub break_category: Option<String>,
}

pub struct SpeakerData {
    pub participant_data: ParticipantData,
}

pub struct AdjudicatorData {
    pub participant_data: ParticipantData,
    pub chair_skill: i16,
    pub panel_skill: i16,
}