mod common;


use common::set_up_db;
use open_tab_entities::{domain::{feedback_form::{FeedbackForm, FeedbackFormVisibility}, entity::LoadEntity, feedback_question::{FeedbackQuestion, QuestionType, RangeQuestionConfig}, feedback_response::{FeedbackResponse, FeedbackResponseValue}}, prelude::TournamentEntity};
use uuid::Uuid;


#[tokio::test]
async fn test_save_form_without_questions_roundtrip() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;

    let form = FeedbackForm {
        uuid: Uuid::from_u128(6000),
        name: "Test Form".into(),
        
        visibility: FeedbackFormVisibility {
            show_non_aligned_for_chairs: true,
            show_teams_for_chairs: true,
            ..Default::default()
        },

        tournament_id: Some(Uuid::from_u128(1)),
        questions: vec![],
    };

    form.save(&db, true).await?;

    let loaded_form = FeedbackForm::get(&db, Uuid::from_u128(6000)).await?;

    assert_eq!(loaded_form, form);

    Ok(())
}


#[tokio::test]
async fn test_save_form_with_questions_roundtrip() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;

    let q1 = FeedbackQuestion {
        uuid: Uuid::from_u128(5001),
        short_name: "test1".into(),
        full_name: "Test Question 1".into(),
        description: "abc".into(),
        question_config: QuestionType::RangeQuestion{config: RangeQuestionConfig {
            min: 0,
            max: 100,
            orientation: open_tab_entities::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
            ..Default::default()
        }},
        tournament_id: Some(Uuid::from_u128(1)),
        is_confidential: false,
    };
    let q2 = FeedbackQuestion {
        uuid: Uuid::from_u128(5000),
        short_name: "test2".into(),
        full_name: "Test Question 1".into(),
        description: "abc".into(),
        question_config: QuestionType::RangeQuestion{config: RangeQuestionConfig {
            min: 0,
            max: 100,
            orientation: open_tab_entities::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
            ..Default::default()
        }},
        tournament_id: Some(Uuid::from_u128(1)),
        is_confidential: false,
    };

    q1.save(&db, true).await?;
    q2.save(&db, true).await?;

    let form = FeedbackForm {
        uuid: Uuid::from_u128(6000),
        name: "Test Form".into(),
        
        visibility: FeedbackFormVisibility { show_chairs_for_wings: true, ..Default::default() },
        tournament_id: Some(Uuid::from_u128(1)),
        questions: vec![
            q1.uuid,
            q2.uuid,
        ],
    };

    form.save(&db, true).await?;

    let loaded_form = FeedbackForm::get(&db, Uuid::from_u128(6000)).await?;

    assert_eq!(form, loaded_form);

    Ok(())
}



#[tokio::test]
async fn test_save_form_change_question_order() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;

    let q1 = FeedbackQuestion {
        uuid: Uuid::from_u128(5001),
        short_name: "test1".into(),
        full_name: "Test Question 1".into(),
        description: "abc".into(),
        question_config: QuestionType::RangeQuestion{config: RangeQuestionConfig {
            min: 0,
            max: 100,
            orientation: open_tab_entities::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
            ..Default::default()
        }},
        tournament_id: Some(Uuid::from_u128(1)),
        is_confidential: false,
    };
    let q2 = FeedbackQuestion {
        uuid: Uuid::from_u128(5000),
        short_name: "test2".into(),
        full_name: "Test Question 1".into(),
        description: "abc".into(),
        question_config: QuestionType::RangeQuestion{config: RangeQuestionConfig {
            min: 0,
            max: 100,
            orientation: open_tab_entities::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
            ..Default::default()
        }},
        tournament_id: Some(Uuid::from_u128(1)),
        is_confidential: false,
    };

    q1.save(&db, true).await?;
    q2.save(&db, true).await?;

    let mut form = FeedbackForm {
        uuid: Uuid::from_u128(6000),
        name: "Test Form".into(),
        
        visibility: FeedbackFormVisibility {
            show_non_aligned_for_chairs: true,
            show_teams_for_chairs: true,
            ..Default::default()
        },
        tournament_id: Some(Uuid::from_u128(1)),
        questions: vec![
            q1.uuid,
            q2.uuid,
        ],
    };

    form.save(&db, true).await?;

    form.questions = vec![
        q2.uuid,
        q1.uuid,
    ];
    form.save(&db, false).await?;

    let loaded_form = FeedbackForm::get(&db, Uuid::from_u128(6000)).await?;

    assert_eq!(form, loaded_form);

    Ok(())
}



