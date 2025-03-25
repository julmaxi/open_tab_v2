use std::collections::{HashMap, HashSet};

use crate::{draw_view::DrawBallot, participants_list_view::Clash, TournamentParticipantsInfo};
use itertools::{izip, Itertools};
use open_tab_entities::{
    domain::{ballot::{self, BallotParseError}, entity::LoadEntity, participant_clash::ParticipantClash},
    prelude::{Ballot, Participant, ParticipantRole, SpeechRole, TournamentDebate, TournamentRound},
    schema::{self, adjudicator::Entity}, EntityGroup, EntityTypeId,
};
use sea_orm::{prelude::Uuid, ConnectionTrait, EntityTrait};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    clashes::{ClashType},
    datastructures::DebateInfo,
};

#[derive(Debug, Clone)]
pub struct DrawEvaluator<'a, C> {
    config: DrawEvaluatorConfig,
    relevant_rounds: Vec<Uuid>,
    context: &'a C
}

#[derive(Debug, Clone)]
pub struct DrawEvaluatorConfig {
    pub adj_adj_clash_factor: f32,
    pub adj_team_clash_factor: f32,
    pub adj_speaker_clash_factor: f32,
    pub team_team_clash_factor: f32,
    pub team_speaker_clash_factor: f32,
    pub speaker_speaker_clash_factor: f32,

    pub adj_adj_repeat_clash_severity: u16,
    pub adj_team_repeat_clash_severity: u16,
    pub adj_non_aligned_speaker_repeat_clash_severity: u16,
    pub team_team_repeat_clash_severity: u16,
    pub team_speaker_repeat_clash_severity: u16,
    pub non_aligned_speakers_repeat_clash_severity: u16,
}

