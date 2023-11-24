use std::collections::HashMap;

use itertools::Itertools;
use open_tab_entities::domain::{self, feedback_form::FeedbackFormVisibility};
use sea_orm::prelude::Uuid;
use serde::{Serialize, Deserialize};
/*
"
shared_questions:
    skill:
        short_name: skill
        full_name: Wie würdest du insgesamt die Kompetenz dieser JurorIn bewerten?,
        type: range
        min: 0
        max: 100 
        orientation: high
        labels: 
            0: Sehr schlecht
            100: Sehr gut

forms:
    chairs_for_wings:
        questions:
            - comments
"
*/


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct QuestionInfo {
    short_name: String,
    full_name: String,
    #[serde(flatten)]
    config: QuestionType,
    #[serde(default)]
    description: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
enum QuestionType {
    #[serde(rename = "range")]
    RangeQuestion {
        #[serde(flatten)]
        config: RangeQuestionConfig
    },
    #[serde(rename = "text")]
    TextQuestion,
    #[serde(rename = "yes_no")]
    YesNoQuestion,
}

impl Into<domain::feedback_question::QuestionType> for QuestionType {
    fn into(self) -> domain::feedback_question::QuestionType {
        match self {
            QuestionType::RangeQuestion { config } => domain::feedback_question::QuestionType::RangeQuestion {
                config: config.into()
            },
            QuestionType::TextQuestion => domain::feedback_question::QuestionType::TextQuestion,
            QuestionType::YesNoQuestion => domain::feedback_question::QuestionType::YesNoQuestion,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct RangeQuestionConfig {
    min: i32,
    max: i32,
    orientation: RangeQuestionOrientation,
    labels: HashMap<i32, String>,
}

impl Into<domain::feedback_question::RangeQuestionConfig> for RangeQuestionConfig {
    fn into(self) -> domain::feedback_question::RangeQuestionConfig {
        domain::feedback_question::RangeQuestionConfig {
            min: self.min,
            max: self.max,
            orientation: match self.orientation {
                RangeQuestionOrientation::HighIsGood => domain::feedback_question::RangeQuestionOrientation::HighIsGood,
                RangeQuestionOrientation::LowIsGood => domain::feedback_question::RangeQuestionOrientation::LowIsGood,
                RangeQuestionOrientation::MeanIsGood => domain::feedback_question::RangeQuestionOrientation::MeanIsGood,
            },
            labels: self.labels.into_iter().collect_vec()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
enum RangeQuestionOrientation {
    #[serde(rename = "high")]
    HighIsGood,
    #[serde(rename = "low")]
    LowIsGood,
    #[serde(rename = "mean")]
    MeanIsGood
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FeedbackForm {
    questions: Vec<QuestionKeyOrInline>
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
enum QuestionKeyOrInline {
    Key(String),
    Inline(QuestionInfo)
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormTemplate {
    #[serde(default)]
    shared_questions: HashMap<String, QuestionInfo>,
    chairs_for_wings: Option<FormKeyOrInline>,

    wings_for_chairs: Option<FormKeyOrInline>,
    wings_for_presidents: Option<FormKeyOrInline>,
    wings_for_wings: Option<FormKeyOrInline>,

    presidents_for_chairs: Option<FormKeyOrInline>,
    presidents_for_wings: Option<FormKeyOrInline>,

    teams_for_chairs: Option<FormKeyOrInline>,
    teams_for_presidents: Option<FormKeyOrInline>,
    teams_for_wings: Option<FormKeyOrInline>,

    non_aligned_for_chairs: Option<FormKeyOrInline>,
    non_aligned_for_presidents: Option<FormKeyOrInline>,
    non_aligned_for_wings: Option<FormKeyOrInline>,
}

impl FormTemplate {
    pub fn into_forms_and_questions_for_tournament(self, tournament_uuid: Uuid) -> Result<(Vec<domain::feedback_form::FeedbackForm>, Vec<domain::feedback_question::FeedbackQuestion>), anyhow::Error> {
        let mut questions : Vec<domain::feedback_question::FeedbackQuestion> = vec![];

        let mut forms : HashMap<String, domain::feedback_form::FeedbackForm> = HashMap::new();
        let mut question_names_to_ids : HashMap<String, Uuid> = HashMap::new();

        for (key, question) in self.shared_questions.iter() {
            let uuid = Uuid::new_v4();
            questions.push(
                domain::feedback_question::FeedbackQuestion {
                    uuid,
                    short_name: question.short_name.clone(),
                    full_name: question.full_name.clone(),
                    description: question.description.clone().unwrap_or("".into()),
                    question_config: question.config.clone().into(),
                    tournament_id: Some(tournament_uuid),
                }
            );
            question_names_to_ids.insert(key.clone(), uuid);
        }

        let all_forms = self.get_all_forms();
        for (key, form) in all_forms.iter() {
            match form {
                FormKeyOrInline::Inline(form) => {
                    let uuid = Uuid::new_v4();
                    forms.insert(
                        key.clone(),
                        domain::feedback_form::FeedbackForm {
                            uuid,
                            name: key.clone(),
                            questions: form.questions.iter().map(|question| {
                                match question {
                                    QuestionKeyOrInline::Key(key) => {
                                        question_names_to_ids.get(key).unwrap().clone()
                                    },
                                    QuestionKeyOrInline::Inline(question) => {
                                        let uuid = Uuid::new_v4();
                                        questions.push(
                                            domain::feedback_question::FeedbackQuestion {
                                                uuid,
                                                short_name: question.short_name.clone(),
                                                full_name: question.full_name.clone(),
                                                description: question.description.clone().unwrap_or("".into()),
                                                question_config: question.config.clone().into(),
                                                tournament_id: Some(tournament_uuid),
                                            }
                                        );
                                        uuid
                                    }
                                }
                            }).collect(),
                            tournament_id: Some(tournament_uuid),
                            visibility: Self::visibility_from_key(&key),
                        }
                    );
                },
                _ => {}
            };
        }

        for (form_key, form) in all_forms.iter() {
            match form {
                FormKeyOrInline::Key(key) => {
                    let existing_form = forms.get_mut(key).ok_or(anyhow::anyhow!("Form {} was not defined", key)).unwrap();
                    existing_form.visibility.union(
                        &Self::visibility_from_key(form_key)
                    );                    
                }
                _ => {}
            }
        }

        Ok((
            forms.into_values().collect(),
            questions
        ))
    }

    fn get_all_forms(&self) -> Vec<(String, FormKeyOrInline)> {
        let mut out = vec![];

        if let Some(form) = &self.chairs_for_wings {
            out.push(("chairs_for_wings".into(), form.clone()));
        }

        if let Some(form) = &self.wings_for_chairs {
            out.push(("wings_for_chairs".into(), form.clone()));
        }

        if let Some(form) = &self.wings_for_presidents {
            out.push(("wings_for_presidents".into(), form.clone()));
        }

        if let Some(form) = &self.wings_for_wings {
            out.push(("wings_for_wings".into(), form.clone()));
        }

        if let Some(form) = &self.presidents_for_chairs {
            out.push(("presidents_for_chairs".into(), form.clone()));
        }

        if let Some(form) = &self.presidents_for_wings {
            out.push(("presidents_for_wings".into(), form.clone()));
        }

        if let Some(form) = &self.teams_for_chairs {
            out.push(("teams_for_chairs".into(), form.clone()));
        }

        if let Some(form) = &self.teams_for_presidents {
            out.push(("teams_for_presidents".into(), form.clone()));
        }

        if let Some(form) = &self.teams_for_wings {
            out.push(("teams_for_wings".into(), form.clone()));
        }

        if let Some(form) = &self.non_aligned_for_chairs {
            out.push(("non_aligned_for_chairs".into(), form.clone()));
        }

        if let Some(form) = &self.non_aligned_for_presidents {
            out.push(("non_aligned_for_presidents".into(), form.clone()));
        }

        if let Some(form) = &self.non_aligned_for_wings {
            out.push(("non_aligned_for_wings".into(), form.clone()));
        }

        out
    }

    fn visibility_from_key(name: &String) -> FeedbackFormVisibility {
        let mut visbility = FeedbackFormVisibility::default();

        match name.as_str() {
            "chairs_for_wings" => {
                visbility.show_chairs_for_wings = true;
            },
            "wings_for_chairs" => {
                visbility.show_wings_for_chairs = true;
            },
            "wings_for_presidents" => {
                visbility.show_wings_for_presidents = true;
            },
            "wings_for_wings" => {
                visbility.show_wings_for_wings = true;
            },
            "presidents_for_chairs" => {
                visbility.show_presidents_for_chairs = true;
            },
            "presidents_for_wings" => {
                visbility.show_presidents_for_wings = true;
            },
            "teams_for_chairs" => {
                visbility.show_teams_for_chairs = true;
            },
            "teams_for_presidents" => {
                visbility.show_teams_for_presidents = true;
            },
            "teams_for_wings" => {
                visbility.show_teams_for_wings = true;
            },
            "non_aligned_for_chairs" => {
                visbility.show_non_aligned_for_chairs = true;
            },
            "non_aligned_for_presidents" => {
                visbility.show_non_aligned_for_presidents = true;
            },
            "non_aligned_for_wings" => {
                visbility.show_non_aligned_for_wings = true;
            },
            _ => {}
        };

        visbility
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
enum FormKeyOrInline {
    Key(String),
    Inline(FeedbackForm)
}

#[cfg(test)]
mod tests {
    use open_tab_entities::domain;
    use sea_orm::prelude::Uuid;

    #[test]
    fn test_parse_range_question() {
        let test_string = "
short_name: skill
full_name: Wie würdest du insgesamt die Kompetenz dieser JurorIn bewerten?
type: range
min: 0
max: 100 
orientation: high
labels: 
    0: Sehr schlecht
    100: Sehr gut";
        let result = serde_yaml::from_str::<super::QuestionInfo>(test_string).unwrap();
        
        assert_eq!(result.short_name, "skill");
        assert_eq!(result.full_name, "Wie würdest du insgesamt die Kompetenz dieser JurorIn bewerten?");
        assert_eq!(result.config, super::QuestionType::RangeQuestion {
            config: super::RangeQuestionConfig {
                min: 0,
                max: 100,
                orientation: super::RangeQuestionOrientation::HighIsGood,
                labels: vec![(0, "Sehr schlecht".into()), (100, "Sehr gut".into())].into_iter().collect()
            }
        });
    }

    #[test]
    fn test_parse_text_question() {
        let test_string = "
short_name: comments
full_name: Kommentare
type: text";
        let result = serde_yaml::from_str::<super::QuestionInfo>(test_string).unwrap();
        
        assert_eq!(result.short_name, "comments");
        assert_eq!(result.full_name, "Kommentare");
        assert_eq!(result.config, super::QuestionType::TextQuestion);
    }

    #[test]
    fn test_parse_yes_no_question() {
        let test_string = "
short_name: chair
full_name: War diese Person Chair?
type: yes_no";
        let result = serde_yaml::from_str::<super::QuestionInfo>(test_string).unwrap();
        
        assert_eq!(result.short_name, "chair");
        assert_eq!(result.full_name, "War diese Person Chair?");
        assert_eq!(result.config, super::QuestionType::YesNoQuestion);
    }

    #[test]
    fn test_parse_form_with_key() {
        let test_string = "
questions:
    - skill
    - comments
    - chair";
        let result = serde_yaml::from_str::<super::FeedbackForm>(test_string).unwrap();
        
        assert_eq!(result.questions, vec![
            super::QuestionKeyOrInline::Key("skill".into()),
            super::QuestionKeyOrInline::Key("comments".into()),
            super::QuestionKeyOrInline::Key("chair".into()),
        ]);
    }

    #[test]
    fn test_parse_form_with_inline() {
        let test_string = "
questions:
    - short_name: skill
      full_name: Wie würdest du insgesamt die Kompetenz dieser JurorIn bewerten?
      type: range
      min: 0
      max: 100 
      orientation: high
      labels: 
          0: Sehr schlecht
          100: Sehr gut
    - short_name: comments
      full_name: Kommentare
      type: text
    - short_name: chair
      full_name: War diese Person Chair?
      type: yes_no";
        let result = serde_yaml::from_str::<super::FeedbackForm>(test_string).unwrap();
        
        assert_eq!(result.questions, vec![
            super::QuestionKeyOrInline::Inline(super::QuestionInfo {
                short_name: "skill".into(),
                full_name: "Wie würdest du insgesamt die Kompetenz dieser JurorIn bewerten?".into(),
                config: super::QuestionType::RangeQuestion {
                    config: super::RangeQuestionConfig {
                        min: 0,
                        max: 100,
                        orientation: super::RangeQuestionOrientation::HighIsGood,
                        labels: vec![(0, "Sehr schlecht".into()), (100, "Sehr gut".into())].into_iter().collect()
                    }
                },
                description: None
            }),
            super::QuestionKeyOrInline::Inline(super::QuestionInfo {
                short_name: "comments".into(),
                full_name: "Kommentare".into(),
                config: super::QuestionType::TextQuestion,
                description: None
            }),
            super::QuestionKeyOrInline::Inline(super::QuestionInfo {
                short_name: "chair".into(),
                full_name: "War diese Person Chair?".into(),
                config: super::QuestionType::YesNoQuestion,
                description: None
            }),
        ]);
    }

    #[test]
    fn test_parse_template() {
        let test_string = "
shared_questions:
    skill:
        short_name: skill
        full_name: Wie würdest du insgesamt die Kompetenz dieser JurorIn bewerten?
        type: range
        min: 0
        max: 100 
        orientation: high
        labels: 
            0: Sehr schlecht
            100: Sehr gut
chairs_for_wings:
    questions:
        - skill
        ";

        let result = serde_yaml::from_str::<super::FormTemplate>(test_string).unwrap();

        let (forms, questions) = result.into_forms_and_questions_for_tournament(Uuid::new_v4()).unwrap();

        assert_eq!(questions.len(), 1);
        assert_eq!(forms.len(), 1);

        assert_eq!(forms[0].questions, vec![questions[0].uuid]);
    }

    #[test]
    fn test_parse_template_with_inline_question() {
        let test_string = "
chairs_for_wings:
    questions:
        - short_name: skill
          full_name: Wie würdest du insgesamt die Kompetenz dieser JurorIn bewerten?
          type: range
          min: 0
          max: 100 
          orientation: high
          labels: 
              0: Sehr schlecht
              100: Sehr gut
        ";

        let result = serde_yaml::from_str::<super::FormTemplate>(test_string).unwrap();

        let (forms, questions) = result.into_forms_and_questions_for_tournament(Uuid::new_v4()).unwrap();

        assert_eq!(questions.len(), 1);
        assert_eq!(forms.len(), 1);

        assert_eq!(forms[0].questions, vec![questions[0].uuid]);
    }

    #[test]
    fn test_parse_template_with_multi_visiblity() {
        let test_string = "
shared_questions:
    skill:
        short_name: skill
        full_name: Wie würdest du insgesamt die Kompetenz dieser JurorIn bewerten?
        type: range
        min: 0
        max: 100 
        orientation: high
        labels:
            0: Sehr schlecht
            100: Sehr gut
chairs_for_wings:
    questions:
        - skill
wings_for_chairs: chairs_for_wings
        ";

        let result = serde_yaml::from_str::<super::FormTemplate>(test_string).unwrap();
        let (forms, _questions) = result.into_forms_and_questions_for_tournament(Uuid::new_v4()).unwrap();
        assert_eq!(forms.len(), 1);
        assert_eq!(forms[0].visibility, domain::feedback_form::FeedbackFormVisibility {
            show_chairs_for_wings: true,
            show_wings_for_chairs: true,
            ..Default::default()
        });
    }
}