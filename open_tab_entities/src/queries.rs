use std::collections::HashMap;

use itertools::Itertools;

use uuid::Uuid;
use sea_orm::{ConnectionTrait, prelude::*, QuerySelect};

use crate::{schema, prelude::SpeechRole, domain::ballot::JudgeRole};


#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ParticipantRoundRole {
    TeamSpeaker { debate_uuid: Uuid, role: SpeechRole },
    NonAlignedSpeaker { debate_uuid: Uuid, position: u8 },
    Adjudicator { debate_uuid: Uuid, position: u8 },
    President { debate_uuid: Uuid },
    Multiple,
    None,
}

pub async fn query_all_participant_roles<C>(db: &C, participant_uuid: Uuid) -> Result<HashMap<Uuid, ParticipantRoundRole>, DbErr> where C: sea_orm::ConnectionTrait {
    let all_rounds : Vec<Uuid> = schema::tournament_round::Entity::find()
        .select_only()
        .column(schema::tournament_round::Column::Uuid)
        .inner_join(schema::tournament::Entity)
        .join_rev(sea_orm::JoinType::InnerJoin, schema::participant::Relation::Tournament.def())
        .filter(schema::participant::Column::Uuid.eq(participant_uuid))
        .into_tuple()
        .all(db).await?;
    let team_rounds : Vec<(Uuid, Uuid, String)> = schema::ballot_team::Entity::find()
        .select_only()
        .column(schema::tournament_debate::Column::RoundId)
        .column(schema::tournament_debate::Column::Uuid)
        .column(schema::ballot_team::Column::Role)
        .inner_join(schema::ballot::Entity)
        .inner_join(schema::team::Entity)
        .join_rev(sea_orm::JoinType::InnerJoin, schema::tournament_debate::Relation::Ballot.def())
        .join(sea_orm::JoinType::InnerJoin, schema::team::Relation::Speaker.def())
        .filter(schema::speaker::Column::Uuid.eq(participant_uuid))
        .into_tuple()
        .all(db).await?;

    let speech_rounds : Vec<(Uuid, Uuid, i32, String)> = schema::ballot_speech::Entity::find()
        .select_only()
        .column(schema::tournament_debate::Column::RoundId)
        .column(schema::tournament_debate::Column::Uuid)
        .column(schema::ballot_speech::Column::Position)
        .column(schema::ballot_speech::Column::Role)
        .inner_join(schema::speaker::Entity)
        .inner_join(schema::ballot::Entity)
        .join_rev(sea_orm::JoinType::InnerJoin, schema::tournament_debate::Relation::Ballot.def())
        .filter(schema::speaker::Column::Uuid.eq(participant_uuid))
        .into_tuple()
        .all(db).await?;
    
    let adjudicator_rounds : Vec<(Uuid, Uuid, i32, String)> = schema::ballot_adjudicator::Entity::find()
        .select_only()
        .column(schema::tournament_debate::Column::RoundId)
        .column(schema::tournament_debate::Column::Uuid)
        .column(schema::ballot_adjudicator::Column::Position)
        .column(schema::ballot_adjudicator::Column::Role)
        .inner_join(schema::ballot::Entity)
        .join_rev(sea_orm::JoinType::InnerJoin, schema::tournament_debate::Relation::Ballot.def())
        .inner_join(schema::adjudicator::Entity)
        .filter(schema::adjudicator::Column::Uuid.eq(participant_uuid))
        .into_tuple()
        .all(db).await?;

    let vals = team_rounds.into_iter().map(|(round_uuid, debate_uuid, role)| {
        dbg!(round_uuid);
        (round_uuid, ParticipantRoundRole::TeamSpeaker {
            debate_uuid,
            role: role.parse().unwrap_or(SpeechRole::Government)
        })
    }).chain(
        speech_rounds.into_iter().map(|(round_uuid, debate_uuid, position, role)| {
            let role : SpeechRole = role.parse().unwrap_or(SpeechRole::NonAligned);
            let role = match role {
                SpeechRole::Government => ParticipantRoundRole::TeamSpeaker {
                    debate_uuid,
                    role: SpeechRole::Government
                },
                SpeechRole::Opposition => ParticipantRoundRole::TeamSpeaker {
                    debate_uuid,
                    role: SpeechRole::Opposition
                },
                SpeechRole::NonAligned => ParticipantRoundRole::NonAlignedSpeaker {
                    debate_uuid,
                    position: position as u8
                },
            };

            (round_uuid, role)
        })
    ).chain(
        adjudicator_rounds.into_iter().map(|(round_uuid, debate_uuid, position, role_str)| {
            let role = role_str.parse().unwrap_or(JudgeRole::Normal);
            let round_role = match role {
                JudgeRole::President => {
                    ParticipantRoundRole::President {
                        debate_uuid
                    }
                }
                _ => ParticipantRoundRole::Adjudicator {
                    debate_uuid,
                    position: position as u8
                }
            };
            (round_uuid, round_role)
        })
    ).into_group_map();

    let mut roles : HashMap<Uuid, ParticipantRoundRole> = vals.into_iter().map(
        |(round_id, found_positions)| {
            let found_positions = found_positions.into_iter().coalesce(
                |prev, next| {
                    if prev == next {
                        Ok(prev)
                    } else {
                        Err((prev, next))
                    }
                }
            ).collect_vec();
            match found_positions.len() {
                0 => (round_id, ParticipantRoundRole::None), // This never happens
                1 => (round_id, found_positions.into_iter().next().unwrap()),
                _ => (round_id, ParticipantRoundRole::Multiple)
            }
        }
    ).collect();

    for round_id in all_rounds {
        if !roles.contains_key(&round_id) {
            roles.insert(round_id, ParticipantRoundRole::None);
        }
    }
    
    Ok(roles)
}
