use std::{collections::HashMap, ops::{Add, AddAssign}};

use itertools::Itertools;
use open_tab_entities::{domain::{self, tournament_plan_node::PlanNodeType}, info::TournamentParticipantsInfo, schema, EntityGroup, EntityType};
use sea_orm::{prelude::Uuid, EntityOrSelect, EntityTrait, QueryFilter, QuerySelect, ColumnTrait};
use serde::Serialize;

use crate::{LoadedView, tournament_tree_view::get_round_names};



pub struct LoadedFeedbackProgressView {
    tournament_uuid: Uuid,
    view: FeedbackProgressView
}

impl LoadedFeedbackProgressView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Ok(
            Self {
                tournament_uuid,
                view: FeedbackProgressView::load(db, tournament_uuid).await?,
            }
        )
    }
}

#[async_trait::async_trait]
impl LoadedView for LoadedFeedbackProgressView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.feedback_forms.len() > 0 || changes.deletions.iter().any(|d| d.0 == EntityType::FeedbackForm) || changes.feedback_responses.len() > 0 || changes.tournament_debates.len() > 0 || changes.ballots.len() > 0 || changes.teams.len() > 0 || changes.participants.len() > 0 || changes.deletions.iter().any(|d| d.0 == EntityType::Participant) || changes.deletions.iter().any(|d| d.0 == EntityType::Team)  {
            self.view = FeedbackProgressView::load(db, self.tournament_uuid).await?;

            let mut out = HashMap::new();
            out.insert(".".to_string(), serde_json::to_value(&self.view)?);

            Ok(Some(out))
        }
        else {
            Ok(None)
        }
    }

    async fn view_string(&self) -> Result<String, anyhow::Error> {
        Ok(serde_json::to_string(&self.view)?)
    }
}


#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct FeedbackProgressView {
    adjudicator_feedback_info: Vec<ParticipantFeedbackInfo>,
    team_feedback_info: Vec<TeamFeedbackInfo>,
    rounds: Vec<FeedbackRound>
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct FeedbackRound {
    uuid: Uuid,
    name: String,
}


#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ParticipantFeedbackInfo {
    participant_id: Uuid,
    name: String,
    round_progress: HashMap<Uuid, FeedbackProgressInfo>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TeamFeedbackInfo {
    team_id: Uuid,
    name: String,
    round_progress: HashMap<Uuid, FeedbackProgressInfo>,
}


#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct FeedbackProgressInfo {
    submission_count: u32,
    submission_requirement: u32,
}


impl FeedbackProgressView {
    async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let feedback_progress_matrix = open_tab_entities::derived_models::feedback_progress::FeedbackProgressMatrix::from_tournament(db, tournament_uuid).await?;

        let speaker_info = TournamentParticipantsInfo::load(db, tournament_uuid).await?;

        let debate_rounds : Vec<(Uuid, Uuid)> = schema::tournament_debate::Entity::find()
            .select_only()
            .column(schema::tournament_debate::Column::Uuid)
            .column(schema::tournament_debate::Column::RoundId)
            .inner_join(schema::tournament_round::Entity)
            .filter(schema::tournament_round::Column::TournamentId.eq(tournament_uuid))
            .into_tuple()
            .all(db)
            .await?;
        let debate_rounds = debate_rounds.into_iter().collect::<HashMap<_, _>>();

        let rounds = schema::tournament_round::Entity::find()
            .filter(schema::tournament_round::Column::TournamentId.eq(tournament_uuid))
            .all(db)
            .await?;

        let mut adjudicator_rows = vec![];
        let empty = Vec::new();
        for participant in speaker_info.get_adjudicators() {
            let feedback_info = feedback_progress_matrix.submission_info_by_participant.get(&participant.uuid).unwrap_or(&empty);
            let mut required_submissions = HashMap::new();
            let mut submission_count = HashMap::new();

            for info in feedback_info {
                let round_id = debate_rounds.get(&info.debate_id).unwrap();
                required_submissions.entry(round_id).or_insert(0).add_assign(1);
                submission_count.entry(round_id).or_insert(0).add_assign((info.submissions.len() > 0) as u32);
            }

            adjudicator_rows.push(ParticipantFeedbackInfo {
                participant_id: participant.uuid,
                name: participant.name.clone(),
                round_progress: required_submissions.into_iter().map(|(round_id, required)| {
                    (*round_id, FeedbackProgressInfo {
                        submission_count: submission_count.get(&round_id).unwrap_or(&0).clone(),
                        submission_requirement: required,
                    })
                }).collect()
            });
        }

        let mut team_rows = vec![];

        for (team, members) in speaker_info.team_members {
            let feedback_info = feedback_progress_matrix.submission_info_by_team.get(&team).unwrap_or(&empty);

            let mut required_submissions = HashMap::new();
            let mut submission_count = HashMap::new();

            for info in feedback_info {
                let round_id = debate_rounds.get(&info.debate_id).unwrap();
                required_submissions.entry(round_id).or_insert(0).add_assign(1);
                submission_count.entry(round_id).or_insert(0).add_assign((info.submissions.len() > 0) as u32);
            }

            for member in members {
                let feedback_info = feedback_progress_matrix.submission_info_by_participant.get(&member).unwrap_or(&empty);
                for info in feedback_info {
                    let round_id = debate_rounds.get(&info.debate_id).unwrap();
                    required_submissions.entry(round_id).or_insert(0).add_assign(1);
                    submission_count.entry(round_id).or_insert(0).add_assign((info.submissions.len() > 0) as u32);
                }
            }

            team_rows.push(TeamFeedbackInfo {
                team_id: team,
                name: speaker_info.teams_by_id.get(&team).map(|t| t.name.clone()).unwrap_or("<Unknown Team>".into()),
                round_progress: required_submissions.into_iter().map(|(round_id, required)| {
                    (*round_id, FeedbackProgressInfo {
                        submission_count: submission_count.get(&round_id).unwrap_or(&0).clone(),
                        submission_requirement: required,
                    })
                }).collect()
            });
        }

        team_rows.sort_by(|a, b| a.name.cmp(&b.name));
        adjudicator_rows.sort_by(|a, b| a.name.cmp(&b.name));

        let round_info = rounds.into_iter().sorted_by_key(|r| r.index).map(|r| {
            FeedbackRound {
                uuid: r.uuid,
                name: format!("R. {}", r.index + 1),
            }
        }).collect();

        Ok(FeedbackProgressView {
            adjudicator_feedback_info: adjudicator_rows,
            team_feedback_info: team_rows,
            rounds: round_info,
        })
    }
}