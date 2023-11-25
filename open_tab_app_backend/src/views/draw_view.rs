use std::collections::HashSet;




use std::collections::HashMap;

use async_trait::async_trait;
use open_tab_entities::domain::entity::LoadEntity;
use open_tab_entities::domain::tournament_venue::TournamentVenue;
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use open_tab_entities::{prelude::*, EntityType};

use open_tab_entities::schema::{self, tournament_round};

use itertools::izip;
use itertools::Itertools;



use crate::draw::evaluation::{DrawEvaluator, DrawIssue};
use crate::tab_view::TeamRoundRole;

use super::base::{LoadedView, TournamentParticipantsInfo};


pub struct LoadedDrawView {
    pub view: DrawView,
    pub tournament_id: Uuid
    //TODO: Use this to cache team and participant names
    //to avoid a full reload every time
    //Alternatively, it would be interesting to try to implement
    //dependent views.
}

impl LoadedDrawView {
    pub async fn load<C>(db: &C, round_uuid: Uuid) -> Result<LoadedDrawView, anyhow::Error> where C: sea_orm::ConnectionTrait {
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
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        // TODO: We assume, the debate index never changes, even though it could in theory
        let changed_debates_by_id : HashMap<_, _> = changes.tournament_debates.iter().map(|d| (d.uuid, d)).collect();
        let changed_ballots_by_id : HashMap<_, _> = changes.ballots.iter().map(|b| (b.uuid, b)).collect();
        let mut indices_to_reload : Vec<usize> = vec![];

        let know_debate_uuids = self.view.debates.iter().map(|d| d.uuid).collect::<HashSet<_>>();
        let has_new_debate = changed_debates_by_id.iter().any(
            |(uuid, _)| !know_debate_uuids.contains(uuid)
        );

        // TODO: Reloads could be much more efficient
        if has_new_debate || changes.tournament_venues.len() > 0 || changes.participants.len() > 0 || changes.participant_clashs.len() > 0 || changes.deletions.iter().any(|d| d.0 == EntityType::TournamentVenue) || changes.deletions.iter().any(|d| d.0 == EntityType::Participant) || changes.deletions.iter().any(|d| d.0 == EntityType::ParticipantClash) {
            let mut out: HashMap<String, Json> = HashMap::new();
            let round = schema::tournament_round::Entity::find_by_id(self.view.round_uuid).one(db).await?.ok_or(DrawViewError::MissingDebate)?;
            self.view = DrawView::load_from_round(db, round).await?;
            out.insert(".".to_string(), serde_json::to_value(&self.view)?);
    
            return Ok(Some(out))
        }
        
        for (idx, debate) in self.view.debates.iter_mut().enumerate() {
            let is_debate_changed = changed_debates_by_id.contains_key(&debate.uuid);
            if is_debate_changed || changed_ballots_by_id.contains_key(&debate.ballot.uuid) {
                indices_to_reload.push(idx);

                if is_debate_changed {
                    debate.ballot.uuid = changed_debates_by_id.get(&debate.uuid).unwrap().ballot_id; 
                }
            }
        }
        if indices_to_reload.len() > 0 {
            let evaluator = DrawEvaluator::new_from_other_rounds(db, self.tournament_id, self.view.round_uuid).await?;

            let info = TournamentParticipantsInfo::load(db, self.tournament_id).await?;
            let mut out : HashMap<String, serde_json::Value> = HashMap::new();
            let ballot_uuids = indices_to_reload.iter().map(|idx| {self.view.debates[*idx].ballot.uuid}).collect_vec();
            let ballots = Ballot::get_many(db, ballot_uuids).await?;
            let debate_uuids = indices_to_reload.iter().map(|idx| {self.view.debates[*idx].uuid}).collect_vec();
            let debates = TournamentDebate::get_many(db, debate_uuids).await?;
            let debate_venue_ids = debates.iter().filter_map(|d| if let Some(venue_id) = d.venue_id {Some((d.uuid, venue_id))} else {None}).collect_vec();
            let venues = TournamentVenue::get_many(db, debate_venue_ids.iter().map(|x| x.1).collect_vec()).await?.into_iter().map(|v| (v.uuid, v)).collect::<HashMap<_, _>>();

            for (idx, ballot, debate) in izip!(indices_to_reload, ballots, debates) {
                self.view.debates[idx].ballot = DrawView::draw_ballot_from_debate_ballot(&ballot, &info, &evaluator, self.view.round_uuid);
                self.view.debates[idx].venue = debate.venue_id.map(|venue_id| venues.get(&venue_id).cloned().map(DrawVenue::from)).flatten();

                out.insert(format!("debates.{}.ballot", idx), serde_json::to_value(&self.view.debates[idx].ballot)?);
                out.insert(format!("debates.{}.venue", idx), serde_json::to_value(&self.view.debates[idx].venue)?);
            }

            let index = DrawView::construct_adjudicator_index(&info, &self.view.debates, self.view.round_uuid);
            out.insert("adjudicator_index".into(), serde_json::to_value(&index)?);
            self.view.adjudicator_index = index;

            let team_index = DrawView::construct_team_index(&info, &self.view.debates);
            out.insert("team_index".into(), serde_json::to_value(&team_index)?);
            self.view.team_index = team_index;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawView {
    round_uuid: Uuid,
    debates: Vec<DrawDebate>,
    adjudicator_index: Vec<AdjudicatorIndexEntry>,
    team_index: Vec<TeamIndexEntry>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AdjudicatorIndexEntry {
    adjudicator: DrawAdjudicator,
    is_available: bool,
    position: AdjudictorPosition
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct TeamIndexEntry {
    team: DrawTeam,
    position: TeamPosition
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="type")]
enum TeamPosition {
    Team { debate_uuid: Uuid, debate_index: usize, role: TeamRoundRole },
    NonAligned { member_positions: HashMap<Uuid, SpeakerPosition> },
    NotSet
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpeakerPosition {
    debate_uuid: Uuid,
    debate_index: usize,
    position: usize
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum AdjudicatorPositionRole {
    President,
    Panel {position: usize}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum AdjudictorPosition {
    NotSet,
    Set {
        debate_uuid: Uuid,
        debate_index: usize,
        position: AdjudicatorPositionRole
    },
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawDebate {
    pub uuid: Uuid,
    pub index: usize,
    pub ballot: DrawBallot,
    pub venue: Option<DrawVenue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrawBallot {
    pub uuid: Uuid,
    pub government: Option<DrawTeam>,
    pub opposition: Option<DrawTeam>,
    pub non_aligned_speakers: Vec<DrawSpeaker>,
    pub adjudicators: Vec<SetDrawAdjudicator>,
    pub president: Option<SetDrawAdjudicator>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawVenue {
    pub uuid: Uuid,
    pub name: String,
}

impl From<TournamentVenue> for DrawVenue {
    fn from(v: TournamentVenue) -> Self {
        DrawVenue {
            uuid: v.uuid,
            name: v.name
        }
    }
}

impl Into<Ballot> for DrawBallot {
    fn into(self) -> Ballot {
        let mut speeches = vec![
            (open_tab_entities::domain::ballot::SpeechRole::Government),
            (open_tab_entities::domain::ballot::SpeechRole::Opposition),
        ].into_iter().flat_map(
            |role| {
                (0..3).map(
                    move |position| Speech {
                        speaker: None,
                        role,
                        position,
                        scores: HashMap::new(),
                    }
                )
            }
        ).collect_vec();
        speeches.extend(
            self.non_aligned_speakers.into_iter().enumerate().map(
                |(idx, u)| Speech {
                    speaker: Some(u.uuid),
                    role: SpeechRole::NonAligned,
                    position: idx as u8,
                    scores: HashMap::new()
                }
            )
        );
        Ballot {
            uuid: self.uuid,
            government: BallotTeam { team: self.government.map(|t| t.uuid), scores: HashMap::new() },
            opposition: BallotTeam { team: self.opposition.map(|t| t.uuid), scores: HashMap::new() },
            speeches,
            adjudicators: self.adjudicators.into_iter().map(|a| a.adjudicator.uuid).collect(),
            president: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrawTeam {
    pub uuid: Uuid,
    pub name: String,
    pub members: Vec<DrawSpeaker>,
    #[serde(skip_deserializing)]
    pub issues: Vec<DrawIssue>
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrawSpeaker {
    pub uuid: Uuid,
    pub name: String,
    pub team_id: Option<Uuid>,
    pub team_name: Option<String>,
    pub institutions: Vec<DrawInstitution>,
    #[serde(skip_deserializing)]
    pub issues: Vec<DrawIssue>
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrawInstitution {
    pub uuid: Uuid,
    pub name: String
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SetDrawAdjudicator {
    #[serde(flatten)]
    pub adjudicator: DrawAdjudicator,
    #[serde(skip_deserializing)]
    pub issues: Vec<DrawIssue>,
    #[serde(skip_deserializing)]
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrawAdjudicator {
    pub uuid: Uuid,
    pub name: String,
    pub institutions: Vec<DrawInstitution>,
}

impl From<DrawAdjudicator> for SetDrawAdjudicator {
    fn from(adjudicator: DrawAdjudicator) -> Self {
        SetDrawAdjudicator {
            adjudicator,
            issues: vec![],
            is_available: true
        }
    }
}


#[derive(Debug, thiserror::Error)]
enum DrawViewError {
    #[error("Missing debate")]
    MissingDebate
}



impl DrawView {
    fn draw_speaker_from_uuid(speaker_uuid: Uuid, info: &TournamentParticipantsInfo) -> DrawSpeaker {
        let speaker = info.participants_by_id.get(&speaker_uuid).unwrap();
        let team = info.speaker_teams.get(&speaker.uuid);

        DrawSpeaker {
            uuid: speaker.uuid,
            name: speaker.name.clone(),
            team_id: team.map(|t| *t),
            team_name: team.map(|t| info.teams_by_id.get(t).map(|t| t.name.clone())).flatten(),
            issues: vec![],
            institutions: speaker.institutions.iter().map(|i| DrawInstitution {
                uuid: i.uuid,
                name: info.institutions_by_id.get(&i.uuid).map(|i| i.name.clone()).unwrap_or_else(|| "Unknown".to_string())
            }).collect()
        }
    }

    fn draw_team_from_ballot_team(team: &BallotTeam, info: &TournamentParticipantsInfo) -> Option<DrawTeam> {
        if let Some(team_uuid) = team.team {
            Some(Self::draw_team_from_uuid(team_uuid, info))
        }
        else {
            None
        }
    }

    fn draw_team_from_uuid(team_uuid: Uuid, info: &TournamentParticipantsInfo) -> DrawTeam {
        DrawTeam {
            uuid: team_uuid,
            name: info.teams_by_id.get(&team_uuid).unwrap().name.clone(),
            members: info.team_members.get(&team_uuid).unwrap().iter().map(|speaker_uuid| {
                Self::draw_speaker_from_uuid(
                    *speaker_uuid, info
                )
            }).collect(),
            issues: vec![]
        }
    }

    fn draw_adjudicator_from_uuid(adjudicator_uuid: Uuid, info: &TournamentParticipantsInfo) -> DrawAdjudicator {
        let adjudicator = info.participants_by_id.get(&adjudicator_uuid).unwrap();

        DrawAdjudicator {
            uuid: adjudicator.uuid,
            name: adjudicator.name.clone(),
            institutions: adjudicator.institutions.iter().map(|i| DrawInstitution {
                uuid: i.uuid,
                name: info.institutions_by_id.get(&i.uuid).map(|i| i.name.clone()).unwrap_or_else(|| "Unknown".to_string())
            }).collect(),
        }
    }

    fn draw_ballot_from_debate_ballot(
        ballot: &Ballot,
        info: &TournamentParticipantsInfo,
        evaluator: &DrawEvaluator,
        round_id: Uuid,
    ) -> DrawBallot {
        let _all_ballot_participant_uuids = ballot.speeches.iter().filter_map(|speech| {
            if speech.role == SpeechRole::NonAligned {
                if let Some(speaker_uuid) = speech.speaker {
                    Some(speaker_uuid)
                }
                else {
                    None
                }
            }
            else {
                None
            }
        }).chain(
            ballot.adjudicators.iter().map(|adjudicator_uuid| *adjudicator_uuid)
        ).chain(
            ballot.government.team.iter().map(
                |team_uuid| info.team_members.get(team_uuid).unwrap_or(&vec![]).iter().map(|speaker_uuid| *speaker_uuid).collect_vec()
            ).flatten()
        ).chain(
            ballot.opposition.team.iter().map(
                |team_uuid| info.team_members.get(team_uuid).unwrap_or(&vec![]).iter().map(|speaker_uuid| *speaker_uuid).collect_vec()
            ).flatten()
        ).collect::<Vec<_>>();

        let mut ballot = DrawBallot {
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
                let mut adj : SetDrawAdjudicator = Self::draw_adjudicator_from_uuid(*adjudicator_uuid, info).into();
                adj.is_available = match info.participants_by_id.get(adjudicator_uuid) {
                    Some(participant) => {
                        match &participant.role {
                            ParticipantRole::Adjudicator (Adjudicator{ unavailable_rounds, .. }) => {
                                !unavailable_rounds.contains(&round_id)
                            },
                            _ => {
                                true
                            }
                        }
                    },
                    None => {
                        true
                    }
                };
                adj
            }).collect(),
            president: ballot.president.map(|president_uuid| {
                let mut adj : SetDrawAdjudicator = Self::draw_adjudicator_from_uuid(president_uuid, info).into();
                adj.is_available = match info.participants_by_id.get(&president_uuid) {
                    Some(participant) => {
                        match &participant.role {
                            ParticipantRole::Adjudicator (Adjudicator{ unavailable_rounds, .. }) => {
                                !unavailable_rounds.contains(&round_id)
                            },
                            _ => {
                                true
                            }
                        }
                    },
                    None => {
                        true
                    }
                };
                adj
            })
        };
        let ballot_evaluation = evaluator.find_issues_in_ballot(&ballot);


        if let Some(gov) = &mut ballot.government {
            gov.issues = ballot_evaluation.government_issues.clone();
        }

        if let Some(opp) = &mut ballot.opposition {
            opp.issues = ballot_evaluation.opposition_issues.clone();
        }

        ballot.adjudicators.iter_mut().for_each(|adjudicator| {
            adjudicator.issues = ballot_evaluation.adjudicator_issues.get(&adjudicator.adjudicator.uuid).unwrap_or(&vec![]).clone();
        });
        ballot.non_aligned_speakers.iter_mut().for_each(|speaker| {
            speaker.issues = ballot_evaluation.non_aligned_issues.get(&speaker.uuid).unwrap_or(&vec![]).clone();
        });

        ballot
    }

    pub async fn load<C>(db: &C, round_uuid: Uuid) -> Result<DrawView, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let round = schema::tournament_round::Entity::find_by_id(round_uuid).one(db).await?.ok_or(DrawViewError::MissingDebate)?;

        return Self::load_from_round(db, round).await;
    }

    fn construct_adjudicator_index(
        info: &TournamentParticipantsInfo,
        debates: &Vec<DrawDebate>,
        round_id: Uuid
    ) -> Vec<AdjudicatorIndexEntry> {
        let adjudicators = info.get_adjudicators();

        let mut adj_positions = HashMap::new();

        debates.iter().for_each(|debate| {
            debate.ballot.adjudicators.iter().enumerate().for_each(|(adj_idx, adjudicator)| {
                adj_positions.insert(adjudicator.adjudicator.uuid, AdjudictorPosition::Set {
                    debate_uuid: debate.uuid,
                    debate_index: debate.index,
                    position: AdjudicatorPositionRole::Panel { position: adj_idx }
                });
             });

            if let Some(president) = &debate.ballot.president {
                adj_positions.insert(president.adjudicator.uuid, AdjudictorPosition::Set {
                    debate_uuid: debate.uuid,
                    debate_index: debate.index,
                    position: AdjudicatorPositionRole::President
                });
            }
        });

        adjudicators.into_iter().map(
            |adj| {
                let draw_adj = Self::draw_adjudicator_from_uuid(adj.uuid, info);
                AdjudicatorIndexEntry {
                    is_available: match &adj.role {
                        ParticipantRole::Adjudicator(adj) => !adj.unavailable_rounds.contains(&round_id),
                        _ => true
                    },
                    adjudicator: draw_adj,
                    position: adj_positions.get(&adj.uuid).cloned().unwrap_or(AdjudictorPosition::NotSet)
                }
            }
        ).sorted_by(|e1, e2| e1.adjudicator.name.cmp(&e2.adjudicator.name)).collect()
    }

    fn construct_team_index(
        info: &TournamentParticipantsInfo,
        debates: &Vec<DrawDebate>,
    ) -> Vec<TeamIndexEntry> {
        let mut team_positions = HashMap::new();

        for team_id in info.teams_by_id.keys() {
            team_positions.insert(*team_id, TeamPosition::NotSet);
        }

        let inverse_team_map = info.team_members.iter().map(|(team_uuid, members)| {
            members.into_iter().map(|member_uuid| (member_uuid, team_uuid)).collect::<Vec<_>>()
        }).flatten().collect::<HashMap<_, _>>();

        debates.iter().for_each(|debate| {
            if let Some(gov) = &debate.ballot.government {
                team_positions.insert(
                    gov.uuid,
                    TeamPosition::Team {
                        debate_uuid: debate.uuid,
                        debate_index: debate.index,
                        role: TeamRoundRole::Government
                    },
                );
            }
            if let Some(opp) = &debate.ballot.opposition {
                team_positions.insert(
                    opp.uuid,
                    TeamPosition::Team {
                        debate_uuid: debate.uuid,
                        debate_index: debate.index,
                        role: TeamRoundRole::Opposition
                    },
                );
            }

            debate.ballot.non_aligned_speakers.iter().enumerate().for_each(|(position, speaker)| {
                let team = inverse_team_map.get(&speaker.uuid);
                if let Some(team_uuid) = team {
                    let mut prev_positions = team_positions.get_mut(team_uuid).unwrap();
                    match &mut prev_positions {
                        TeamPosition::NonAligned { member_positions } => {
                            member_positions.insert(
                                speaker.uuid,
                                SpeakerPosition {
                                    debate_uuid: debate.uuid,
                                    debate_index: debate.index,
                                    position
                                },
                            );
                        },
                        TeamPosition::NotSet => {
                            team_positions.insert(
                                **team_uuid,
                                TeamPosition::NonAligned {
                                    member_positions: HashMap::from_iter(vec![
                                        (
                                            speaker.uuid,
                                            SpeakerPosition {
                                                debate_uuid: debate.uuid,
                                                debate_index: debate.index,
                                                position
                                            }
                                        )
                                    ])
                                },
                            );
                        },
                        TeamPosition::Team { .. } => {
                            //TODO: Report Error
                        },
                    }
                }
            });

        });

        team_positions.into_iter().map(
            |(team_uuid, position)| {
                let draw_team = Self::draw_team_from_uuid(team_uuid, info);
                
                TeamIndexEntry {
                    team: draw_team,
                    position: position
                }
            }
        ).sorted_by(|e1, e2| e1.team.name.cmp(&e2.team.name)).collect()
    }

    async fn load_from_round<C>(db: &C, round: tournament_round::Model) -> Result<DrawView, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let debates = schema::tournament_debate::Entity::find().filter(schema::tournament_debate::Column::RoundId.eq(round.uuid)).all(db).await?;

        let ballot_uuids = debates.iter().map(|debate| debate.ballot_id).collect_vec();

        let ballots = Ballot::get_many(db, ballot_uuids).await?;

        // FIXME: This will fail if a participant is missing
        // from the tournament.
        let participant_info = TournamentParticipantsInfo::load(db, round.tournament_id).await?;

        let _clash_map = crate::draw::clashes::ClashMap::new_for_tournament(Default::default(), round.tournament_id, db).await?;

        //clash_map.add_dynamic_clashes_from_round_ballots(round_draws, &participant_info.team_members);
        //let evaluator = crate::draw::evaluation::DrawEvaluator::new(clash_map, Default::default());
        //let evaluator = DrawEvaluator::new_from_rounds(db, round.tournament_id, &other_rounds).await?;
        let evaluator = DrawEvaluator::new_from_other_rounds(db, round.tournament_id, round.uuid).await?;

        let debate_venue_ids = debates.iter().filter_map(|d| if let Some(venue_id) = d.venue_id {Some((d.uuid, venue_id))} else {None}).collect_vec();
        let venues = TournamentVenue::get_many(db, debate_venue_ids.iter().map(|x| x.1).collect_vec()).await?.into_iter().map(|v| (v.uuid, v)).collect::<HashMap<_, _>>();
        let debate_venues = venues.into_iter().map(|(venue_id, venue)| (venue_id, DrawVenue::from(venue))).collect::<HashMap<_, _>>();

        let debates = izip![debates, ballots.into_iter()].map(
            |(debate, debate_ballot)| {
                DrawDebate {
                    uuid: debate.uuid,
                    index: debate.index as usize,
                    ballot: Self::draw_ballot_from_debate_ballot(&debate_ballot, &participant_info, &evaluator, round.uuid),
                    venue: debate.venue_id.map(|id| debate_venues.get(&id).cloned()).flatten(),
                }
            }
        ).sorted_by_key(|d| d.index).collect();

        Ok(DrawView {
            adjudicator_index: Self::construct_adjudicator_index(&participant_info, &debates, round.uuid),
            team_index: Self::construct_team_index(&participant_info, &debates),
            round_uuid: round.uuid,
            debates
        })
    }
}