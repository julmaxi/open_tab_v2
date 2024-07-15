use std::{sync::Arc, collections::{HashSet, HashMap}, cmp::Ordering};

use itertools::{Itertools, izip, repeat_n};
use async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::{tournament_break::TournamentBreak, tournament_venue::TournamentVenue, tournament_plan_node::{TournamentPlanNode, RoundGroupConfig, PlanNodeType, BreakConfig}, entity::LoadEntity, tournament_plan_edge::TournamentPlanEdge, self, ballot}, EntityTypeId, tab::TeamRoundRole, derived_models::{BreakNodeBackgroundInfo, NodeExecutionError}};

use rand::{seq::SliceRandom, thread_rng, Rng};
use sea_orm::{prelude::*, QueryOrder};

use crate::{draw::{PreliminaryRoundGenerator, PreliminariesDrawMode, evaluation::DrawEvaluator, preliminary::{RoundGenerationContext, DrawTeamInfo}, tab_draw::{pair_teams, pair_speakers, TeamPair, assign_teams}, flow_optimization::{OptimizationState, OptimizationOptions}}, TournamentParticipantsInfo, draw_view::{DrawBallot, DrawTeam, DrawSpeaker, DrawAdjudicator, SetDrawAdjudicator}, views};
use serde::{Serialize, Deserialize};

use super::{ActionTrait, edit_tree::reindex_rounds};

use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum RedrawMode {
    Venues,
    MissingNonAligned
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedrawRoundAction {
    pub round_id: Uuid,
    #[serde(flatten)]
    pub mode: RedrawMode
}


#[async_trait]
impl ActionTrait for RedrawRoundAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let round = open_tab_entities::schema::tournament_round::Entity::find_by_id(self.round_id)
            .one(db)
            .await?;
        if round.is_none() {
            anyhow::bail!("Round not found");
        }
        let round = round.unwrap();

        match &self.mode {
            RedrawMode::Venues => {

                let debates = open_tab_entities::schema::tournament_debate::Entity::find()
                    .filter(open_tab_entities::schema::tournament_debate::Column::RoundId.eq(self.round_id))
                    .order_by_asc(open_tab_entities::schema::tournament_debate::Column::Index)
                    .all(db)
                    .await?;

                let venues = open_tab_entities::schema::tournament_venue::Entity::find()
                    .filter(
                        open_tab_entities::schema::tournament_venue::Column::TournamentId.eq(round.tournament_id)
                    )
                    .order_by_asc(
                        open_tab_entities::schema::tournament_venue::Column::OrderingIndex
                    )
                    .all(db)
                    .await?;
                let mut g = EntityGroup::new(
                    round.tournament_id
                );
                let mut venue_iter = venues.into_iter();
                for mut debate in debates.into_iter() {
                    debate.venue_id = venue_iter.next().map(|v| v.uuid);
                    g.add(Entity::TournamentDebate(
                        domain::debate::TournamentDebate::from_model(debate)
                    ));
                }

                Ok(g)
            },
            RedrawMode::MissingNonAligned => { 
                let debates = open_tab_entities::schema::tournament_debate::Entity::find()
                .filter(open_tab_entities::schema::tournament_debate::Column::RoundId.eq(self.round_id))
                .order_by_asc(open_tab_entities::schema::tournament_debate::Column::Index)
                .all(db)
                .await?;

                let ballot_ids = debates.iter().map(|d| d.ballot_id).collect::<Vec<_>>();

                let mut ballots = domain::ballot::Ballot::get_many(db, ballot_ids).await?;

                let mut non_aligned_teams = HashSet::new();
                let mut used_speakers = HashSet::new();

                let info = TournamentParticipantsInfo::load(db, round.tournament_id).await?;

                let mut g = EntityGroup::new(
                    round.tournament_id
                );

                for ballot in &ballots {
                    for speech in &ballot.speeches {
                        match &speech.role {
                            SpeechRole::NonAligned => {
                                if let Some(id) = &speech.speaker {
                                    used_speakers.insert(id);                                    
                                    non_aligned_teams.insert(info.speaker_teams[&id]);
                                }
                            }
                            _ => {}
                        }
                    }
                }

                let desired_non_aligned_speakers = non_aligned_teams.iter().flat_map(|t| info.team_members[t].iter()).collect_vec();//.flatten().cloned().collect_vec();

                let mut missing = desired_non_aligned_speakers.into_iter().filter(|s| !used_speakers.contains(s)).collect_vec();

                let mut rng = thread_rng();
                missing.shuffle(&mut rng);

                let mut missing_iter = missing.into_iter();

                for ballot in ballots.iter_mut() {
                    let mut did_change = false;

                    for speech in ballot.speeches.iter_mut() {
                        match &speech.role {
                            SpeechRole::NonAligned => {
                                if speech.speaker.is_none() {
                                    if let Some(speaker) = missing_iter.next() {
                                        speech.speaker = Some(speaker.clone());
                                        did_change = true;
                                    }
                                }
                            },
                            _ => {}
                        }
                    }

                    if did_change {
                        g.add(Entity::Ballot(ballot.clone()));
                    }
                }

                Ok(g)
            }
        }
    }
}