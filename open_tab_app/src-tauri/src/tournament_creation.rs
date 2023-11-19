use serde::{Serialize, Deserialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentCreationConfig {
    pub name: String,
    pub num_preliminaries: u32,
    pub num_break_rounds: u32,
}
