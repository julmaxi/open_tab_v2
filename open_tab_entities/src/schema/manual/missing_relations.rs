use sea_orm::entity::prelude::*;


#[derive(Copy, Clone, Debug, EnumIter)]
pub enum MissingRoundRelations {
    TournamentBreakSourceRound,
    TournamentBreakChildRound,
    TournamentBreakSpeaker,
    TournamentBreakTeam,
}

impl RelationTrait for MissingRoundRelations {
    fn def(&self) -> RelationDef {
        match self {
            Self::TournamentBreakSourceRound => super::tournament_break::Entity::has_many(super::tournament_break_source_round::Entity).into(),
            Self::TournamentBreakChildRound => super::tournament_break::Entity::has_many(super::tournament_break_child_round::Entity).into(),
            Self::TournamentBreakSpeaker => super::tournament_break::Entity::has_many(super::tournament_break_speaker::Entity).into(),
            Self::TournamentBreakTeam => super::tournament_break::Entity::has_many(super::tournament_break_team::Entity).into(),
        }
    }
}

impl Related<super::tournament_break_speaker::Entity> for super::tournament_break::Entity {
    fn to() -> RelationDef {
        MissingRoundRelations::TournamentBreakSpeaker.def()
    }
}

impl Related<super::tournament_break_team::Entity> for super::tournament_break::Entity {
    fn to() -> RelationDef {
        MissingRoundRelations::TournamentBreakTeam.def()
    }
}

impl Related<super::tournament_break_child_round::Entity> for super::tournament_break::Entity {
    fn to() -> RelationDef {
        MissingRoundRelations::TournamentBreakChildRound.def()
    }
}

impl Related<super::tournament_break_source_round::Entity> for super::tournament_break::Entity {
    fn to() -> RelationDef {
        MissingRoundRelations::TournamentBreakSourceRound.def()
    }
}