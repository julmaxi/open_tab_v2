use std::{error::Error, collections::HashMap};

use migration::MigratorTrait;
use sea_orm::{Database, Statement};
use open_tab_entities::{prelude::*, domain::{self}, mock::{self, MockOption}, queries::{query_all_participant_roles, ParticipantRoundRole}, prelude::{BallotTeam, Speech, SpeechRole}, EntityGroup, Entity};
use sea_orm::prelude::*;


pub async fn set_up_db(with_mock_env: bool) -> Result<DatabaseConnection, Box<dyn Error>> {
    let db = Database::connect("sqlite::memory:").await?;
    migration::Migrator::up(&db, None).await.unwrap();
    let _r = db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "PRAGMA foreign_keys = ON;",
        vec![])
    ).await?;

    if with_mock_env {
        mock::make_mock_tournament_with_options(MockOption {
            deterministic_uuids: true,
            draw_debates: false,
            ..Default::default()
        }).save_all_and_log_for_tournament(&db, Uuid::from_u128(1)).await?;
    }
    Ok(db)
}

#[tokio::test]
async fn test_find_team_roles() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;

    let debate_1 = domain::debate::TournamentDebate {
        uuid: Uuid::from_u128(200),
        round_id: Uuid::from_u128(100),
        index: 0,
        ballot_id: Uuid::from_u128(300),
    };
    let ballot_1 = domain::ballot::Ballot {
        uuid: Uuid::from_u128(300),
        government: BallotTeam {
            team: Some(Uuid::from_u128(1000)),
            ..Default::default()
        },
        speeches: vec![
            Speech { speaker:Some(Uuid::from_u128(2000)), role: SpeechRole::Government, position: 0, scores: Default::default() },
        ],
        ..Default::default()
    };
    let debate_2 = domain::debate::TournamentDebate {
        uuid: Uuid::from_u128(201),
        round_id: Uuid::from_u128(101),
        index: 0,
        ballot_id: Uuid::from_u128(301),
    };
    let ballot_2 = domain::ballot::Ballot {
        uuid: Uuid::from_u128(301),
        opposition: BallotTeam {
            team: Some(Uuid::from_u128(1000)),
            ..Default::default()
        },
        ..Default::default()
    };

    let mut groups = EntityGroup::new();
    groups.add(Entity::TournamentDebate(debate_1));
    groups.add(Entity::TournamentDebate(debate_2));
    groups.add(Entity::Ballot(ballot_1));
    groups.add(Entity::Ballot(ballot_2));

    groups.save_all(&db).await?;

    let roles = query_all_participant_roles(&db, Uuid::from_u128(2000)).await?;

    let debate_1_role = roles.get(&Uuid::from_u128(100)).unwrap();
    assert_eq!(debate_1_role, &ParticipantRoundRole::TeamSpeaker { debate_uuid: Uuid::from_u128(200), role: open_tab_entities::prelude::SpeechRole::Government });
    let debate_2_role = roles.get(&Uuid::from_u128(101)).unwrap();
    assert_eq!(debate_2_role, &ParticipantRoundRole::TeamSpeaker { debate_uuid: Uuid::from_u128(201), role: open_tab_entities::prelude::SpeechRole::Opposition });

    Ok(())
}


#[tokio::test]
async fn test_find_non_aligned_roles() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;

    let debate_1 = domain::debate::TournamentDebate {
        uuid: Uuid::from_u128(200),
        round_id: Uuid::from_u128(100),
        index: 0,
        ballot_id: Uuid::from_u128(300),
    };
    let ballot_1 = domain::ballot::Ballot {
        uuid: Uuid::from_u128(300),
        speeches: vec![
            domain::ballot::Speech {
                position: 0,
                speaker: Some(Uuid::from_u128(2000)),
                role: open_tab_entities::prelude::SpeechRole::NonAligned,
                scores: HashMap::new(),
            },
        ],
        ..Default::default()
    };
    let debate_2 = domain::debate::TournamentDebate {
        uuid: Uuid::from_u128(201),
        round_id: Uuid::from_u128(101),
        index: 0,
        ballot_id: Uuid::from_u128(301),
    };
    let ballot_2 = domain::ballot::Ballot {
        uuid: Uuid::from_u128(301),
        speeches: vec![
            domain::ballot::Speech {
                position: 1,
                speaker: Some(Uuid::from_u128(2000)),
                role: open_tab_entities::prelude::SpeechRole::NonAligned,
                scores: HashMap::new(),
            },
        ],
        ..Default::default()
    };

    let mut groups = EntityGroup::new();
    groups.add(Entity::TournamentDebate(debate_1));
    groups.add(Entity::TournamentDebate(debate_2));
    groups.add(Entity::Ballot(ballot_1));
    groups.add(Entity::Ballot(ballot_2));

    groups.save_all(&db).await?;

    let roles = query_all_participant_roles(&db, Uuid::from_u128(2000)).await?;

    let debate_1_role = roles.get(&Uuid::from_u128(100)).unwrap();
    assert_eq!(debate_1_role, &ParticipantRoundRole::NonAlignedSpeaker { debate_uuid: Uuid::from_u128(200), position: 0 });
    let debate_2_role = roles.get(&Uuid::from_u128(101)).unwrap();
    assert_eq!(debate_2_role, &ParticipantRoundRole::NonAlignedSpeaker { debate_uuid: Uuid::from_u128(201), position: 1 });

    Ok(())
}