#[tokio::test]
async fn test_save_response() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;

    let q1 = FeedbackQuestion {
        uuid: Uuid::from_u128(5001),
        short_name: "test1".into(),
        full_name: "Test Question 1".into(),
        description: "abc".into(),
        question_config: QuestionType::RangeQuestion{config: RangeQuestionConfig {
            min: 0,
            max: 100,
            orientation: open_tab_entities::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
            ..Default::default()
        }},
        tournament_id: Some(Uuid::from_u128(1)),
        is_confidential: false,
    };
    let q2 = FeedbackQuestion {
        uuid: Uuid::from_u128(5000),
        short_name: "test2".into(),
        full_name: "Test Question 2".into(),
        description: "abc".into(),
        question_config: QuestionType::RangeQuestion{config: RangeQuestionConfig {
            min: 0,
            max: 100,
            orientation: open_tab_entities::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
            ..Default::default()
        }},
        tournament_id: Some(Uuid::from_u128(1)),
        is_confidential: false,
    };

    q1.save(&db, true).await?;
    q2.save(&db, true).await?;

    let form = FeedbackForm {
        uuid: Uuid::from_u128(6000),
        name: "Test Form".into(),
        
        visibility: FeedbackFormVisibility {
            show_non_aligned_for_chairs: true,
            show_teams_for_chairs: true,
            ..Default::default()
        },
        tournament_id: Some(Uuid::from_u128(1)),
        questions: vec![
            q1.uuid,
            q2.uuid,
        ],
    };

    form.save(&db, true).await?;

    let response = FeedbackResponse {
        uuid: Uuid::from_u128(7000),

        author_participant_id: Uuid::from_u128(2000),
        target_participant_id: Uuid::from_u128(3000),
        source_team_id: None,
        source_participant_id: Some(Uuid::from_u128(2000)),
        source_debate_id: Uuid::from_u128(200),

        values: vec![
            (
                q1.uuid,
                FeedbackResponseValue::Int { val: 0 }
            ),
            (
                q2.uuid,
                FeedbackResponseValue::Int { val: 100 }
            ),
        ].into_iter().collect(),
    };

    response.save(&db, true).await?;

    let loaded_response = FeedbackResponse::get(&db, Uuid::from_u128(7000)).await?;
    assert_eq!(response, loaded_response);

    Ok(())
}


#[tokio::test]
async fn test_update_response() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;

    let q1 = FeedbackQuestion {
        uuid: Uuid::from_u128(5001),
        short_name: "test1".into(),
        full_name: "Test Question 1".into(),
        description: "abc".into(),
        question_config: QuestionType::RangeQuestion{config: RangeQuestionConfig {
            min: 0,
            max: 100,
            orientation: open_tab_entities::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
            ..Default::default()
        }},
        tournament_id: Some(Uuid::from_u128(1)),
        is_confidential: false,
    };
    let q2 = FeedbackQuestion {
        uuid: Uuid::from_u128(5000),
        short_name: "test2".into(),
        full_name: "Test Question 1".into(),
        description: "abc".into(),
        question_config: QuestionType::RangeQuestion{config: RangeQuestionConfig {
            min: 0,
            max: 100,
            orientation: open_tab_entities::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
            ..Default::default()
        }},
        tournament_id: Some(Uuid::from_u128(1)),
        is_confidential: false,
    };

    q1.save(&db, true).await?;
    q2.save(&db, true).await?;

    let form = FeedbackForm {
        uuid: Uuid::from_u128(6000),
        name: "Test Form".into(),
        
        visibility: FeedbackFormVisibility {
            show_non_aligned_for_chairs: true,
            show_teams_for_chairs: true,
            ..Default::default()
        },
        tournament_id: Some(Uuid::from_u128(1)),
        questions: vec![
            q1.uuid,
            q2.uuid,
        ],
    };

    form.save(&db, true).await?;

    let _response = FeedbackResponse {
        uuid: Uuid::from_u128(7000),

        author_participant_id: Uuid::from_u128(2000),
        target_participant_id: Uuid::from_u128(3000),
        source_team_id: None,
        source_participant_id: Some(Uuid::from_u128(2000)),
        source_debate_id: Uuid::from_u128(200),

        values: vec![
            (
                q1.uuid,
                FeedbackResponseValue::Int { val: 0 }
            ),
            (
                q2.uuid,
                FeedbackResponseValue::Int { val: 100 }
            ),
        ].into_iter().collect(),
    };

    let mut response = FeedbackResponse {
        uuid: Uuid::from_u128(7000),

        author_participant_id: Uuid::from_u128(2000),
        target_participant_id: Uuid::from_u128(3000),
        source_team_id: None,
        source_participant_id: Some(Uuid::from_u128(2000)),
        source_debate_id: Uuid::from_u128(200),

        values: vec![
            (
                q1.uuid,
                FeedbackResponseValue::Int { val: 0 }
            ),
        ].into_iter().collect(),
    };

    response.save(&db, true).await?;

    response.values.insert(q2.uuid, FeedbackResponseValue::Int { val: 100 });
    response.save(&db, false).await?;
    let loaded_response = FeedbackResponse::get(&db, Uuid::from_u128(7000)).await?;
    assert_eq!(response, loaded_response);

    Ok(())
}