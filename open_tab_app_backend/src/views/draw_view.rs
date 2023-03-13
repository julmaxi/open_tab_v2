use std::fmt::Display;
use std::{collections::HashMap, error::Error};

use migration::async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use open_tab_entities::prelude::*;

use open_tab_entities::schema::{self, tournament_round};

use itertools::izip;
use itertools::Itertools;


#[async_trait]
pub trait LoadedView : Sync + Send {
    // We can't use a connection trait here, since otherwise the trait is not object safe
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroups) -> Result<Option<HashMap<String, serde_json::Value>>, Box<dyn Error>>;
    async fn view_string(&self) -> Result<String, Box<dyn Error>>;
}

pub struct LoadedDrawView {
    pub view: DrawView,
    pub tournament_id: Uuid
    //TODO: Use this to cache team and participant names
    //to avoid a full reload every time
    //Alternatively, it would be interesting to try to implement
    //dependent views.
}

impl LoadedDrawView {
    pub async fn load<C>(db: &C, round_uuid: Uuid) -> Result<LoadedDrawView, Box<dyn Error>> where C: ConnectionTrait {
        let round = schema::tournament_round::Entity::find_by_id(round_uuid).one(db).await?.ok_or(DrawViewError::MissingDebate)?;

        Ok(
            LoadedDrawView {
                tournament_id: round.tournament_id,
                view: DrawView::load_from_round(db, round).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedDrawView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroups) -> Result<Option<HashMap<String, serde_json::Value>>, Box<dyn Error>> {
        // TODO: We assume, the debate index never changes, even though it could in theory
        let changed_debates_by_id : HashMap<_, _> = changes.debates.iter().map(|d| (d.uuid, d)).collect();
        let changed_ballots_by_id : HashMap<_, _> = changes.ballots.iter().map(|b| (b.uuid, b)).collect();
        let mut indices_to_reload : Vec<usize> = vec![];

        for (idx, debate) in self.view.debates.iter_mut().enumerate() {
            let is_debate_changed = changed_debates_by_id.contains_key(&debate.uuid);
            if is_debate_changed || changed_ballots_by_id.contains_key(&debate.ballot.uuid) {
                indices_to_reload.push(idx);

                if is_debate_changed {
                    debate.ballot.uuid = debate.uuid; 
                }
            }
        }

        if indices_to_reload.len() > 0 {
            let info = TournamentParticipantsInfo::load(db, self.tournament_id).await?;
            let mut out : HashMap<String, serde_json::Value> = HashMap::new();
            let ballot_uuids = indices_to_reload.iter().map(|idx| {self.view.debates[*idx].ballot.uuid}).collect_vec();
            let ballots = Ballot::get_many(db, ballot_uuids).await?;

            for (idx, ballot) in izip!(indices_to_reload, ballots) {
                self.view.debates[idx].ballot = DrawView::draw_ballot_from_debate_ballot(&ballot, &info);

                out.insert(format!("debates.{}.ballot", idx), serde_json::to_value(&self.view.debates[idx].ballot)?);
            }

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
pub struct DrawView {
    round_uuid: Uuid,
    debates: Vec<DrawDebate>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawDebate {
    pub uuid: Uuid,
    pub index: usize,
    pub ballot: DrawBallot
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawBallot {
    pub uuid: Uuid,
    pub government: Option<DrawTeam>,
    pub opposition: Option<DrawTeam>,
    pub non_aligned_speakers: Vec<DrawSpeaker>,
    pub adjudicators: Vec<DrawAdjudicator>,
    pub president: Option<DrawAdjudicator>
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrawTeam {
    pub uuid: Uuid,
    pub name: String,
    pub members: Vec<DrawSpeaker>
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrawSpeaker {
    pub uuid: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrawAdjudicator {
    pub uuid: Uuid,
    pub name: String,
}

#[derive(Debug)]
enum DrawViewError {
    MissingDebate
}

impl Display for DrawViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for DrawViewError {
}


struct TournamentParticipantsInfo {
    participants_by_id: HashMap<Uuid, Participant>,
    teams_by_id: HashMap<Uuid, Team>,
    team_members: HashMap<Uuid, Vec<Uuid>>
}

impl TournamentParticipantsInfo {
    async fn load<C>(db: &C, tournament_id: Uuid) -> Result<Self, Box<dyn Error>> where C: ConnectionTrait {
        let all_participants = Participant::get_all_in_tournament(db, tournament_id).await?;
        let team_members = all_participants.iter().filter_map(|speaker| {
            if let ParticipantRole::Speaker(speaker_info) = &speaker.role {
                if let Some(team_uuid) = speaker_info.team_id {
                    Some((team_uuid, speaker.uuid))
                }
                else {
                    None
                }
            }
            else {
                None
            }
        }).into_group_map();
        let teams_by_id = Team::get_all_in_tournament(db, tournament_id).await?.into_iter().map(|team| (team.uuid, team)).collect::<HashMap<_, _>>();
        let participants_by_id = all_participants.into_iter().map(|speaker| (speaker.uuid, speaker)).collect::<HashMap<_, _>>();

        Ok(Self {
            participants_by_id,
            teams_by_id,
            team_members
        })
    }
}

/*

let all_participants = Participant::get_all_in_tournament(db, round.tournament_id).await?;
let team_members = all_participants.iter().filter_map(|speaker| {
    if let ParticipantRole::Speaker(speaker_info) = &speaker.role {
        if let Some(team_uuid) = speaker_info.team_id {
            Some((team_uuid, speaker.uuid))
        }
        else {
            None
        }
    }
    else {
        None
    }
}).into_group_map();
let teams_by_id = Team::get_all_in_tournament(db, round.tournament_id).await?.into_iter().map(|team| (team.uuid, team)).collect::<HashMap<_, _>>();
let participants_by_id = all_participants.into_iter().map(|speaker| (speaker.uuid, speaker)).collect::<HashMap<_, _>>();

 */

impl DrawView {
    fn draw_speaker_from_uuid(speaker_uuid: Uuid, info: &TournamentParticipantsInfo) -> DrawSpeaker {
        let speaker = info.participants_by_id.get(&speaker_uuid).unwrap();
        DrawSpeaker {
            uuid: speaker.uuid,
            name: speaker.name.clone()
        }
    }

    fn draw_team_from_ballot_team(team: &BallotTeam, info: &TournamentParticipantsInfo) -> Option<DrawTeam> {
        if let Some(team_uuid) = team.team {
            Some(DrawTeam {
                uuid: team_uuid,
                name: info.teams_by_id.get(&team_uuid).unwrap().name.clone(),
                members: info.team_members.get(&team_uuid).unwrap().iter().map(|speaker_uuid| {
                    Self::draw_speaker_from_uuid(*speaker_uuid, info)
                }).collect()
            })
        }
        else {
            None
        }
    }

    fn draw_adjudicator_from_uuid(adjudicator_uuid: Uuid, info: &TournamentParticipantsInfo) -> DrawAdjudicator {
        let adjudicator = info.participants_by_id.get(&adjudicator_uuid).unwrap();
        DrawAdjudicator {
            uuid: adjudicator.uuid,
            name: adjudicator.name.clone()
        }
    }

    fn draw_ballot_from_debate_ballot(
        ballot: &Ballot,
        info: &TournamentParticipantsInfo
    ) -> DrawBallot {
        DrawBallot {
            uuid: ballot.uuid,
            government: Self::draw_team_from_ballot_team(&ballot.government, info),
            opposition: Self::draw_team_from_ballot_team(&ballot.opposition, info),
            non_aligned_speakers: ballot.speeches.iter().filter_map(|speech| {
                if speech.role == SpeechRole::NonAligned {
                    if let Some(speaker_uuid) = speech.speaker {
                        Some(Self::draw_speaker_from_uuid(speaker_uuid, info))
                    }
                    else {
                        None
                    }
                }
                else {
                    None
                }
            }).collect(),
            adjudicators: ballot.adjudicators.iter().map(|adjudicator_uuid| {
                Self::draw_adjudicator_from_uuid(*adjudicator_uuid, info)
            }).collect(),
            president: ballot.president.map(|president_uuid| {
                Self::draw_adjudicator_from_uuid(president_uuid, info)
            })
        }
    }

    pub async fn load<C>(db: &C, round_uuid: Uuid) -> Result<DrawView, Box<dyn Error>> where C: ConnectionTrait {
        let round = schema::tournament_round::Entity::find_by_id(round_uuid).one(db).await?.ok_or(DrawViewError::MissingDebate)?;

        return Self::load_from_round(db, round).await;
    }

    async fn load_from_round<C>(db: &C, round: tournament_round::Model) -> Result<DrawView, Box<dyn Error>> where C: ConnectionTrait {
        let debates = schema::tournament_debate::Entity::find().filter(schema::tournament_debate::Column::RoundId.eq(round.uuid)).all(db).await?;

        let ballot_uuids = debates.iter().map(|debate| debate.ballot_id).collect_vec();

        let ballots = Ballot::get_many(db, ballot_uuids).await?;

        // FIXME: This will fail if a participant is missing
        // from the tournament.
        let participant_info = TournamentParticipantsInfo::load(db, round.tournament_id).await?;

        let debates = izip![debates, ballots.into_iter()].map(
            |(debate, debate_ballot)| {
                DrawDebate {
                    uuid: debate.uuid,
                    index: debate.index as usize,
                    ballot: Self::draw_ballot_from_debate_ballot(&debate_ballot, &participant_info)
                }
            }
        ).collect();

        Ok(DrawView { round_uuid: round.uuid, debates })
    }
}