impl Default for DrawEvaluatorConfig {
    fn default() -> Self {
        DrawEvaluatorConfig {
            adj_adj_clash_factor: 0.3,
            adj_team_clash_factor: 1.0,
            adj_speaker_clash_factor: 0.5,
            team_team_clash_factor: 0.2,
            team_speaker_clash_factor: 0.1,
            speaker_speaker_clash_factor: 0.1,
            adj_adj_repeat_clash_severity: 40,
            adj_team_repeat_clash_severity: 40,
            adj_non_aligned_speaker_repeat_clash_severity: 40,
            team_team_repeat_clash_severity: 10,
            team_speaker_repeat_clash_severity: 10,
            non_aligned_speakers_repeat_clash_severity: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DrawIssueTarget {
    Adjudicator {
        uuid: Uuid,
    },
    Speaker {
        uuid: Uuid,
    },
    Team {
        uuid: Uuid,
        involved_speakers: Vec<Uuid>,
    },
}

impl DrawIssueTarget {
    pub fn uuid(&self) -> Uuid {
        match self {
            DrawIssueTarget::Adjudicator { uuid } => *uuid,
            DrawIssueTarget::Speaker { uuid } => *uuid,
            DrawIssueTarget::Team { uuid, .. } => *uuid,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DrawIssue {
    #[serde(flatten)]
    pub issue_type: ClashType,
    pub severity: u16,
    pub target: DrawIssueTarget,
}

#[derive(Debug)]
pub struct BallotEvaluationResult {
    pub government_issues: Vec<DrawIssue>,
    pub opposition_issues: Vec<DrawIssue>,
    pub non_aligned_issues: HashMap<Uuid, Vec<DrawIssue>>,
    pub adjudicator_issues: HashMap<Uuid, Vec<DrawIssue>>,
}

impl BallotEvaluationResult {
    pub fn new() -> Self {
        BallotEvaluationResult {
            government_issues: Vec::new(),
            opposition_issues: Vec::new(),
            non_aligned_issues: HashMap::new(),
            adjudicator_issues: HashMap::new(),
        }
    }

    pub fn total_severity(&self) -> u32 {
        self.government_issues
            .iter()
            .map(|i| i.severity as u32)
            .sum::<u32>()
            + self
                .opposition_issues
                .iter()
                .map(|i| i.severity as u32)
                .sum::<u32>()
            + self
                .non_aligned_issues
                .iter()
                .map(|(_, issues)| issues.iter().map(|i| i.severity as u32).sum::<u32>())
                .sum::<u32>()
            + self
                .adjudicator_issues
                .iter()
                .map(|(_, issues)| issues.iter().map(|i| i.severity as u32).sum::<u32>())
                .sum::<u32>()
    }
}


struct DrawInProgress {
    history: ParticipantsDebateHistory,
    relevant_rounds: Vec<Uuid>,
}

struct ParticipantsDebateHistory {
    participant_debates: HashMap<Uuid, HashMap<Uuid, ParticipantsDebateHistoryEntry>>,
}

impl ParticipantsDebateHistory {
    pub fn get_debate_participants(&self) -> HashMap<Uuid, Vec<Uuid>> {
        self.participant_debates.iter().flat_map(
            |(p_id, round_debates)| {
                round_debates.iter().map(|(d_id, _)| (*d_id, *p_id))
            }
        ).into_group_map()
    }
}

impl ParticipantsDebateHistory {
    async fn new_for_tournament<C>(db: &C, tournament_id: Uuid, team_members: &HashMap<Uuid, Vec<Uuid>>) -> anyhow::Result<Self> where C: ConnectionTrait {
        let rounds = TournamentRound::get_all_in_tournament(db, tournament_id).await?.into_iter().map(|r| r.uuid).collect::<Vec<_>>();
        let debates = TournamentDebate::get_all_in_rounds(db, rounds.clone()).await?;
        
        let flat_debates = debates.iter().flat_map(|d| d.iter()).map(|d| d.uuid).collect::<Vec<_>>();

        let ballots : HashMap<_, _> = Ballot::get_all_in_debates(db, flat_debates).await?.into_iter().collect();

        let mut new_val = ParticipantsDebateHistory {
            participant_debates: HashMap::new(),
        };

        for (round_id, debates) in izip![rounds, debates] {
            for debate in debates {
                let ballot = ballots.get(&debate.uuid).unwrap();
                let debate_id = debate.uuid;

                new_val.add_from_ballot(round_id, debate_id, &ballot, team_members);
            }
        }
        
        Ok(new_val)
    }

    fn remove_debate_entries(&mut self, debate_id: Uuid) {
        for (_, round_activties) in self.participant_debates.iter_mut() {
            round_activties.retain(|_, entry| entry.debate_id != debate_id);
        }
    }

    fn add_from_ballot(
        &mut self,
        round_id: Uuid,
        debate_id: Uuid,
        ballot: &ballot::Ballot,
        team_members: &HashMap<Uuid, Vec<Uuid>>
    ) -> Vec<Uuid> {
        let gov_team = ballot.government.clone().team;
        let opp_team = ballot.opposition.clone().team;
        let chair = ballot.adjudicators.get(0);
        let wings = ballot.adjudicators.iter().skip(1).collect::<Vec<_>>();
        let non_aligned_speakers = ballot.speeches.iter().filter_map(|s| match s.role {
            SpeechRole::NonAligned => s.speaker,
            _ => None
        }).collect::<Vec<_>>();

        let gov_members = if let Some(gov_team) = gov_team {
            team_members.get(&gov_team).cloned().unwrap_or_default()
        }
        else {
            vec![]
        };

        let opp_members = if let Some(opp_team) = opp_team {
            team_members.get(&opp_team).cloned().unwrap_or_default()
        }
        else {
            vec![]
        };

        let gov_members = gov_members.iter().map(|m| (*m, ParticipantsDebateHistoryEntryRole::Team)).collect::<HashMap<_, _>>();
        let opp_members = opp_members.iter().map(|m| (*m, ParticipantsDebateHistoryEntryRole::Team)).collect::<HashMap<_, _>>();

        let chair = chair.map(
            |chair| {
                (*chair, ParticipantsDebateHistoryEntryRole::Chair)
            }
        );
        let wings = wings.into_iter().map(|w| (*w, ParticipantsDebateHistoryEntryRole::Wing)).collect::<HashMap<_, _>>();
        let non_aligned_speakers = non_aligned_speakers.iter().map(|s| (*s, ParticipantsDebateHistoryEntryRole::NonAligned)).collect::<HashMap<_, _>>();

        let all_participants: HashMap<Uuid, ParticipantsDebateHistoryEntryRole> = gov_members.into_iter().chain(opp_members.into_iter()).chain(chair.into_iter()).chain(wings.into_iter()).chain(non_aligned_speakers.into_iter()).collect::<HashMap<_, _>>();

        let affected_participant_ids = all_participants.keys().cloned().collect::<Vec<_>>();

        for (p, role) in all_participants {
            self.participant_debates.entry(p).or_insert_with(HashMap::new).insert(round_id, ParticipantsDebateHistoryEntry {
                role,
                debate_id
            });
        }

        affected_participant_ids
    }


    fn add_from_draw_ballot(
        &mut self,
        round_id: Uuid,
        debate_id: Uuid,
        ballot: &DrawBallot,
        team_members: &HashMap<Uuid, Vec<Uuid>>
    ) {
        let gov_team = ballot.government.clone().unwrap();
        let opp_team = ballot.opposition.clone().unwrap();
        let chair = ballot.adjudicators.get(0);
        let wings = ballot.adjudicators.iter().skip(1).collect::<Vec<_>>();
        let non_aligned_speakers = ballot.non_aligned_speakers.clone();

        let gov_members = team_members.get(&gov_team.uuid).cloned().unwrap_or_default();
        let opp_members = team_members.get(&opp_team.uuid).cloned().unwrap_or_default();

        let gov_members = gov_members.iter().map(|m| (*m, ParticipantsDebateHistoryEntryRole::Team)).collect::<HashMap<_, _>>();
        let opp_members = opp_members.iter().map(|m| (*m, ParticipantsDebateHistoryEntryRole::Team)).collect::<HashMap<_, _>>();

        let chair = chair.map(
            |chair| {
                (chair.adjudicator.uuid, ParticipantsDebateHistoryEntryRole::Chair)
            }
        );
        let wings = wings.iter().map(|w| (w.adjudicator.uuid, ParticipantsDebateHistoryEntryRole::Wing)).collect::<HashMap<_, _>>();
        let non_aligned_speakers = non_aligned_speakers.iter().filter_map(|s| (s.clone().map(|s| (s.uuid, ParticipantsDebateHistoryEntryRole::NonAligned)))).collect::<HashMap<_, _>>();

        let all_participants: HashMap<Uuid, ParticipantsDebateHistoryEntryRole> = gov_members.into_iter().chain(opp_members.into_iter()).chain(chair.into_iter()).chain(wings.into_iter()).chain(non_aligned_speakers.into_iter()).collect::<HashMap<_, _>>();

        for (p, role) in all_participants {
            self.participant_debates.entry(p).or_insert_with(HashMap::new).insert(round_id, ParticipantsDebateHistoryEntry {
                role,
                debate_id
            });
        }
    }
}

#[derive(Debug)]
struct ParticipantsDebateHistoryEntry {
    role: ParticipantsDebateHistoryEntryRole,
    debate_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
enum ParticipantsDebateHistoryEntryRole {
    Chair,
    Wing,
    NonAligned,
    Team
}

impl ParticipantsDebateHistory {
    fn new() -> Self {
        ParticipantsDebateHistory {
            participant_debates: HashMap::new(),
        }
    }
}

#[derive(Error, Debug)]
pub enum DrawEvaluationError {
    #[error("Rounds are not in same tournament")]
    RoundsTournamentMismatch,
    #[error("Ballot Parse Error")]
    BallotParseError(#[from] BallotParseError),
    #[error("SeaORM Error")]
    SeaORMError(#[from] sea_orm::error::DbErr),
    #[error("Other")]
    Other(#[from] anyhow::Error),
}

pub trait DrawEvaluationContext {
    fn get_speaker_team(&self, uuid: Uuid) -> Option<Uuid>;
    fn get_participant(&self, uuid: Uuid) -> Option<&Participant>;
    fn get_participant_declared_clashes(&self, uuid: Uuid) -> Option<&HashMap<Uuid, u16>>;
    fn get_team_members(&self, team_id: Uuid) -> Option<&Vec<Uuid>>;
    fn get_participant_debate_history(&self, uuid: Uuid) -> Option<&HashMap<Uuid, ParticipantsDebateHistoryEntry>>;
}

pub struct DrawConstructionEvaluationContext {
    speaker_teams: HashMap<Uuid, Uuid>,
    participants_by_id: HashMap<Uuid, Participant>,
    participant_declared_clashes: HashMap<Uuid, HashMap<Uuid, u16>>,
    history: ParticipantsDebateHistory,
    team_members: HashMap<Uuid, Vec<Uuid>>,
}

impl DrawEvaluationContext for DrawConstructionEvaluationContext {
    fn get_speaker_team(&self, uuid: Uuid) -> Option<Uuid> {
        self.speaker_teams.get(&uuid).cloned()
    }

    fn get_participant(&self, uuid: Uuid) -> Option<&Participant> {
        self.participants_by_id.get(&uuid)
    }

    fn get_participant_declared_clashes(&self, uuid: Uuid) -> Option<&HashMap<Uuid, u16>> {
        self.participant_declared_clashes.get(&uuid)
    }

    fn get_team_members(&self, team_id: Uuid) -> Option<&Vec<Uuid>> {
        self.team_members.get(&team_id)
    }

    fn get_participant_debate_history(&self, uuid: Uuid) -> Option<&HashMap<Uuid, ParticipantsDebateHistoryEntry>> {
        self.history.participant_debates.get(&uuid)
    }
}

impl DrawConstructionEvaluationContext {
    pub async fn new_from_tournament<C>(db: &C, tournament_id: Uuid) -> anyhow::Result<Self> where C:ConnectionTrait {
        let participants = Participant::get_all_in_tournament(db, tournament_id).await?;
        let speaker_teams = participants.iter().filter_map(|p| {
            if let ParticipantRole::Speaker(speaker_info) = &p.role {
                speaker_info.team_id.map(|t_id| (p.uuid, t_id))
            }
            else {
                None
            }
        }).collect::<HashMap<_, _>>();

        let all_participant_declared_clashes =  ParticipantClash::get_all_in_tournament(db, tournament_id).await?.into_iter().flat_map(|clash| {
            vec![
                (
                    (clash.declaring_participant_id, clash.target_participant_id),
                    clash.clash_severity
                ),
                (
                    (clash.target_participant_id, clash.declaring_participant_id),
                    clash.clash_severity
                )
            ]
        }).into_group_map();
        let all_participant_declared_clashes = all_participant_declared_clashes.into_iter().map(
            |((p1, p2), severities)| ((p1, p2), severities.into_iter().max().unwrap_or(0))
        ).collect::<HashMap<_, _>>();

        let mut participant_declared_clashes = HashMap::new();

        for ((p1, p2), severity) in all_participant_declared_clashes {
            participant_declared_clashes.entry(p1).or_insert_with(HashMap::new).insert(p2, severity);
            participant_declared_clashes.entry(p2).or_insert_with(HashMap::new).insert(p1, severity);
        }

        let team_members = participants.iter().filter_map(|p| {
            if let ParticipantRole::Speaker(speaker_info) = &p.role {
                speaker_info.team_id.map(|t_id| (t_id, p.uuid))
            }
            else {
                None
            }
        }).into_group_map();

        Ok(
            DrawConstructionEvaluationContext {
                speaker_teams,
                participants_by_id: participants.into_iter().map(|p| (p.uuid, p)).collect(),
                participant_declared_clashes,
                history: ParticipantsDebateHistory::new_for_tournament(
                    db,
                    tournament_id,
                    &team_members
                ).await?,
                team_members,
            }
        )
    }
    
    pub fn add_round_ballots(&mut self, round_id: Uuid, ballots: Vec<&DrawBallot>) {
        for ballot in ballots.into_iter() {
            //This is *not* the debate id that the debate will be saved as.
            //Instead it is a temporary indicator for use only within the draw evaluator.
            let debate_id = Uuid::new_v4();

            self.history.add_from_draw_ballot(round_id, debate_id, ballot, &self.team_members);
        }
    }
}


pub struct TournamentObservingDrawEvaluationContext {
    tournament_id: Uuid,
    participant_info: TournamentParticipantsInfo,
    participant_clash_severities: HashMap<Uuid, HashMap<Uuid, u16>>,
    history: ParticipantsDebateHistory,
    debate_participants: HashMap<Uuid, Vec<Uuid>>,
    debate_active_ballots: HashMap<Uuid, Uuid>,
    clash_participants: HashMap<Uuid, (Uuid, Uuid)>,
    debate_rounds: HashMap<Uuid, Uuid>,
}

impl TournamentObservingDrawEvaluationContext {
    pub async fn new_from_tournament<C>(
        db: &C,
        tournament_id: Uuid
    ) -> anyhow::Result<Self> where C: ConnectionTrait {
        let participant_info = TournamentParticipantsInfo::load(db, tournament_id).await?;
        let all_participant_declared_clashes =  ParticipantClash::get_all_in_tournament(db, tournament_id).await?.into_iter().collect_vec();

        let mut clash_participants = HashMap::new();

        for clash in all_participant_declared_clashes.iter() {
            clash_participants.insert(clash.uuid, (clash.declaring_participant_id, clash.target_participant_id));
        }

        let all_participant_declared_clashes = all_participant_declared_clashes.into_iter().flat_map(|clash| {
            vec![
                (
                    (clash.declaring_participant_id, clash.target_participant_id),
                    clash.clash_severity
                ),
                (
                    (clash.target_participant_id, clash.declaring_participant_id),
                    clash.clash_severity
                )
            ]
        }).into_group_map().into_iter().map(
            |((p1, p2), severities)| ((p1, p2), severities.into_iter().max().unwrap_or(0))
        ).collect::<HashMap<_, _>>();

        let mut participant_clash_severities = HashMap::new();

        for ((p1, p2), severity) in all_participant_declared_clashes {
            participant_clash_severities.entry(p1).or_insert_with(HashMap::new).insert(p2, severity);
            participant_clash_severities.entry(p2).or_insert_with(HashMap::new).insert(p1, severity);
        }

        let history = ParticipantsDebateHistory::new_for_tournament(
            db,
            tournament_id,
            &participant_info.team_members
        ).await?;

        let debate_participants = history.get_debate_participants();

        let rounds: Vec<Uuid> = TournamentRound::get_all_in_tournament(db, tournament_id).await?.into_iter().map(|r| r.uuid).collect::<Vec<_>>();
        let debates = TournamentDebate::get_all_in_rounds(db, rounds.clone()).await?;

        let debate_rounds = debates.iter().flatten().map(|d| (d.uuid, d.round_id)).collect::<HashMap<_, _>>();

        let debate_active_ballots = debates.into_iter().flat_map(|d| d.into_iter()).map(|d| (d.uuid, d.ballot_id)).collect::<HashMap<_, _>>();

        Ok(
            TournamentObservingDrawEvaluationContext {
                tournament_id,
                history,
                participant_info,
                participant_clash_severities,
                debate_participants,
                debate_active_ballots,
                clash_participants,
                debate_rounds
            }
        )
    }

    pub async fn update_from_changes<C>(&mut self, db: &C, changes: &EntityGroup) -> anyhow::Result<Vec<Uuid>> where C: ConnectionTrait {
        let mut affected_debates = HashSet::new();

        let changed_entities = changes.as_group_map();
        let deleted_entities = changes.as_delete_map();

        let mut changed_ballots = changed_entities.ballots.iter().map(|b| (b.uuid, b)).collect::<HashMap<_, _>>();
        let mut ballot_ids_to_load = vec![];

        let mut debate_ballots_to_reevaluate = HashSet::new();

        for deleted_debate in deleted_entities.tournament_debates.iter() {
            self.history.remove_debate_entries(*deleted_debate);
        }

        for changed_debate in changed_entities.tournament_debates.iter() {
            self.debate_rounds.insert(changed_debate.uuid, changed_debate.round_id);
            if Some(&changed_debate.ballot_id) != self.debate_active_ballots.get(&changed_debate.uuid) {
                debate_ballots_to_reevaluate.insert((changed_debate.uuid, changed_debate.ballot_id));
                if !changed_ballots.contains_key(&changed_debate.ballot_id) {
                    ballot_ids_to_load.push(changed_debate.ballot_id);
                }
                self.debate_active_ballots.insert(changed_debate.uuid, changed_debate.ballot_id);
            }
        }

        let ballot_debates = self.debate_active_ballots.iter().map(|(debate_id, ballot_id)| (*ballot_id, *debate_id)).collect::<HashMap<_, _>>();

        for changed_ballot in changed_entities.ballots.iter() {
            let ballot_debate = ballot_debates.get(&changed_ballot.uuid);

            if let Some(ballot_debate) = ballot_debate {
                debate_ballots_to_reevaluate.insert((*ballot_debate, changed_ballot.uuid));
            }
        }

        let missing_ballots = Ballot::get_many(db, ballot_ids_to_load).await?;
        for ballot in missing_ballots.iter() {
            changed_ballots.insert(ballot.uuid, ballot);
        }

        affected_debates.extend(debate_ballots_to_reevaluate.iter().map(|(d, _)| *d));


        let mut affected_participants = vec![];
        for (debate_id, ballot_id) in debate_ballots_to_reevaluate {
            let ballot = changed_ballots.get(&ballot_id).unwrap();
            affected_participants.extend(self.history.add_from_ballot(self.debate_rounds.get(&debate_id).cloned().unwrap_or_default(), debate_id, ballot, &self.participant_info.team_members).into_iter());
        }

        for p in affected_participants.iter() {
            affected_debates.extend(self.history.participant_debates.get(&p).into_iter().flat_map(|m| m.values().map(|d| d.debate_id)));
        }

        if changes.has_changes_for_types(vec![
            EntityTypeId::Participant,
            EntityTypeId::Team,
            EntityTypeId::TournamentInstitution
        ]) {
            self.participant_info = TournamentParticipantsInfo::load(db, self.tournament_id).await?;
            affected_debates.extend(self.debate_active_ballots.keys().cloned());
        }

        let mut participant_clash_reload_ids = HashSet::new();

        for deleted_participant in deleted_entities.participants {
            self.participant_clash_severities.remove(&deleted_participant);
            for v in self.participant_clash_severities.values_mut() {
                v.remove(&deleted_participant);
            }
        }

        for deleted_clash in deleted_entities.participant_clashs {
            if let Some((p1, p2)) = self.clash_participants.get(&deleted_clash) {
                participant_clash_reload_ids.insert(*p1);
                participant_clash_reload_ids.insert(*p2);
            }
            self.clash_participants.remove(&deleted_clash);
        }

        for added_clash in changed_entities.participant_clashs {
            participant_clash_reload_ids.insert(added_clash.declaring_participant_id);
            participant_clash_reload_ids.insert(added_clash.target_participant_id);

            self.clash_participants.insert(added_clash.uuid, (added_clash.declaring_participant_id, added_clash.target_participant_id));
        }

        for reload_id in participant_clash_reload_ids.iter() {
            self.participant_clash_severities.remove(reload_id);
            affected_debates.extend(
                self.history.participant_debates.get(reload_id).into_iter().flat_map(|m| m.values().map(|d| d.debate_id))
            );
        }

        let reloaded_clashes = ParticipantClash::get_all_declared_by_participants(db, participant_clash_reload_ids.into_iter().collect()).await?;

        let all_participant_declared_clashes: HashMap<(_, _), _> = reloaded_clashes.into_iter().flat_map(|clash| {
            vec![
                (
                    (clash.declaring_participant_id, clash.target_participant_id),
                    clash.clash_severity
                ),
                (
                    (clash.target_participant_id, clash.declaring_participant_id),
                    clash.clash_severity
                )
            ]
        }).into_group_map().into_iter().map(
            |((p1, p2), severities)| ((p1, p2), severities.into_iter().max().unwrap_or(0))
        ).collect::<HashMap<_, _>>();

        for ((p1, p2), severity) in all_participant_declared_clashes {
            self.participant_clash_severities.entry(p1).or_insert_with(HashMap::new).insert(p2, severity);
            self.participant_clash_severities.entry(p2).or_insert_with(HashMap::new).insert(p1, severity);
        }

        Ok(Vec::from_iter(affected_debates.into_iter()))
    }
}

impl DrawEvaluationContext for TournamentObservingDrawEvaluationContext {
    fn get_speaker_team(&self, uuid: Uuid) -> Option<Uuid> {
        self.participant_info.speaker_teams.get(&uuid).cloned()
    }

    fn get_participant(&self, uuid: Uuid) -> Option<&Participant> {
        self.participant_info.participants_by_id.get(&uuid)
    }

    fn get_participant_declared_clashes(&self, uuid: Uuid) -> Option<&HashMap<Uuid, u16>> {
        self.participant_clash_severities.get(&uuid)
    }

    fn get_team_members(&self, team_id: Uuid) -> Option<&Vec<Uuid>> {
        self.participant_info.team_members.get(&team_id)
    }

    fn get_participant_debate_history(&self, uuid: Uuid) -> Option<&HashMap<Uuid, ParticipantsDebateHistoryEntry>> {
        self.history.participant_debates.get(&uuid)
    }
}


impl<'a, C> DrawEvaluator<'a, C> {
    pub fn new(
        config: DrawEvaluatorConfig,
        relevant_rounds: Vec<Uuid>,
        context: &'a C
    ) -> Self {
        DrawEvaluator {
            config,
            relevant_rounds,
            context
        }
    }

    pub fn get_base_severity(&self, clash_type: &ClashType) -> u16 {
        match clash_type {
            ClashType::JudgeHasSeenJudge { .. } => self.config.adj_adj_repeat_clash_severity,
            ClashType::DeclaredClash { severity } => *severity,
            ClashType::InstitutionalClash { severity, .. } => *severity,
            ClashType::SameTeamClash => 1000,
            ClashType::SpeakersHaveMetAsNonAligned { round } => 10,
            ClashType::SpeakersHaveMetAsTeamAndNonAligned { round } => 10,
            ClashType::SpeakersHaveMetAsTeam { round } => 10,
            ClashType::JudgeHasSeenSpeaker { round, judge_was_chair, speaker_was_in_team } => 10,
        }
    }

    pub fn find_issues_in_ballot(&self, ballot: &DrawBallot) -> BallotEvaluationResult where C: DrawEvaluationContext {
        self.find_issues_in_debate(&ballot.into())
    }

    pub fn get_permanent_clashes_between_participants(&self, p1: Uuid, p2: Uuid) -> Vec<ClashType> where C: DrawEvaluationContext {
        let p1_info: Option<&Participant> = self.context.get_participant(p1);
        let p2_info = self.context.get_participant(p2);

        if !p1_info.is_some() || !p2_info.is_some() {
            return vec![];
        }

        let mut all_clashes = vec![];

        let declared_clash_severity = self.context.get_participant_declared_clashes(p1).and_then(
            |m| m.get(&p2).cloned()
        ).unwrap_or_default().max(
            self.context.get_participant_declared_clashes(p2).and_then(
                |m| m.get(&p1).cloned()
            ).unwrap_or_default()
        );

        if declared_clash_severity > 0 {
            all_clashes.push(ClashType::DeclaredClash {
                severity: declared_clash_severity,
            });
        }

        let p1_info = p1_info.unwrap();
        let p2_info = p2_info.unwrap();

        match (self.context.get_speaker_team(p1), self.context.get_speaker_team(p2)) {
            (Some(t1), Some(t2)) => {
                if t1 == t2 {
                    all_clashes.push(ClashType::SameTeamClash {});
                }
            },
            _ => {}
        }

        let p1_institutions : HashMap<_, _> = p1_info.institutions.iter().map(|i| (i.uuid, i.clash_severity)).collect();

        for p2_inst in p2_info.institutions.iter() {
            if let Some(p1_severity) = p1_institutions.get(&p2_inst.uuid) {
                all_clashes.push(ClashType::InstitutionalClash {
                    institution_id: p2_inst.uuid,
                    severity: (p1_severity / 2 + p2_inst.clash_severity / 2),
                });
            }
        }

        all_clashes
    }

    pub fn get_dynamic_clashes_between_participants(&self, p1: Uuid, p2: Uuid) -> Vec<ClashType> where C: DrawEvaluationContext {
        let p1_hist = self.context.get_participant_debate_history(p1);
        let p2_hist = self.context.get_participant_debate_history(p2);

        if !p1_hist.is_some() || !p2_hist.is_some() {
            return vec![];
        }

        let p1_hist = p1_hist.unwrap();
        let p2_hist = p2_hist.unwrap();

        self.relevant_rounds.iter().filter_map(
            |r| {
                let p1_role = p1_hist.get(&r);
                let p2_role = p2_hist.get(&r);

                match (p1_role, p2_role) {
                    (Some(p1_role), Some(p2_role)) if p1_role.debate_id == p2_role.debate_id => {
                        match (
                            p1_role.role.min(p2_role.role),
                            p1_role.role.max(p2_role.role)
                        ) {
                            (ParticipantsDebateHistoryEntryRole::Chair, ParticipantsDebateHistoryEntryRole::Wing) => {
                                Some(ClashType::JudgeHasSeenJudge { round: *r })
                            },
                            (ParticipantsDebateHistoryEntryRole::Chair, ParticipantsDebateHistoryEntryRole::NonAligned) => {
                                Some(ClashType::JudgeHasSeenSpeaker {
                                    round: *r,
                                    judge_was_chair: true,
                                    speaker_was_in_team: false
                                })
                            },
                            (ParticipantsDebateHistoryEntryRole::Chair, ParticipantsDebateHistoryEntryRole::Team) => {
                                Some(ClashType::JudgeHasSeenSpeaker {
                                    round: *r,
                                    judge_was_chair: true,
                                    speaker_was_in_team: true
                                })
                            },
                            (ParticipantsDebateHistoryEntryRole::Wing, ParticipantsDebateHistoryEntryRole::Wing) => {
                                Some(ClashType::JudgeHasSeenJudge { round: *r })
                            },
                            (ParticipantsDebateHistoryEntryRole::Wing, ParticipantsDebateHistoryEntryRole::NonAligned) => {
                                Some(ClashType::JudgeHasSeenSpeaker {
                                    round: *r,
                                    judge_was_chair: false,
                                    speaker_was_in_team: false
                                })
                            },
                            (ParticipantsDebateHistoryEntryRole::Wing, ParticipantsDebateHistoryEntryRole::Team) => {
                                Some(ClashType::JudgeHasSeenSpeaker {
                                    round: *r,
                                    judge_was_chair: false,
                                    speaker_was_in_team: true
                                })
                            },
                            (ParticipantsDebateHistoryEntryRole::NonAligned, ParticipantsDebateHistoryEntryRole::NonAligned) => {
                                Some(ClashType::SpeakersHaveMetAsNonAligned { round: *r })
                            },
                            (ParticipantsDebateHistoryEntryRole::NonAligned, ParticipantsDebateHistoryEntryRole::Team) => {
                                Some(ClashType::SpeakersHaveMetAsTeamAndNonAligned { round: *r })
                            },
                            (
                                ParticipantsDebateHistoryEntryRole::Team,
                                ParticipantsDebateHistoryEntryRole::Team
                            ) => {
                                Some(ClashType::SpeakersHaveMetAsTeam { round: *r })
                            },
                            _ => None
                        }
                    },
                    _ => None
                }
            }
        ).collect()
    }

    pub fn get_all_clashes_between_participants(&self, p1: Uuid, p2: Uuid) -> Vec<ClashType> where C: DrawEvaluationContext {
        let mut all_clashes = self.get_permanent_clashes_between_participants(p1, p2);
        all_clashes.extend(self.get_dynamic_clashes_between_participants(p1, p2));
        all_clashes
    }

    pub fn find_issues_in_debate(
        &self,
        ballot: &DebateInfo,
    ) -> BallotEvaluationResult where C: DrawEvaluationContext {
        let gov_member_ids = ballot
            .government
            .map(|t| self.context.get_team_members(t).cloned())
            .flatten()
            .unwrap_or(vec![]);
        let opp_member_ids = ballot
            .opposition
            .map(|t| self.context.get_team_members(t).cloned())
            .flatten()
            .unwrap_or(vec![]);
        let adjudicator_ids = ballot
            .chair
            .iter()
            .chain(ballot.wings.iter())
            .cloned()
            .collect_vec();

        let mut issues = BallotEvaluationResult::new();

        for adj_pair in adjudicator_ids.iter().combinations(2) {
            let adj_1_id = adj_pair[0];
            let adj_2_id = adj_pair[1];

            let clashes = self.get_all_clashes_between_participants(*adj_1_id, *adj_2_id);
            for clash in clashes {
                let severity = (self.get_base_severity(&clash) as f32
                    * self.config.adj_adj_clash_factor) as u16;
                issues
                    .adjudicator_issues
                    .entry(*adj_1_id)
                    .or_insert_with(Vec::new)
                    .push(DrawIssue {
                        issue_type: clash.clone(),
                        severity: severity,
                        target: DrawIssueTarget::Adjudicator { uuid: *adj_2_id },
                    });
                issues
                    .adjudicator_issues
                    .entry(*adj_2_id)
                    .or_insert_with(Vec::new)
                    .push(DrawIssue {
                        issue_type: clash.clone(),
                        severity: severity,
                        target: DrawIssueTarget::Adjudicator { uuid: *adj_1_id },
                    });
            }
        }

        for (adj_id, speaker_id) in adjudicator_ids
            .iter()
            .cartesian_product(ballot.non_aligned_speakers.iter())
        {
            let clashes = self.get_all_clashes_between_participants(*adj_id, *speaker_id);
            for clash in clashes {
                let severity = (self.get_base_severity(&clash) as f32
                    * self.config.adj_speaker_clash_factor) as u16;
                issues
                    .adjudicator_issues
                    .entry(*adj_id)
                    .or_insert_with(Vec::new)
                    .push(DrawIssue {
                        issue_type: clash.clone(),
                        severity: severity,
                        target: DrawIssueTarget::Speaker { uuid: *speaker_id },
                    });
                issues
                    .non_aligned_issues
                    .entry(*speaker_id)
                    .or_insert_with(Vec::new)
                    .push(DrawIssue {
                        issue_type: clash.clone(),
                        severity: severity,
                        target: DrawIssueTarget::Adjudicator { uuid: *adj_id },
                    });
            }
        }

        for adj_id in adjudicator_ids.iter() {
            vec![
                (&ballot.government, &gov_member_ids),
                (&ballot.opposition, &opp_member_ids),
            ]
            .into_iter()
            .map(|(team_id, member_ids)| {
                member_ids
                    .iter()
                    .flat_map(|member_id| {
                       self.get_all_clashes_between_participants(*adj_id, *member_id).iter()
                            .map(|c| {
                                DrawIssue {
                                        issue_type: c.clone(),
                                        severity: (self.get_base_severity(&c) as f32
                                            * self.config.adj_team_clash_factor)
                                            as u16,
                                        target: DrawIssueTarget::Team {
                                            uuid: *team_id.as_ref().unwrap(),
                                            involved_speakers: vec![*member_id],
                                        },
                                }
                            })
                            .collect_vec()
                    })
                    .sorted()
                    .coalesce(coalesce_issues)
                    .collect_vec()
            })
            .flatten()
            .for_each(|issue| {
                issues
                    .adjudicator_issues
                    .entry(*adj_id)
                    .or_insert_with(Vec::new)
                    .push(issue.clone());
                match &issue.target {
                    DrawIssueTarget::Team { uuid: team_id, .. } => {
                        if *team_id
                            == ballot
                                .government
                                .as_ref()
                                .map(|t| *t)
                                .unwrap_or(Uuid::nil())
                        {
                            issues.government_issues.push(DrawIssue {
                                target: DrawIssueTarget::Adjudicator { uuid: *adj_id },
                                ..issue
                            });
                        } else if *team_id
                            == ballot
                                .opposition
                                .as_ref()
                                .map(|t| *t)
                                .unwrap_or(Uuid::nil())
                        {
                            issues.opposition_issues.push(DrawIssue {
                                target: DrawIssueTarget::Adjudicator { uuid: *adj_id },
                                ..issue
                            });
                        } else {
                            unreachable!()
                        }
                    }
                    _ => unreachable!(),
                }
            });
        }

        for non_aligned_id in ballot.non_aligned_speakers.iter() {
            vec![
                (&ballot.government, &gov_member_ids),
                (&ballot.opposition, &opp_member_ids),
            ]
            .into_iter()
            .map(|(team_id, member_ids)| {
                member_ids
                    .iter()
                    .flat_map(|member_id| {
                        self.get_all_clashes_between_participants(*non_aligned_id, *member_id)
                            .iter()
                            .map(|c| {
                                DrawIssue {
                                    issue_type: c.clone(),
                                    severity: (self.get_base_severity(&c) as f32
                                        * self.config.team_speaker_clash_factor)
                                        as u16,
                                    target: DrawIssueTarget::Team {
                                        uuid: *team_id.as_ref().unwrap(),
                                        involved_speakers: vec![*member_id],
                                    },
                                }
                            })
                            .collect_vec()
                    })
                    .sorted()
                    .coalesce(coalesce_issues)
                    .collect_vec()
            })
            .flatten()
            .for_each(|issue| {
                issues
                    .non_aligned_issues
                    .entry(*non_aligned_id)
                    .or_insert_with(Vec::new)
                    .push(issue.clone());
                match &issue.target {
                    DrawIssueTarget::Team { uuid: team_id, .. } => {
                        if *team_id
                            == ballot
                                .government
                                .as_ref()
                                .map(|t| *t)
                                .unwrap_or(Uuid::nil())
                        {
                            issues.government_issues.push(DrawIssue {
                                target: DrawIssueTarget::Speaker {
                                    uuid: *non_aligned_id,
                                },
                                ..issue
                            });
                        } else if *team_id
                            == ballot
                                .opposition
                                .as_ref()
                                .map(|t| *t)
                                .unwrap_or(Uuid::nil())
                        {
                            issues.opposition_issues.push(DrawIssue {
                                target: DrawIssueTarget::Speaker {
                                    uuid: *non_aligned_id,
                                },
                                ..issue
                            });
                        } else {
                            unreachable!()
                        }
                    }
                    _ => unreachable!(),
                }
            });

            ballot
                .non_aligned_speakers
                .iter()
                .filter(|id| *id != non_aligned_id)
                .map(|other_id| {
                    self.get_all_clashes_between_participants(*non_aligned_id, *other_id )
                        .iter()
                        .map(|c| {
                            DrawIssue {
                                issue_type: c.clone(),
                                severity: (self.get_base_severity(&c) as f32
                                    * self.config.speaker_speaker_clash_factor)
                                    as u16,
                                target: DrawIssueTarget::Speaker { uuid: *other_id },
                            }
                        })
                        .sorted()
                        .coalesce(coalesce_issues)
                        .collect_vec()
                })
                .flatten()
                .for_each(|issue| {
                    issues
                        .non_aligned_issues
                        .entry(*non_aligned_id)
                        .or_insert_with(Vec::new)
                        .push(issue.clone());
                });
        }

        for gov_speaker_id in gov_member_ids {
            opp_member_ids.iter().cloned().flat_map(|opp_speaker_id| {
                self.get_all_clashes_between_participants(gov_speaker_id, opp_speaker_id)
                    .iter()
                        .map(|c| {
                            DrawIssue {
                                issue_type: c.clone(),
                                severity: (self.get_base_severity(&c) as f32
                                    * self.config.team_team_clash_factor)
                                    as u16,
                                target: DrawIssueTarget::Team {
                                    uuid: ballot
                                        .opposition
                                        .as_ref()
                                        .map(|t| *t)
                                        .unwrap_or(Uuid::nil()),
                                    involved_speakers: vec![opp_speaker_id],
                                },
                            }
                        })
                        .collect_vec()
                })
                .sorted()
                .coalesce(coalesce_issues)
                .for_each(|issue| {
                    issues.government_issues.push(issue.clone());
                    issues.opposition_issues.push(DrawIssue {
                        target: DrawIssueTarget::Team {
                            uuid: ballot
                                .government
                                .as_ref()
                                .map(|t| *t)
                                .unwrap_or(Uuid::nil()),
                            involved_speakers: vec![gov_speaker_id],
                        },
                        ..issue
                    });
                });
        }

        issues
    }
}

fn coalesce_issues(prev: DrawIssue, next: DrawIssue) -> Result<DrawIssue, (DrawIssue, DrawIssue)> {
    // Some issues should not be repeated individually for each speaker in a team, since that
    // may confuse the user. Specifically a) If we were to account for each insitutional clash
    // individually, we would end up with a lot of clashes for the typical non-mixed team.
    // For the team repetition, we have a similar issue where we would artificially inflate the
    // severity of these clashes.
    match (&prev.issue_type, &next.issue_type) {
        (
            ClashType::JudgeHasSeenSpeaker { round: round1, judge_was_chair: judge_was_chair1, speaker_was_in_team: speaker_was_in_team1 },
            ClashType::JudgeHasSeenSpeaker { round: round2, judge_was_chair: judge_was_chair2, speaker_was_in_team: speaker_was_in_team2 },
        ) if round1 == round2 && *speaker_was_in_team1 == true => {
            match (&prev.target, &next.target) {
                (
                    DrawIssueTarget::Team { uuid, involved_speakers },
                    DrawIssueTarget::Team { uuid: next_uuid, involved_speakers: next_involved_speakers },
                ) => {
                    if uuid == next_uuid {
                        Ok(DrawIssue {
                            issue_type: prev.issue_type.clone(),
                            severity: prev.severity + next.severity,
                            target: DrawIssueTarget::Team {
                                uuid: *uuid,
                                involved_speakers: involved_speakers.iter().chain(next_involved_speakers.iter()).copied().collect_vec(),
                            },
                        })
                    } else {
                        Err((prev, next))
                    }
                },
                _ => {
                    Err((prev, next))
                }
            }
        },
        (
            ClashType::InstitutionalClash {
                severity: severity_1,
                institution_id: i_id_1,
            },
            ClashType::InstitutionalClash {
                severity: severity_2,
                institution_id: i_id_2,
            },
        ) if i_id_1 == i_id_2 => match (&prev.target, &next.target) {
            (
                DrawIssueTarget::Team {
                    uuid: t_id_1,
                    involved_speakers: is_1,
                },
                DrawIssueTarget::Team {
                    uuid: t_id_2,
                    involved_speakers: is_2,
                },
            ) if t_id_1 == t_id_2 => Ok(DrawIssue {
                issue_type: ClashType::InstitutionalClash {
                    severity: u16::max(*severity_1, *severity_2),
                    institution_id: *i_id_1,
                },
                severity: u16::max(prev.severity, next.severity),
                target: DrawIssueTarget::Team {
                    uuid: *t_id_1,
                    involved_speakers: is_1.iter().chain(is_2.iter()).copied().collect_vec(),
                },
            }),
            _ => Err((prev, next)),
        },
        (_, _) => Err((prev, next)),
    }
}
/* 
#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use itertools::Itertools;
    use open_tab_entities::info::TournamentParticipantsInfo;
    use sea_orm::prelude::Uuid;

    use crate::{
        draw::{
            clashes::{ClashMap, ClashMapEntry, ClashType},
            evaluation::{DrawEvaluatorConfig, DrawIssue},
        },
        draw_view::{DrawAdjudicator, DrawBallot, DrawSpeaker, DrawTeam},
    };

    use super::DrawEvaluator;

    #[test]
    fn test_finds_institution_clashes_between_adjudicators() {
        let mut clash_map = ClashMap::new(Default::default());
        clash_map.add_clash_entry(
            Uuid::from_u128(600),
            Uuid::from_u128(601),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );

        let ballot = DrawBallot {
            adjudicators: vec![
                DrawAdjudicator {
                    uuid: Uuid::from_u128(600),
                    ..Default::default()
                }
                .into(),
                DrawAdjudicator {
                    uuid: Uuid::from_u128(601),
                    ..Default::default()
                }
                .into(),
            ],
            ..Default::default()
        };

        let evaluator = DrawEvaluator::new(
            clash_map,
            DrawEvaluatorConfig {
                adj_adj_clash_factor: 2.0,
                ..Default::default()
            },
        );
        let issues = evaluator.find_issues_in_ballot(&ballot, &TournamentParticipantsInfo::new());

        assert_eq!(
            issues
                .adjudicator_issues
                .get(&Uuid::from_u128(600))
                .unwrap(),
            &vec![DrawIssue {
                issue_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100)
                },
                severity: 180,
                target: crate::draw::evaluation::DrawIssueTarget::Adjudicator {
                    uuid: Uuid::from_u128(601)
                }
            }]
        );
        assert_eq!(
            issues
                .adjudicator_issues
                .get(&Uuid::from_u128(601))
                .unwrap(),
            &vec![DrawIssue {
                issue_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100)
                },
                severity: 180,
                target: crate::draw::evaluation::DrawIssueTarget::Adjudicator {
                    uuid: Uuid::from_u128(600)
                }
            }]
        );
    }

    #[test]
    fn test_finds_institution_clashes_between_adj_and_non_aligned() {
        let mut clash_map = ClashMap::new(Default::default());
        clash_map.add_clash_entry(
            Uuid::from_u128(600),
            Uuid::from_u128(700),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );

        let ballot = DrawBallot {
            adjudicators: vec![DrawAdjudicator {
                uuid: Uuid::from_u128(600),
                ..Default::default()
            }
            .into()],
            non_aligned_speakers: vec![Some(DrawSpeaker {
                uuid: Uuid::from_u128(700),
                ..Default::default()
            })],
            ..Default::default()
        };

        let evaluator = DrawEvaluator::new(
            clash_map,
            DrawEvaluatorConfig {
                adj_speaker_clash_factor: 2.0,
                ..Default::default()
            }
        );
        let issues = evaluator.find_issues_in_ballot(&ballot, &TournamentParticipantsInfo::new());

        assert_eq!(
            issues
                .adjudicator_issues
                .get(&Uuid::from_u128(600))
                .unwrap(),
            &vec![DrawIssue {
                issue_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100)
                },
                severity: 180,
                target: crate::draw::evaluation::DrawIssueTarget::Speaker {
                    uuid: Uuid::from_u128(700)
                }
            }]
        );
        assert_eq!(
            issues
                .non_aligned_issues
                .get(&Uuid::from_u128(700))
                .unwrap(),
            &vec![DrawIssue {
                issue_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100)
                },
                severity: 180,
                target: crate::draw::evaluation::DrawIssueTarget::Adjudicator {
                    uuid: Uuid::from_u128(600)
                }
            }]
        );
    }

    #[test]
    fn test_finds_institution_clashes_between_adj_and_gov() {
        let mut clash_map = ClashMap::new(Default::default());
        clash_map.add_clash_entry(
            Uuid::from_u128(600),
            Uuid::from_u128(700),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );

        let ballot = DrawBallot {
            adjudicators: vec![DrawAdjudicator {
                uuid: Uuid::from_u128(600),
                ..Default::default()
            }
            .into()],
            government: Some(DrawTeam {
                members: vec![DrawSpeaker {
                    uuid: Uuid::from_u128(700),
                    ..Default::default()
                }],
                uuid: Uuid::from_u128(800),
                ..Default::default()
            }),
            ..Default::default()
        };

        let evaluator = DrawEvaluator::new(
            clash_map,
            DrawEvaluatorConfig {
                adj_team_clash_factor: 2.0,
                ..Default::default()
            }
        );
        let mut info = TournamentParticipantsInfo::new();

        if let Some(g) = &ballot.government {
            info.team_members.insert(g.uuid, g.members.iter().map(|s| s.uuid).collect());
        }
        if let Some(o) = &ballot.opposition {
            info.team_members.insert(o.uuid, o.members.iter().map(|s| s.uuid).collect());
        }

        let issues = evaluator.find_issues_in_ballot(&ballot, &info);

        assert_eq!(
            issues
                .adjudicator_issues
                .get(&Uuid::from_u128(600))
                .unwrap(),
            &vec![DrawIssue {
                issue_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100)
                },
                severity: 180,
                target: crate::draw::evaluation::DrawIssueTarget::Team {
                    uuid: Uuid::from_u128(800),
                    involved_speakers: vec![Uuid::from_u128(700)]
                }
            }]
        );
        assert_eq!(
            issues.government_issues,
            vec![DrawIssue {
                issue_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100)
                },
                severity: 180,
                target: crate::draw::evaluation::DrawIssueTarget::Adjudicator {
                    uuid: Uuid::from_u128(600)
                }
            }]
        );
    }

    #[test]
    fn test_repeat_institution_clashes_between_adj_and_gov_are_collated() {
        let mut clash_map = ClashMap::new(Default::default());
        clash_map.add_clash_entry(
            Uuid::from_u128(600),
            Uuid::from_u128(700),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );
        clash_map.add_clash_entry(
            Uuid::from_u128(600),
            Uuid::from_u128(701),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 40,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );

        let ballot = DrawBallot {
            adjudicators: vec![DrawAdjudicator {
                uuid: Uuid::from_u128(600),
                ..Default::default()
            }
            .into()],
            government: Some(DrawTeam {
                members: vec![
                    DrawSpeaker {
                        uuid: Uuid::from_u128(700),
                        ..Default::default()
                    },
                    DrawSpeaker {
                        uuid: Uuid::from_u128(701),
                        ..Default::default()
                    },
                ],
                uuid: Uuid::from_u128(800),
                ..Default::default()
            }),
            ..Default::default()
        };

        let evaluator = DrawEvaluator::new(
            clash_map,
            DrawEvaluatorConfig {
                adj_team_clash_factor: 2.0,
                ..Default::default()
            }
        );

        let mut info: TournamentParticipantsInfo = TournamentParticipantsInfo::new();

        if let Some(g) = &ballot.government {
            info.team_members.insert(g.uuid, g.members.iter().map(|s| s.uuid).collect());
        }
        if let Some(o) = &ballot.opposition {
            info.team_members.insert(o.uuid, o.members.iter().map(|s| s.uuid).collect());
        }

        let issues = evaluator.find_issues_in_ballot(&ballot, &info);

        let adj_issues = issues
            .adjudicator_issues
            .get(&Uuid::from_u128(600))
            .unwrap();
        assert_eq!(adj_issues[0].severity, 180);
        match adj_issues[0].issue_type {
            ClashType::InstitutionalClash {
                severity,
                institution_id,
            } => {
                assert_eq!(severity, 90);
                assert_eq!(institution_id, Uuid::from_u128(100));
            }
            _ => panic!("Incorrect Clash typee"),
        }
        match &adj_issues[0].target {
            crate::draw::evaluation::DrawIssueTarget::Team {
                uuid: team_id,
                involved_speakers,
            } => {
                assert_eq!(
                    involved_speakers.iter().map(|u| *u).sorted().collect_vec(),
                    vec![Uuid::from_u128(700), Uuid::from_u128(701)]
                );
                assert_eq!(team_id, &Uuid::from_u128(800));
            }
            _ => panic!("Incorrect target type"),
        }

        assert_eq!(
            issues.government_issues,
            vec![DrawIssue {
                issue_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100)
                },
                severity: 180,
                target: crate::draw::evaluation::DrawIssueTarget::Adjudicator {
                    uuid: Uuid::from_u128(600)
                }
            }]
        );
    }

    #[test]
    fn test_finds_institution_clashes_between_gov_and_opp() {
        let mut clash_map = ClashMap::new(Default::default());
        clash_map.add_clash_entry(
            Uuid::from_u128(700),
            Uuid::from_u128(710),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );

        let ballot = DrawBallot {
            government: Some(DrawTeam {
                members: vec![DrawSpeaker {
                    uuid: Uuid::from_u128(700),
                    ..Default::default()
                }],
                uuid: Uuid::from_u128(800),
                ..Default::default()
            }),
            opposition: Some(DrawTeam {
                members: vec![DrawSpeaker {
                    uuid: Uuid::from_u128(710),
                    ..Default::default()
                }],
                uuid: Uuid::from_u128(801),
                ..Default::default()
            }),
            ..Default::default()
        };

        let evaluator = DrawEvaluator::new(
            clash_map,
            DrawEvaluatorConfig {
                team_team_clash_factor: 2.0,
                ..Default::default()
            }
        );

        let mut info: TournamentParticipantsInfo = TournamentParticipantsInfo::new();

        if let Some(g) = &ballot.government {
            info.team_members.insert(g.uuid, g.members.iter().map(|s| s.uuid).collect());
        }
        if let Some(o) = &ballot.opposition {
            info.team_members.insert(o.uuid, o.members.iter().map(|s| s.uuid).collect());
        }

        let issues = evaluator.find_issues_in_ballot(&ballot, &info);

        assert_eq!(
            issues.government_issues,
            vec![DrawIssue {
                issue_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100)
                },
                severity: 180,
                target: crate::draw::evaluation::DrawIssueTarget::Team {
                    uuid: Uuid::from_u128(801),
                    involved_speakers: vec![Uuid::from_u128(710)]
                }
            }]
        );
        assert_eq!(
            issues.opposition_issues,
            vec![DrawIssue {
                issue_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100)
                },
                severity: 180,
                target: crate::draw::evaluation::DrawIssueTarget::Team {
                    uuid: Uuid::from_u128(800),
                    involved_speakers: vec![Uuid::from_u128(700)]
                }
            }]
        );
    }

    #[test]
    fn test_finds_and_collates_institution_clashes_between_opp_and_non_aligned() {
        let mut clash_map = ClashMap::new(Default::default());
        clash_map.add_clash_entry(
            Uuid::from_u128(710),
            Uuid::from_u128(720),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );
        clash_map.add_clash_entry(
            Uuid::from_u128(720),
            Uuid::from_u128(711),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 10,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );

        let ballot = DrawBallot {
            opposition: Some(DrawTeam {
                members: vec![
                    DrawSpeaker {
                        uuid: Uuid::from_u128(710),
                        ..Default::default()
                    },
                    DrawSpeaker {
                        uuid: Uuid::from_u128(711),
                        ..Default::default()
                    },
                ],
                uuid: Uuid::from_u128(801),
                ..Default::default()
            }),
            non_aligned_speakers: vec![Some(DrawSpeaker {
                uuid: Uuid::from_u128(720),
                ..Default::default()
            })],
            ..Default::default()
        };

        let evaluator = DrawEvaluator::new(
            clash_map,
            DrawEvaluatorConfig {
                team_speaker_clash_factor: 2.0,
                ..Default::default()
            }
        );

        let mut info: TournamentParticipantsInfo = TournamentParticipantsInfo::new();

        if let Some(g) = &ballot.government {
            info.team_members.insert(g.uuid, g.members.iter().map(|s| s.uuid).collect());
        }
        if let Some(o) = &ballot.opposition {
            info.team_members.insert(o.uuid, o.members.iter().map(|s| s.uuid).collect());
        }

        let issues = evaluator.find_issues_in_ballot(&ballot, &info);

        assert_eq!(
            issues.opposition_issues,
            vec![DrawIssue {
                issue_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100)
                },
                severity: 180,
                target: crate::draw::evaluation::DrawIssueTarget::Speaker {
                    uuid: Uuid::from_u128(720)
                }
            }]
        );

        match issues
            .non_aligned_issues
            .get(&Uuid::from_u128(720))
            .unwrap()[0]
            .issue_type
        {
            ClashType::InstitutionalClash {
                severity,
                institution_id,
            } => {
                assert_eq!(severity, 90);
                assert_eq!(institution_id, Uuid::from_u128(100));
            }
            _ => panic!("Incorrect Clash typee"),
        }

        match &issues
            .non_aligned_issues
            .get(&Uuid::from_u128(720))
            .unwrap()[0]
            .target
        {
            crate::draw::evaluation::DrawIssueTarget::Team {
                uuid: team_id,
                involved_speakers,
            } => {
                assert_eq!(
                    involved_speakers.iter().map(|u| *u).sorted().collect_vec(),
                    vec![Uuid::from_u128(710), Uuid::from_u128(711)]
                );
                assert_eq!(team_id, &Uuid::from_u128(801));
            }
            _ => panic!("Incorrect target type"),
        }
    }
}
*/