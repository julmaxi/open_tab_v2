use itertools::izip;
use open_tab_entities::domain::ballot::Ballot;
use open_tab_entities::domain::entity::LoadEntity;
use open_tab_entities::schema;
use sea_orm::ActiveValue;
use sea_orm::QueryOrder;
use sea_orm::prelude::Uuid;
use std::{collections::HashMap, error::Error};

use migration::async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use open_tab_entities::prelude::*;

use open_tab_entities::domain;

use itertools::Itertools;

use crate::LoadedView;
use crate::TournamentParticipantsInfo;

pub struct LoadedRoundResultsView {
    pub view: RoundResultsView,
    pub round_id: Uuid
}

impl LoadedRoundResultsView {
    pub async fn load<C>(db: &C, round_uuid: Uuid) -> Result<Self, Box<dyn Error>> where C: ConnectionTrait {
        Ok(
            LoadedRoundResultsView {
                round_id: round_uuid,
                view: RoundResultsView::load(db, round_uuid).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedRoundResultsView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, Box<dyn Error>> {
        if changes.tournament_debates.len() > 0 || changes.ballots.len() > 0 || changes.teams.len() > 0 || changes.participants.len() > 0 {
            println!("Refreshing round results view {}", self.round_id);
            self.view = RoundResultsView::load(db, self.round_id).await?;

            let mut out = HashMap::new();
            out.insert(".".to_string(), serde_json::to_value(&self.view)?);
            println!("Done round results view {}", self.round_id);

            Ok(Some(out))
        }
        else {
            Ok(None)
        }
    }

    async fn view_string(&self) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::to_string(&self.view)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundResultsView {
    debates: Vec<ResultDebate>
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultDebate {
    uuid: Uuid,
    name: String,
    backup_ballots: Vec<BackupBallot>,
    ballot: DisplayBallot
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupBallot {
    name: String,
    uuid: Uuid,
    ballot_uuid: Uuid,
    ballot: DisplayBallot
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayBallot {
    pub uuid: Uuid,

    pub adjudicators: Vec<DisplayAdjudicator>,
    pub government: DisplayBallotTeam,
    pub opposition: DisplayBallotTeam,

    pub speeches: Vec<DisplayBallotSpeech>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayAdjudicator {
    pub uuid: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DisplayBallotTeam {
    pub uuid: Option<Uuid>,
    pub name: Option<String>,
    pub members: Vec<DisplaySpeaker>,
    pub scores: HashMap<Uuid, i16>,
    pub total_team_score: Option<f64>,
    pub total_speech_score: Option<f64>,
    pub total_score: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplaySpeaker {
    pub uuid: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayBallotSpeech {
    pub scores: HashMap<Uuid, i16>,
    pub speaker: Option<DisplaySpeaker>,
    pub position: u8,
    pub role: SpeechRole,
    pub total_score: Option<f64>,
}

impl DisplayBallot {
    fn from_ballot_and_info(ballot: Ballot, info: &TournamentParticipantsInfo) -> Self {
        let adjudicators = ballot.adjudicators.iter().map(|adjudicator| DisplayAdjudicator {
            uuid: *adjudicator,
            name: info.participants_by_id.get(adjudicator).map(|adj| adj.name.clone()).unwrap_or("Unknown".into())
        }).collect_vec();

        let government = DisplayBallotTeam {
            uuid: ballot.government.team,
            name: ballot.government.team.map(|team| info.teams_by_id.get(&team)).flatten().map(|team| team.name.clone()),
            members: ballot.government.team.map(|team| {
                info.team_members.get(&team).unwrap().iter().map(|member|
                    DisplaySpeaker {
                        uuid: *member,
                        name: info.participants_by_id.get(member).map(|m| m.name.clone()).unwrap_or("Unknown".into())
                    }).collect_vec()
            }).into_iter().flatten().collect_vec(),
            scores: ballot.government.scores.iter().map(|scores| (*scores.0, scores.1.total())).collect(),
            total_team_score: ballot.government.team_score(),
            total_speech_score: ballot.government_speech_total(),
            total_score: ballot.government_total()
        };

        let opposition = DisplayBallotTeam {
            uuid: ballot.opposition.team,
            name: ballot.opposition.team.map(|team| info.teams_by_id.get(&team)).flatten().map(|team| team.name.clone()),
            members: ballot.opposition.team.map(|team| {
                info.team_members.get(&team).unwrap().iter().map(|member|
                    DisplaySpeaker {
                        uuid: *member,
                        name: info.participants_by_id.get(member).map(|m| m.name.clone()).unwrap_or("Unknown".into())
                    }).collect_vec()
            }).into_iter().flatten().collect_vec(),
            scores: ballot.opposition.scores.iter().map(|scores| (*scores.0, scores.1.total())).collect(),
            total_team_score: ballot.opposition.team_score(),
            total_speech_score: ballot.opposition_speech_total(),
            total_score: ballot.opposition_total()
        };

        let speeches = ballot.speeches.iter().map(|speech| DisplayBallotSpeech {
            scores: speech.scores.iter().map(|scores| (*scores.0, scores.1.total())).collect(),
            speaker: speech.speaker.map(|speaker| DisplaySpeaker {
                uuid: speaker,
                name: info.participants_by_id.get(&speaker).map(|s| s.name.clone()).unwrap_or("Unknown".into())
            }),
            position: speech.position,
            role: speech.role,
            total_score: speech.speaker_score()
        }).collect_vec();

        DisplayBallot {
            uuid: ballot.uuid,
            adjudicators,
            government,
            opposition,
            speeches
        }
    }
}

impl Into<Ballot> for DisplayBallot {
    fn into(self) -> Ballot {
        dbg!(&self);
        let adjudicators = self.adjudicators.into_iter().map(|adj| adj.uuid).collect_vec();
        let government = BallotTeam {
            team: self.government.uuid,
            scores: self.government.scores.into_iter().map(|(adj, score)| (adj, TeamScore::Aggregate(score))).collect()
        };
        let opposition = BallotTeam {
            team: self.opposition.uuid,
            scores: self.opposition.scores.into_iter().map(|(adj, score)| (adj, TeamScore::Aggregate(score))).collect()
        };
        let speeches = self.speeches.into_iter().map(|speech| Speech {
            speaker: speech.speaker.map(|speaker| speaker.uuid),
            position: speech.position,
            role: speech.role,
            scores: speech.scores.into_iter().map(|(adj, score)| (adj, SpeakerScore::Aggregate(score))).collect()
        }).collect_vec();

        Ballot {
            uuid: self.uuid,
            adjudicators,
            government,
            opposition,
            speeches,
            president: None
        }
    }
}

impl RoundResultsView {
    async fn load<C>(db: &C, round_uuid: Uuid) -> Result<Self, Box<dyn Error>> where C: ConnectionTrait {
        let round = schema::tournament_round::Entity::find().filter(schema::tournament_round::Column::Uuid.eq(round_uuid)).one(db).await?.ok_or("Round not found")?;
        let info = TournamentParticipantsInfo::load(db, round.tournament_id).await?;

        let debates = schema::tournament_debate::Entity::find()
            .filter(schema::tournament_debate::Column::RoundId.eq(round.uuid))
            .order_by_asc(schema::tournament_debate::Column::Index)
            .all(db)
            .await?;

        let backup_ballots = schema::debate_backup_ballot::Entity::find()
            .filter(schema::debate_backup_ballot::Column::DebateId.is_in(debates.iter().map(|debate| debate.uuid).collect_vec()))
            .all(db)
            .await?;

        let all_ballot_uuids = debates.iter().map(|debate| debate.ballot_id).chain(backup_ballots.iter().map(|ballot| ballot.ballot_id)).collect_vec();

        let all_ballots_by_id : HashMap<_, _> = Ballot::get_many(db, all_ballot_uuids).await?.into_iter().map(|ballot| (ballot.uuid, DisplayBallot::from_ballot_and_info(ballot, &info))).collect();

        let out_debates: Result<Vec<_>, Box<dyn Error>> = debates.into_iter().map(
            |debate| {
                let backup_ballots: Result<Vec<BackupBallot>, Box<dyn Error>> = backup_ballots.iter().filter(|ballot| ballot.debate_id == debate.uuid).map(|ballot| Ok(BackupBallot {
                    name: ballot.timestamp.to_string(),
                    uuid: ballot.uuid,
                    ballot_uuid: ballot.ballot_id,
                    ballot: all_ballots_by_id.get(&ballot.ballot_id).ok_or("Ballot not found")?.clone()
                })).collect();
                Ok(ResultDebate {
                    uuid: debate.uuid,
                    name: format!("Debate {}", debate.index + 1),
                    backup_ballots: backup_ballots?,
                    ballot: all_ballots_by_id.get(&debate.ballot_id).cloned().ok_or("Ballot not found")?
                })
            }
        ).collect();

        Ok(RoundResultsView {
            debates: out_debates?
        })
    }
}