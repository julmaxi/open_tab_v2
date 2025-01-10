use std::collections::HashMap;

use itertools::{Itertools, izip};
use async_trait::async_trait;
use open_tab_entities::{domain::entity::LoadEntity, prelude::*, schema::{self, adjudicator::Column}};

use sea_orm::{prelude::*, JoinType, QuerySelect, RelationBuilder};
use thiserror::Error;

use crate::draw_view::{DrawBallot, DrawDebate};
use serde::{Serialize, Deserialize};

use super::ActionTrait;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateDrawAction {
    pub tournament_id: Uuid,
    #[serde(default)]
    pub updated_ballots: Vec<DrawBallot>,
    #[serde(default)]
    pub updated_debates: Vec<DrawDebate>
}

#[derive(Debug, Error)]
pub enum UpdateDrawError {
    #[error("Debate {0} not found")]
    DebateNotFound(Uuid),
}


#[async_trait]
impl ActionTrait for UpdateDrawAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
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

        let mut groups = EntityGroup::new(self.tournament_id);

        for (ballot, debate) in izip![ballots, self.updated_ballots.iter()] {
            let mut new_ballot = ballot.clone();

            new_ballot.government.team = if let Some(gov) = &debate.government { Some(gov.uuid) } else { None };
            new_ballot.opposition.team = if let Some(opp) = &debate.opposition { Some(opp.uuid) } else { None };

            let mut existing_non_aligned_speeches = new_ballot.speeches.iter_mut().filter(|speech| speech.role == open_tab_entities::domain::ballot::SpeechRole::NonAligned).collect_vec();

            let mut new_speeches = vec![];
            for (i, speech) in debate.non_aligned_speakers.iter().enumerate() {
                if i < existing_non_aligned_speeches.len() {
                    existing_non_aligned_speeches[i].speaker = speech.as_ref().map(|s| s.uuid);
                } else {
                    new_speeches.push(Speech {
                        speaker: speech.as_ref().map(|s| s.uuid),
                        role: open_tab_entities::domain::ballot::SpeechRole::NonAligned,
                        position: i as u8,
                        scores: HashMap::new(),
                        is_opt_out: false,
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

        let relevant_debates = open_tab_entities::domain::debate::TournamentDebate::get_many(db, self.updated_debates.iter().map(|d| d.uuid).collect()).await?.into_iter().map(|d| (d.uuid, d)).collect::<HashMap<_, _>>();

        let relevant_venues_by_round = self.updated_debates.iter().filter(|d| d.venue.is_some()).map(|d| (relevant_debates.get(&d.uuid), d.venue.as_ref().unwrap().uuid)).filter_map(
            |(d, v)| match d {
                Some(d) => Some((v, d.round_id)),
                None => None
            }
        ).collect_vec();

        let mut potentially_changed_debates = open_tab_entities::schema::tournament_debate::Entity::find()
            .filter(
                // Overselects, but sea orm does not support tuple condidtions.
                open_tab_entities::schema::tournament_debate::Column::VenueId.is_in(relevant_venues_by_round.iter().map(|(v, _)| v.clone()).collect_vec())
                .and(
                    open_tab_entities::schema::tournament_debate::Column::RoundId.is_in(relevant_venues_by_round.iter().map(|(_, r)| r.clone()).collect_vec())
                )
            )
            .all(db).await?.into_iter().map(|d| ((d.venue_id.unwrap(), d.round_id), d)).collect::<HashMap<_, _>>();

        for debate in self.updated_debates {
            let mut existing_debate = open_tab_entities::domain::debate::TournamentDebate::try_get(
                db,
                debate.uuid
            ).await?.ok_or(UpdateDrawError::DebateNotFound(debate.uuid))?;

            match debate.venue {
                Some(venue) => {
                    let prev_venue_debate = potentially_changed_debates.remove(&(venue.uuid, existing_debate.round_id));
                    if let Some(mut prev_venue_debate) = prev_venue_debate {
                        if prev_venue_debate.uuid != existing_debate.uuid {
                            prev_venue_debate.venue_id = None;

                            groups.add(Entity::TournamentDebate(
                                TournamentDebate::from_model(prev_venue_debate)
                            ));
                        }
                    }

                    existing_debate.venue_id = Some(venue.uuid);
                },
                None => {
                    existing_debate.venue_id = None;
                }
            }

            groups.add(Entity::TournamentDebate(existing_debate));
        }
        Ok(groups)
    }
}