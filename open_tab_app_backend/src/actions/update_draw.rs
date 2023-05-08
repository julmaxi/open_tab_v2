use std::{error::Error, fmt::{Display, Formatter}, collections::HashMap};

use itertools::{Itertools, izip};
use migration::async_trait::async_trait;
use open_tab_entities::{prelude::*};

use sea_orm::prelude::*;

use crate::draw_view::DrawBallot;
use serde::{Serialize, Deserialize};

use super::ActionTrait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDrawAction {
    pub updated_ballots: Vec<DrawBallot>
}

#[derive(Debug)]
pub enum UpdateDrawError {
}

impl Display for UpdateDrawError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for UpdateDrawError {}

#[async_trait]
impl ActionTrait for UpdateDrawAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroups, Box<dyn Error>> where C: ConnectionTrait {
        let mut groups = EntityGroups::new();

        let values = self.updated_ballots.iter().map(|d| d.uuid).collect_vec();

        let ballots = open_tab_entities::domain::ballot::Ballot::try_get_many(
            db,
            values.clone()
        ).await?.into_iter().enumerate().map(|(idx, ballot)| {
            if let Some(ballot) = ballot {
                ballot
            } else {
                Ballot{ uuid: values[idx], ..Default::default() }
            }
        });

        for (ballot, debate) in izip![ballots, self.updated_ballots.iter()] {
            let mut new_ballot = ballot.clone();

            new_ballot.government.team = if let Some(gov) = &debate.government { Some(gov.uuid) } else { None };
            new_ballot.opposition.team = if let Some(opp) = &debate.opposition { Some(opp.uuid) } else { None };

            let mut existing_non_aligned_speeches = new_ballot.speeches.iter_mut().filter(|speech| speech.role == open_tab_entities::domain::ballot::SpeechRole::NonAligned).collect_vec();

            let mut new_speeches = vec![];
            for (i, speech) in debate.non_aligned_speakers.iter().enumerate() {
                if i < existing_non_aligned_speeches.len() {
                    existing_non_aligned_speeches[i].speaker = Some(speech.uuid);
                } else {
                    new_speeches.push(Speech {
                        speaker: Some(speech.uuid),
                        role: open_tab_entities::domain::ballot::SpeechRole::NonAligned,
                        position: i as u8,
                        scores: HashMap::new(),
                    });
                }
            }
            drop(existing_non_aligned_speeches);
            new_ballot.speeches = new_ballot.speeches.into_iter().filter(|speech| speech.role != open_tab_entities::domain::ballot::SpeechRole::NonAligned || speech.position < debate.non_aligned_speakers.len() as u8).collect_vec();
            new_ballot.speeches.extend(new_speeches);

            let old_adjudicators = new_ballot.adjudicators.clone();
            for (i, adjudicator) in debate.adjudicators.iter().enumerate() {
                if i < new_ballot.adjudicators.len() {
                    new_ballot.adjudicators[i] = adjudicator.adjudicator.uuid;
                }
                else {
                    new_ballot.adjudicators.push(adjudicator.adjudicator.uuid);
                }
            }

            new_ballot.adjudicators.truncate(debate.adjudicators.len());

            for deleted_adjudicator in old_adjudicators.iter().filter(|uuid| !new_ballot.adjudicators.contains(uuid)) {
                new_ballot.government.scores.remove(deleted_adjudicator);
                new_ballot.opposition.scores.remove(deleted_adjudicator);
                new_ballot.speeches.iter_mut().for_each(|speech| {
                    speech.scores.remove(deleted_adjudicator);
                });
            }

            new_ballot.president = if let Some(president) = &debate.president {Some(president.adjudicator.uuid)} else {None};

            groups.add(Entity::Ballot(new_ballot));
        }

        Ok(groups)
    }
}