#[tokio::test]
async fn test_find_adjudicator_roles() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;

    let debate_1 = domain::debate::TournamentDebate {
        uuid: Uuid::from_u128(200),
        round_id: Uuid::from_u128(100),
        index: 0,
        ballot_id: Uuid::from_u128(300),
    };
    let ballot_1 = domain::ballot::Ballot {
        uuid: Uuid::from_u128(300),
        adjudicators: vec![
            Uuid::from_u128(3000)
        ],
        ..Default::default()
    };
    let debate_2 = domain::debate::TournamentDebate {
        uuid: Uuid::from_u128(201),
        round_id: Uuid::from_u128(101),
        index: 0,
        ballot_id: Uuid::from_u128(301),
    };
    let ballot_2 = domain::ballot::Ballot {
        uuid: Uuid::from_u128(301),
        president: Some(Uuid::from_u128(3000)),
        ..Default::default()
    };

    let mut groups = EntityGroup::new();
    groups.add(Entity::TournamentDebate(debate_1));
    groups.add(Entity::TournamentDebate(debate_2));
    groups.add(Entity::Ballot(ballot_1));
    groups.add(Entity::Ballot(ballot_2));

    groups.save_all(&db).await?;

    let roles = query_all_participant_roles(&db, Uuid::from_u128(3000)).await?;

    let debate_1_role = roles.get(&Uuid::from_u128(100)).unwrap();
    assert_eq!(debate_1_role, &ParticipantRoundRole::Adjudicator { debate_uuid: Uuid::from_u128(200), position: 0 });
    let debate_2_role = roles.get(&Uuid::from_u128(101)).unwrap();
    assert_eq!(debate_2_role, &ParticipantRoundRole::President { debate_uuid: Uuid::from_u128(201) });

    Ok(())
}

#[tokio::test]
async fn test_find_no_role() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;

    let debate_1 = domain::debate::TournamentDebate {
        uuid: Uuid::from_u128(200),
        round_id: Uuid::from_u128(100),
        index: 0,
        ballot_id: Uuid::from_u128(300),
    };
    let ballot_1 = domain::ballot::Ballot {
        uuid: Uuid::from_u128(300),
        adjudicators: vec![
            Uuid::from_u128(3000)
        ],
        ..Default::default()
    };
    let debate_2 = domain::debate::TournamentDebate {
        uuid: Uuid::from_u128(201),
        round_id: Uuid::from_u128(101),
        index: 0,
        ballot_id: Uuid::from_u128(301),
    };
    let ballot_2 = domain::ballot::Ballot {
        uuid: Uuid::from_u128(301),
        ..Default::default()
    };

    let mut groups = EntityGroup::new();
    groups.add(Entity::TournamentDebate(debate_1));
    groups.add(Entity::TournamentDebate(debate_2));
    groups.add(Entity::Ballot(ballot_1));
    groups.add(Entity::Ballot(ballot_2));

    groups.save_all(&db).await?;

    let roles = query_all_participant_roles(&db, Uuid::from_u128(3000)).await?;

    let debate_2_role = roles.get(&Uuid::from_u128(101)).unwrap();
    assert_eq!(debate_2_role, &ParticipantRoundRole::None);

    Ok(())
}

#[tokio::test]
async fn test_multiple_roles() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;

    let debate_1 = domain::debate::TournamentDebate {
        uuid: Uuid::from_u128(200),
        round_id: Uuid::from_u128(100),
        index: 0,
        ballot_id: Uuid::from_u128(300),
    };
    let ballot_1 = domain::ballot::Ballot {
        uuid: Uuid::from_u128(300),
        speeches: vec![
            domain::ballot::Speech {
                position: 0,
                speaker: Some(Uuid::from_u128(2000)),
                role: open_tab_entities::prelude::SpeechRole::NonAligned,
                scores: HashMap::new(),
            },
        ],
        government: BallotTeam {
            team: Some(Uuid::from_u128(1000)),
            ..Default::default()
        },
        ..Default::default()
    };

    let mut groups = EntityGroup::new();
    groups.add(Entity::TournamentDebate(debate_1));
    groups.add(Entity::Ballot(ballot_1));

    groups.save_all(&db).await?;

    let roles = query_all_participant_roles(&db, Uuid::from_u128(2000)).await?;

    let debate_1_role = roles.get(&Uuid::from_u128(100)).unwrap();
    assert_eq!(debate_1_role, &ParticipantRoundRole::Multiple);

    Ok(())
}
