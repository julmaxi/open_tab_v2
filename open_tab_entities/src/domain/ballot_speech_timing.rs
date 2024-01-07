use std::collections::HashMap;

use async_trait::async_trait;
use itertools::Itertools;
use open_tab_macros::SimpleEntity;
use sea_orm::prelude::*;
use serde::{Serialize, Deserialize};

use crate::schema;

use super::ballot::{Ballot, SpeechRole};
use crate::domain::entity::LoadEntity;


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, SimpleEntity)]
#[module_path = "crate::schema::ballot_speech_timing"]
#[get_many_tournaments_func = "get_many_tournaments_impl"]
pub struct BallotSpeechTiming {
    pub uuid: Uuid,
    pub speech_ballot_id: Uuid,
    
    pub speech_role: String,
    pub speech_position: i32,
    pub start_time: Option<DateTime>,
    pub end_time: Option<DateTime>,
    pub response_start_time: Option<DateTime>,
    pub response_end_time: Option<DateTime>
}


impl BallotSpeechTiming {
    async fn get_many_tournaments_impl<C>(db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let ballots = schema::ballot::Entity::find()
            .inner_join(schema::ballot_speech::Entity)
            .filter(
                schema::ballot_speech::Column::BallotId.is_in(entities.iter().map(|timing| timing.speech_ballot_id).collect_vec())
            ).all(db).await?;

        let ballot_ids = ballots.iter().map(|ballot| ballot.uuid).collect_vec();
        let ballot_tournaments = Ballot::get_tournaments_from_ids(db, ballot_ids.clone()).await?.into_iter();

        let tournament_ids_by_ballot_id = ballot_ids.into_iter().zip(ballot_tournaments).collect::<HashMap<_, _>>();

        entities.iter().map(|b| {
            Ok(tournament_ids_by_ballot_id.get(&b.speech_ballot_id).cloned().flatten())
        }).collect()
    }

    pub async fn get_all_in_debate<C>(db: &C, debate_uuid: Uuid) -> Result<Vec<BallotSpeechTiming>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let debate_ballots = schema::ballot::Entity::find()
            .inner_join(schema::ballot_speech::Entity)
            .inner_join(schema::tournament_debate::Entity)
            .filter(
                schema::tournament_debate::Column::Uuid.eq(debate_uuid)
            ).all(db).await?;

        let ballot_ids = debate_ballots.iter().map(|ballot| ballot.uuid).collect_vec();

        let speech_timings = schema::ballot_speech_timing::Entity::find()
            .filter(
                schema::ballot_speech_timing::Column::SpeechBallotId.is_in(ballot_ids)
            ).all(db).await?;
        
        Ok(speech_timings.into_iter().map(Self::from_model).collect())
    }

    pub async fn get_from_speech<C>(db: &C, ballot_uuid: Uuid, role: SpeechRole, position: i32) -> Result<Option<BallotSpeechTiming>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let speech_timings = schema::ballot_speech_timing::Entity::find()
            .filter(
                schema::ballot_speech_timing::Column::SpeechBallotId.eq(ballot_uuid).and(
                    schema::ballot_speech_timing::Column::SpeechRole.eq(role.to_str()).and(
                        schema::ballot_speech_timing::Column::SpeechPosition.eq(position)
                    )
                )
            ).one(db).await?;
        
        Ok(speech_timings.map(Self::from_model))
    }
}
