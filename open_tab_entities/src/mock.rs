
use std::{collections::HashMap};

use crate::{EntityGroup, domain::{tournament::Tournament, participant::ParticipantInstitution, participant_clash::ParticipantClash, feedback_question::FeedbackQuestion, feedback_form::{FeedbackForm, FeedbackFormVisibility}}};
use sea_orm::{prelude::*};
use crate::prelude::*;
use itertools::{Itertools};


use faker_rand::en_us::{names::FullName, company::CompanyName};

use crate::group::EntityGroupTrait;

use crate::domain::round::DrawType;


#[derive(Debug)]
pub struct MockOption {
    pub deterministic_uuids: bool,
    pub num_teams: u32,
    pub num_adjudicators: u32,
    pub draw_debates: bool,
    pub use_random_names: bool,
    pub use_feedback: bool,

}

impl Default for MockOption {
    fn default() -> Self {
        Self {
            deterministic_uuids: false,
            num_teams: 27,
            num_adjudicators: 27,
            draw_debates: true,
            use_random_names: false,
            use_feedback: true
        }
    }
}

pub fn make_mock_tournament() -> EntityGroup {
    return make_mock_tournament_with_options(Default::default());
}

pub fn make_mock_tournament_with_options(options: MockOption) -> EntityGroup {
    /*
    Tournament: 1
    Teams: 1000
    Speakers: 2000
    Adjudicators: 3000
    Rounds: 100
    Debates: 200
    Ballots: 400
    Venues: 500
    Feedback Questions: 600
    Feedback Forms: 700
    */

    assert!(options.num_teams % 3 == 0);
    assert!(options.num_teams >= 3);
    let mut groups = EntityGroup::new();

    let tournament_uuid = if options.deterministic_uuids {Uuid::from_u128(1)} else {Uuid::new_v4()};
    groups.add(Entity::Tournament(Tournament {
        uuid: tournament_uuid,
        ..Default::default()
    }));

    let institutions = (0..options.num_teams).map(|i| {
        let uuid = if options.deterministic_uuids {Uuid::from_u128(500 + i as u128)} else {Uuid::new_v4()};
        
        let name = if options.use_random_names {
            rand::random::<CompanyName>().to_string()
        }
        else {
            format!("Institution {}", uuid)
        };
        crate::domain::tournament_institution::TournamentInstitution {
            uuid,
            name,
            tournament_id: tournament_uuid,
        }
    }).collect_vec();

    let venues = (0..options.num_teams / 3).map(|i| {
        let uuid = if options.deterministic_uuids {Uuid::from_u128(500 + i as u128)} else {Uuid::new_v4()};
        
        let name = if options.use_random_names {
            rand::random::<CompanyName>().to_string()
        }
        else {
            format!("Venue {}", i)
        };
        crate::domain::tournament_venue::TournamentVenue {
            uuid,
            name,
            tournament_id: tournament_uuid,
        }
    }).collect_vec();
    
    let teams = (0..options.num_teams).map(|i| {
        let uuid = if options.deterministic_uuids {Uuid::from_u128(1000 + i as u128)} else {Uuid::new_v4()};
        let name = format!("Team {}", i);
        crate::domain::team::Team {
            uuid,
            name,
            tournament_id: tournament_uuid,
        }
    }).collect_vec();

    let speakers = teams.iter().enumerate().map(|(team_idx, team)| {
        let members = (0..3).map(|i| {
            let uuid = if options.deterministic_uuids {Uuid::from_u128(2000 + (team_idx as u128) * 10 + i)} else {Uuid::new_v4()};

            let name = if options.use_random_names {
                rand::random::<FullName>().to_string()
            }
            else {
                format!("Speaker {}", uuid)
            };
            let mut institutions = if options.deterministic_uuids {
                vec![
                    ParticipantInstitution {
                        uuid: Uuid::from_u128(500 + team_idx as u128),
                        clash_severity: 100
                    }
                ]
            }
            else {
                vec![]
            };
            if options.deterministic_uuids && i == 1 {
                institutions.push(
                    ParticipantInstitution {
                        uuid: if team_idx == 0 {Uuid::from_u128(500 + team_idx as u128 + 1)} else {Uuid::from_u128(500 + (team_idx - 1) as u128)},
                        clash_severity: 50
                    }
                );    
            }
            let mut registration_key = [0; 32];
            registration_key[0] = 1;
            registration_key[1] = 2;
            Participant {
                uuid,
                name,
                tournament_id: tournament_uuid,
                role: ParticipantRole::Speaker(Speaker { team_id: Some(team.uuid) }),
                institutions,
                registration_key: Some(registration_key.to_vec())
            }
        }).collect_vec();

        members
    }).collect_vec();

    let adjudicators = (0..options.num_adjudicators).map(|adj_idx| {
        let uuid = if options.deterministic_uuids {Uuid::from_u128(3000 + adj_idx as u128)} else {Uuid::new_v4()};
        let name = if options.use_random_names {
            rand::random::<FullName>().to_string()
        }
        else {
            format!("Adjudicator {}", uuid)
        };

        let institutions = if options.deterministic_uuids {
            vec![ParticipantInstitution {
                uuid: Uuid::from_u128(500 + adj_idx as u128),
                clash_severity: 100
            }]
        }
        else {
            vec![]
        };
        let mut registration_key = [0; 32];
        registration_key[0] = 1;
        registration_key[1] = 2;

        Participant {
            uuid,
            name,
            tournament_id: tournament_uuid,
            role: ParticipantRole::Adjudicator(Adjudicator {..Default::default() }),
            institutions,
            registration_key: Some(registration_key.to_vec())
        }
    }).collect_vec();

    let rounds = (0..3).map(|i| {
        let uuid = if options.deterministic_uuids {Uuid::from_u128(100 + i as u128)} else {Uuid::new_v4()};
        TournamentRound {
            uuid,
            tournament_id: tournament_uuid,
            index: i as u64,
            draw_type: Some(DrawType::StandardPreliminaryDraw),
            draw_release_time: if i == 0 {Some(chrono::Utc::now().naive_utc())} else {None},
            ..Default::default()
        }
    }).collect_vec();

    let clashes = if options.deterministic_uuids {
        vec![
            ParticipantClash {
                uuid:Uuid::from_u128(600),
                clash_severity:100,
                declaring_participant_id: adjudicators[0].uuid,
                target_participant_id: speakers[0][0].uuid,
            },
            ParticipantClash {
                uuid:Uuid::from_u128(601),
                clash_severity:100,
                declaring_participant_id: speakers[0][1].uuid,
                target_participant_id: adjudicators[1].uuid,
            },
            ParticipantClash {
                uuid:Uuid::from_u128(602),
                clash_severity:50,
                declaring_participant_id: speakers[1][1].uuid,
                target_participant_id: adjudicators[0].uuid,
            },
            ParticipantClash {
                uuid:Uuid::from_u128(603),
                clash_severity:25,
                declaring_participant_id: speakers[1][2].uuid,
                target_participant_id: adjudicators[0].uuid,
            },
        ]
    } else {
        vec![]
    };

    if options.use_feedback {
        let q1_id = if options.deterministic_uuids {Uuid::from_u128(600)} else {Uuid::new_v4()};

        let questions = vec![
            FeedbackQuestion {
                uuid: q1_id,
                short_name: "skill".into(),
                full_name: "Wie würdest du insgesamt die Kompetenz dieser JurorIn bewerten?".into(),
                description: "".into(),
                question_config: crate::domain::feedback_question::QuestionType::RangeQuestion{config: crate::domain::feedback_question::RangeQuestionConfig {
                    min: 0,
                    max: 100,
                    orientation: crate::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
                    labels: vec![
                        (0, "Sehr schlecht".into()),
                        (100, "Sehr gut".into()),
                    ] }
                },
                tournament_id: Some(tournament_uuid),
            },
        ];

        let forms = vec![
            FeedbackForm {
                uuid: if options.deterministic_uuids {Uuid::from_u128(700)} else {Uuid::new_v4()},
                name: "General Feedback".to_string(),
                visibility: FeedbackFormVisibility::all(),
                tournament_id: Some(tournament_uuid),
                questions: vec![
                    q1_id
                ]
            }
        ];

        questions.into_iter().for_each(|q| {
            groups.add(Entity::FeedbackQuestion(q));
        });

        forms.into_iter().for_each(|f| {
            groups.add(Entity::FeedbackForm(f));
        });
    }

    if options.draw_debates {
        let ballots = rounds.iter().enumerate().map(
            |(round_idx, _round)| {
                (0..9).map(
                    |debate_idx| {
                        let uuid = if options.deterministic_uuids {Uuid::from_u128(400 + (round_idx as u128) * 10 + debate_idx as u128)} else {Uuid::new_v4()};
                        let mut  speeches = vec![
                            (crate::domain::ballot::SpeechRole::Government),
                            (crate::domain::ballot::SpeechRole::Opposition),
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

                        if round_idx < 2 {
                            speeches.extend((0..3).map(|speaker_idx| {
                                Speech {
                                    speaker:Some(speakers[debate_idx * 3 + 2][speaker_idx].uuid),
                                    role: crate::domain::ballot::SpeechRole::NonAligned,
                                    position: speaker_idx as u8,
                                    scores: HashMap::new(),
                                }
                            }))
                        }
                        Ballot {
                            uuid,
                            adjudicators: if round_idx < 2 {(0..3).map(|i| adjudicators[debate_idx * 3 + i].uuid).collect()} else {vec![]},
                            government: BallotTeam {
                                // Round 3 has an empty draw for testing purposes
                                team: if round_idx < 2 {Some(teams[debate_idx * 3].uuid)} else {None},
                                ..Default::default()
                            },
                            opposition: BallotTeam {
                                team: if round_idx < 2 {Some(teams[debate_idx * 3 + 1].uuid)} else {None},
                                ..Default::default()
                            },
                            speeches,
                            ..Default::default()
                        }
                    }
                ).collect_vec()
        }).collect_vec();
    
        let debates = ballots.iter().enumerate().map(|(round_idx, round_ballots)| {
            let round_debates = round_ballots.iter().enumerate().map(|(debate_idx, ballot)| {
                let uuid = if options.deterministic_uuids {Uuid::from_u128(200 + debate_idx as u128 + 10 * round_idx as u128)} else {Uuid::new_v4()};
                TournamentDebate {
                    uuid,
                    round_id: rounds[round_idx].uuid,
                    ballot_id: ballot.uuid,
                    index: debate_idx as u64,
                    venue_id: Some(venues[debate_idx].uuid),
                    is_motion_released_to_non_aligned: false
                }
            }).collect_vec();
            round_debates
        }).collect_vec();

        ballots.into_iter().flatten().for_each(|ballot| groups.add(Entity::Ballot(ballot)));
        debates.into_iter().flatten().for_each(|debate| groups.add(Entity::TournamentDebate(debate)));    
    }

    teams.into_iter().for_each(|team| groups.add(Entity::Team(team)));
    speakers.into_iter().flatten().for_each(|speaker| groups.add(Entity::Participant(speaker)));
    adjudicators.into_iter().for_each(|adjudicator| groups.add(Entity::Participant(adjudicator)));
    rounds.into_iter().for_each(|round| groups.add(Entity::TournamentRound(round)));
    institutions.into_iter().for_each(|i| groups.add(Entity::TournamentInstitution(i)));
    clashes.into_iter().for_each(|c| groups.add(Entity::ParticipantClash(c)));
    venues.into_iter().for_each(|v| groups.add(Entity::TournamentVenue(v)));

    groups
}