use std::collections::HashMap;

use crate::{derived_models::get_participant_public_name, domain::entity::LoadEntity, info::TournamentParticipantsInfo, prelude::{Participant, Team}};

use super::{base::{SpeakerTabEntry, SpeakerTabEntryDetailedScore, TabView, TeamTabEntry, TeamTabEntryDetailedScore}, BreakRelevantTabView};
use sea_orm::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentedTabView {
    pub num_rounds: u32,
    pub team_tab: Vec<AugmentedTeamTabEntry>,
    pub speaker_tab: Vec<AugmentedSpeakerTabEntry>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentedTeamTabEntry {
    pub rank: u32,
    pub team_name: String,
    pub team_uuid: Uuid,
    pub total_score: f64,
    pub avg_score: Option<f64>,
    pub detailed_scores: Vec<Option<TeamTabEntryDetailedScore>>,
    pub member_ranks: Vec<u32>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentedSpeakerTabEntry {
    pub rank: u32,
    pub speaker_uuid: Uuid,
    pub team_uuid: Uuid,
    pub speaker_name: String,
    pub team_name: String,
    pub total_score: f64,
    pub avg_score: Option<f64>,
    pub detailed_scores: Vec<Option<SpeakerTabEntryDetailedScore>>,
    pub is_anonymous: bool,
}

impl AugmentedTabView {
    pub async fn load_from_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let tab = TabView::load_from_tournament(db, tournament_id).await?;
        let info: TournamentParticipantsInfo = TournamentParticipantsInfo::load(db, tournament_id).await?;

        Ok(
            AugmentedTabView::from_tab_view(&tab, &info.teams_by_id, &info.participants_by_id, false)
        )
    }

    pub fn from_tab_view(tab: &TabView, teams: &HashMap<Uuid, Team>, participants: &HashMap<Uuid, Participant>, respect_anonymity: bool) -> Self {
        AugmentedTabView {
            num_rounds: tab.num_rounds,
            team_tab: tab.team_tab.iter().map(|e| AugmentedTeamTabEntry::from_tab_entry(e, &teams)).collect(),
            speaker_tab: tab.speaker_tab.iter().map(|e| AugmentedSpeakerTabEntry::from_tab_entry(e, &teams, &participants, respect_anonymity)).collect()
        }
    }
}

impl AugmentedSpeakerTabEntry {
    pub fn from_tab_entry(entry: &SpeakerTabEntry, teams: &HashMap<Uuid, Team>, participants: &HashMap<Uuid, Participant>, respect_anonymity: bool) -> Self {
        let (speaker_name, is_anonymous, team_name) = if let Some(p) = participants.get(&entry.speaker_uuid) {
            let team = teams.get(&entry.team_uuid);
            (if respect_anonymity {get_participant_public_name(p)} else { p.name.clone() }, p.is_anonymous, team.map(|t| t.name.clone()).unwrap_or_else(|| "<Unknown Team>".into()))
        }
        else {
            ("<Unknown Participant>".into(), false, "<Unknown Team>".into())
        };
        
        AugmentedSpeakerTabEntry {
            rank: entry.rank,
            speaker_uuid: entry.speaker_uuid,
            team_uuid: entry.team_uuid,
            total_score: entry.total_score,
            avg_score: entry.avg_score,
            detailed_scores: entry.detailed_scores.clone(),
            is_anonymous,
            team_name,
            speaker_name
        }
    }
}

impl AugmentedTeamTabEntry {
    pub fn from_tab_entry(entry: &TeamTabEntry, teams: &HashMap<Uuid, Team>)  -> Self {
        let team_name = if let Some(t) = teams.get(&entry.team_uuid) {
            t.name.clone()
        }
        else {
            "<Unknown Team>".into()
        };
        AugmentedTeamTabEntry {
            rank: entry.rank,
            team_name,
            team_uuid: entry.team_uuid,
            total_score: entry.total_score,
            avg_score: entry.avg_score,
            detailed_scores: entry.detailed_scores.clone(),
            member_ranks: entry.member_ranks.clone()
        }
    }
}


#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct BreakingAdjudicatorInfo {
    pub name: String,
    pub uuid: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentedBreakRelevantTabView {
    pub tab: AugmentedTabView,
    pub speaker_teams: HashMap<Uuid, Uuid>,
    pub team_members: HashMap<Uuid, Vec<Uuid>>,
    pub breaking_teams: Vec<Uuid>,
    pub breaking_speakers: Vec<Uuid>,
    pub breaking_adjudicators: Vec<BreakingAdjudicatorInfo>
}

impl AugmentedBreakRelevantTabView {
    pub async fn load_from_node<C>(db: &C, node_uuid: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let target_node = crate::domain::tournament_plan_node::TournamentPlanNode::get(db, node_uuid).await?;
        let tab = BreakRelevantTabView::load_from_node(db, node_uuid).await?;
        let info: TournamentParticipantsInfo = TournamentParticipantsInfo::load(db, target_node.tournament_id).await?;

        Ok(
            AugmentedBreakRelevantTabView::from_break_relevant_tab(&tab, &info.teams_by_id, &info.participants_by_id, false)
        )
    }
    pub fn from_break_relevant_tab(tab: &BreakRelevantTabView, teams: &HashMap<Uuid, Team>, participants: &HashMap<Uuid, Participant>, respect_anonymity: bool) -> Self {
        AugmentedBreakRelevantTabView {
            tab: AugmentedTabView::from_tab_view(&tab.tab, teams, participants, respect_anonymity),
            speaker_teams: tab.speaker_teams.clone(),
            team_members: tab.team_members.clone(),
            breaking_teams: tab.breaking_teams.clone(),
            breaking_speakers: tab.breaking_speakers.clone(),
            breaking_adjudicators: tab.breaking_adjudicators.iter().map(|a| {
                let adj = participants.get(&a);
                let name = if let Some(adj) = adj {
                    if respect_anonymity {
                        get_participant_public_name(adj)
                    }
                    else {
                        adj.name.clone()
                    }
                }
                else {
                    "<Unknown Adjudicator>".into()
                };

                BreakingAdjudicatorInfo {
                    uuid: *a,
                    name
                }
            }).collect()
        }
    }
}

