use sea_orm::entity::prelude::*;


#[derive(Copy, Clone, Debug, EnumIter)]
pub enum MissingRoundRelations {
    TournamentBreakSpeaker,
    TournamentBreakTeam,
}

impl RelationTrait for MissingRoundRelations {
    fn def(&self) -> RelationDef {
        match self {
            Self::TournamentBreakSpeaker => super::tournament_break::Entity::has_many(super::tournament_break_speaker::Entity).into(),
            Self::TournamentBreakTeam => super::tournament_break::Entity::has_many(super::tournament_break_team::Entity).into(),
        }
    }
}
