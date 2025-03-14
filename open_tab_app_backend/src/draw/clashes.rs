use std::collections::HashMap;
use itertools::Itertools;
use sea_orm::prelude::*;

use open_tab_entities::{domain::participant_clash::ParticipantClash, prelude::{Participant, Ballot, SpeechRole}, domain::participant::ParticipantRole};
use serde::{Serialize, Deserialize};



#[derive(Debug, Clone)]
pub struct ClashMap {
    pub clashes: Vec<Vec<ClashMapEntry>>,
    pub clash_matrix: HashMap<Uuid, HashMap<Uuid, usize>>,
    config: ClashMapConfig,
    empty: Vec<ClashMapEntry>
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClashMapEntry {
    pub clash_type: ClashType
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClashType {
    TeamSpeakerHasSeenTeamSpeaker{round: Uuid},
    TeamSpeakerHasSeenNonAlignedSpeaker{round: Uuid},
    NonAlignedSpeakerHasSeenNonAlignedSpeaker{round: Uuid},
    JudgeHasSeenTeamSpeaker{round: Uuid},
    JudgeHasSeenNonAlignedSpeaker{round: Uuid},
    JudgeHasSeenJudge{round: Uuid},
    DeclaredClash{severity: u16},
    InstitutionalClash{severity: u16, institution_id: Uuid},
    SameTeamClash
}

#[derive(Debug, Clone)]
pub struct ClashMapConfig {
    pub ignore_speaker_adj_clashes: bool,
}

impl Default for ClashMapConfig {
    fn default() -> Self {
        ClashMapConfig {
            ignore_speaker_adj_clashes: false,
        }
    }
}


impl ClashMap {
    pub fn new(config: ClashMapConfig) -> Self {
        ClashMap {
            clash_matrix: HashMap::new(),
            clashes: Vec::new(),
            config,
            empty: Vec::new()
        }
    }

    pub async fn new_for_tournament<C>(config: ClashMapConfig, tournament_id: Uuid, db: &C) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut clash_map = ClashMap::new(config);

        let all_clashes = ParticipantClash::get_all_in_tournament(db, tournament_id).await?;
        let all_participants_by_id = Participant::get_all_in_tournament(db, tournament_id).await?.into_iter().map(|p| (p.uuid, p)).collect::<HashMap<_, _>>();

        for clash in all_clashes.into_iter() {
            let declaring_participant = all_participants_by_id.get(&clash.declaring_participant_id);
            let target_participant = all_participants_by_id.get(&clash.target_participant_id);

            match (declaring_participant, target_participant) {
                (Some(declaring_participant), Some(target_participant)) => clash_map.add_declared_clash(declaring_participant, target_participant, clash),
                _ => {}
            }
        }

        let institution_buckets = all_participants_by_id.values().flat_map(|p| p.institutions.iter().map(|i| (i.uuid, (p.uuid, i.clash_severity)))).into_group_map();

        institution_buckets.into_iter().for_each(|(institution_id, participants)| {
            participants.iter().combinations(2).for_each(|pair| {
                let (p1, s1) = pair[0];
                let (p2, s2) = pair[1];

                let clash_entry = ClashMapEntry {
                    clash_type: ClashType::InstitutionalClash { severity: (s1 + s2) / 2, institution_id },
                };

                clash_map.add_clash_entry(*p1, *p2, clash_entry);
            });
        });

        let teams = all_participants_by_id.iter().filter_map(
            |p| {
                match &p.1.role {
                    ParticipantRole::Speaker(speaker) => if let Some(team_id) = speaker.team_id {
                        Some((team_id, p.1.uuid))
                    } else {
                        None
                    },
                    _ => None
                }
            }
        ).into_group_map();

        teams.into_values().flat_map(|members| members.into_iter().combinations(2)).for_each(
            |pair| {
                let clash_entry = ClashMapEntry {
                    clash_type: ClashType::SameTeamClash,
                };

                clash_map.add_clash_entry(pair[0], pair[1], clash_entry);
            }
        );

        Ok(clash_map)
    }

    pub fn add_dynamic_clashes_from_round_ballots(&mut self, round_draws: Vec<&(Uuid, Vec<Ballot>)>, team_members: &HashMap<Uuid, Vec<Uuid>>) -> Result<(), anyhow::Error> {
        for (round_id, ballots) in round_draws {
            for ballot in ballots {
                for adj_pair in ballot.adjudicators.iter().combinations(2) {
                    let clash_entry = ClashMapEntry {
                        clash_type: ClashType::JudgeHasSeenJudge{round: *round_id},
                    };

                    self.add_clash_entry(*adj_pair[0], *adj_pair[1], clash_entry);
                }

                let all_gov_speaker_uuids = ballot.government.team.clone().map(|t| team_members.get(&t)).flatten().map(|m| m.clone()).unwrap_or(Vec::new());
                let all_opp_speaker_uuids = ballot.opposition.team.clone().map(|t| team_members.get(&t)).flatten().map(|m| m.clone()).unwrap_or(Vec::new());
                let all_non_aligned_speaker_uuids = ballot.speeches.iter().filter_map(|s| match s.role {
                    SpeechRole::NonAligned => s.speaker,
                    _ => None
                }).collect_vec();

                for (gov_speaker, opp_speaker) in all_gov_speaker_uuids.iter().cartesian_product(all_opp_speaker_uuids.iter()) {
                    let clash_entry = ClashMapEntry {
                        clash_type: ClashType::TeamSpeakerHasSeenTeamSpeaker{round: *round_id},
                    };

                    self.add_clash_entry(*gov_speaker, *opp_speaker, clash_entry);
                }

                for (gov_speaker, non_aligned_speaker) in all_gov_speaker_uuids.iter().cartesian_product(all_non_aligned_speaker_uuids.iter()) {
                    let clash_entry = ClashMapEntry {
                        clash_type: ClashType::TeamSpeakerHasSeenNonAlignedSpeaker{round: *round_id},
                    };

                    self.add_clash_entry(*gov_speaker, *non_aligned_speaker, clash_entry);
                }

                for (opp_speaker, non_aligned_speaker) in all_opp_speaker_uuids.iter().cartesian_product(all_non_aligned_speaker_uuids.iter()) {
                    let clash_entry = ClashMapEntry {
                        clash_type: ClashType::TeamSpeakerHasSeenNonAlignedSpeaker{round: *round_id},
                    };

                    self.add_clash_entry(*opp_speaker, *non_aligned_speaker, clash_entry);
                }

                let all_team_speaker_uuids = all_gov_speaker_uuids.into_iter().chain(all_opp_speaker_uuids.into_iter());

                for (speaker, adj) in all_team_speaker_uuids.cartesian_product(ballot.adjudicators.iter().map(|a| a)) {
                    let clash_entry = ClashMapEntry {
                        clash_type: ClashType::JudgeHasSeenTeamSpeaker { round: *round_id }
                    };

                    self.add_clash_entry(speaker, *adj, clash_entry);
                }

                for (speaker, adj) in all_non_aligned_speaker_uuids.iter().cartesian_product(ballot.adjudicators.iter().map(|a| a)) {
                    let clash_entry = ClashMapEntry {
                        clash_type: ClashType::JudgeHasSeenNonAlignedSpeaker { round: *round_id }
                    };

                    self.add_clash_entry(*speaker, *adj, clash_entry);
                }
            }
        }

        Ok(())
    }

    pub fn add_declared_clash(&mut self, declaring_participant: &Participant, target_participant: &Participant, clash: ParticipantClash) {
        if self.config.ignore_speaker_adj_clashes && matches!(declaring_participant.role, ParticipantRole::Speaker(_)) && matches!(target_participant.role, ParticipantRole::Adjudicator(_)) {
            return;
        }
        else {
            let clash_entry = ClashMapEntry {
                clash_type: ClashType::DeclaredClash{severity: clash.clash_severity},
            };

            self.add_clash_entry_for_participants(declaring_participant, target_participant, clash_entry);
        }
    }

    pub fn add_clash_entry_for_participants(&mut self, declaring_participant: &Participant, target_participant: &Participant, clash_entry: ClashMapEntry) {
        self.add_clash_entry(declaring_participant.uuid, target_participant.uuid, clash_entry);
    }

    pub fn add_clash_entry(&mut self, declaring_participant: Uuid, target_participant: Uuid, clash_entry: ClashMapEntry) {
        let (first_uuid, second_uuid) = if declaring_participant < target_participant {
            (declaring_participant, target_participant)
        }
        else {
            (declaring_participant, target_participant)
        };

        if let Some(vec_idx) = self.clash_matrix.get(&first_uuid).unwrap_or(&HashMap::new()).get(&second_uuid) {
            self.clashes[*vec_idx].push(clash_entry);
        }
        else {
            let vec_idx = self.clashes.len();
            self.clashes.push(vec![clash_entry]);
            self.clash_matrix.entry(first_uuid).or_insert_with(HashMap::new).insert(second_uuid, vec_idx);
            self.clash_matrix.entry(second_uuid).or_insert_with(HashMap::new).insert(first_uuid, vec_idx);
        }
    }

    pub fn get_clashes_for_participant(&self, uuid: &Uuid) -> HashMap<Uuid, &Vec<ClashMapEntry>> {
        self.clash_matrix.get(
            uuid
        ).unwrap_or(
            &HashMap::new()
        ).iter().map(
            |(k, v)| (*k, &self.clashes[*v])
        ).collect()
    }
    
    pub fn get_clashes_between_participants(&self, first_uuid: &Uuid, second_uuid: &Uuid) -> &Vec<ClashMapEntry> {
        self.clash_matrix.get(first_uuid).and_then(|m| m.get(second_uuid)).map(|v| &self.clashes[*v]).unwrap_or(&self.empty)
    }
}