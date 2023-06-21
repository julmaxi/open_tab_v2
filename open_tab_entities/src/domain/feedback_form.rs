use std::{error::Error, collections::HashMap, ops::BitOr, str::FromStr};

use async_trait::async_trait;
use itertools::{Itertools, izip};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::{utilities::{load_many, BatchLoad}, schema};

use sea_orm::{prelude::*, LoaderTrait, ConnectionTrait, ColumnTrait, EntityTrait, ActiveValue, QueryOrder};

use super::{entity::LoadEntity, TournamentEntity};



#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackForm {
    pub uuid: Uuid,
    pub name: String,

    pub visibility: FeedbackFormVisibility,

    pub tournament_id: Option<Uuid>,

    pub questions: Vec<Uuid>,
}


#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FeedbackFormVisibility {
    pub show_chairs_for_wings: bool,
    pub show_chairs_for_presidents: bool,

    pub show_wings_for_chairs: bool,
    pub show_wings_for_presidents: bool,
    pub show_wings_for_wings: bool,
   
    pub show_presidents_for_chairs: bool,
    pub show_presidents_for_wings: bool,

    pub show_teams_for_chairs: bool,
    pub show_teams_for_presidents: bool,
    pub show_teams_for_wings: bool,

    pub show_non_aligned_for_chairs: bool,
    pub show_non_aligned_for_presidents: bool,
    pub show_non_aligned_for_wings: bool
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag="type")]
pub enum FeedbackSourceRole {
    Chair,
    Wing,
    President,
    Team,
    NonAligned
}

#[derive(Debug, thiserror::Error)]
pub enum RoleParseError {
    #[error("Invalid feedback source role: {0}")]
    InvalidRole(String)
}

impl FromStr for FeedbackSourceRole {
    type Err = RoleParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "chair" => Ok(FeedbackSourceRole::Chair),
            "wing" => Ok(FeedbackSourceRole::Wing),
            "president" => Ok(FeedbackSourceRole::President),
            "team" => Ok(FeedbackSourceRole::Team),
            "non_aligned" => Ok(FeedbackSourceRole::NonAligned),
            _ => Err(RoleParseError::InvalidRole(s.into()))
        }
    }
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag="type")]
pub enum FeedbackTargetRole {
    Chair,
    Wing,
    President
}


impl FromStr for FeedbackTargetRole {
    type Err = RoleParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "chair" => Ok(FeedbackTargetRole::Chair),
            "wing" => Ok(FeedbackTargetRole::Wing),
            "president" => Ok(FeedbackTargetRole::President),
            _ => Err(RoleParseError::InvalidRole(s.into()))
        }
    }
}

impl BitOr<&FeedbackFormVisibility> for FeedbackFormVisibility {
    type Output = FeedbackFormVisibility;

    fn bitor(self, rhs: &FeedbackFormVisibility) -> Self::Output {
        FeedbackFormVisibility {
            show_chairs_for_wings: self.show_chairs_for_wings || rhs.show_chairs_for_wings,
            show_chairs_for_presidents: self.show_chairs_for_presidents || rhs.show_chairs_for_presidents,
            show_wings_for_chairs: self.show_wings_for_chairs || rhs.show_wings_for_chairs,
            show_wings_for_presidents: self.show_wings_for_presidents || rhs.show_wings_for_presidents,
            show_wings_for_wings: self.show_wings_for_wings || rhs.show_wings_for_wings,
            show_presidents_for_chairs: self.show_presidents_for_chairs || rhs.show_presidents_for_chairs,
            show_presidents_for_wings: self.show_presidents_for_wings || rhs.show_presidents_for_wings,
            show_teams_for_chairs: self.show_teams_for_chairs || rhs.show_teams_for_chairs,
            show_teams_for_presidents: self.show_teams_for_presidents || rhs.show_teams_for_presidents,
            show_teams_for_wings: self.show_teams_for_wings || rhs.show_teams_for_wings,
            show_non_aligned_for_chairs: self.show_non_aligned_for_chairs || rhs.show_non_aligned_for_chairs,
            show_non_aligned_for_presidents: self.show_non_aligned_for_presidents || rhs.show_non_aligned_for_presidents,
            show_non_aligned_for_wings: self.show_non_aligned_for_wings || rhs.show_non_aligned_for_wings,
        }
    }
}

impl FeedbackFormVisibility {
    pub fn all() -> Self {
        FeedbackFormVisibility {
            show_chairs_for_wings: true,
            show_chairs_for_presidents: true,
            show_wings_for_chairs: true,
            show_wings_for_presidents: true,
            show_wings_for_wings: true,
            show_presidents_for_chairs: true,
            show_presidents_for_wings: true,
            show_teams_for_chairs: true,
            show_teams_for_presidents: true,
            show_teams_for_wings: true,
            show_non_aligned_for_chairs: true,
            show_non_aligned_for_presidents: true,
            show_non_aligned_for_wings: true,
        }
    }
    pub fn to_feedback_direction_pairs(&self) -> Vec<(FeedbackSourceRole, FeedbackTargetRole)> {
        let mut pairs = Vec::new();

        if self.show_chairs_for_wings {
            pairs.push((FeedbackSourceRole::Chair, FeedbackTargetRole::Wing));
        }

        if self.show_chairs_for_presidents {
            pairs.push((FeedbackSourceRole::Chair, FeedbackTargetRole::President));
        }

        if self.show_wings_for_chairs {
            pairs.push((FeedbackSourceRole::Wing, FeedbackTargetRole::Chair));
        }

        if self.show_wings_for_presidents {
            pairs.push((FeedbackSourceRole::Wing, FeedbackTargetRole::President));
        }

        if self.show_wings_for_wings {
            pairs.push((FeedbackSourceRole::Wing, FeedbackTargetRole::Wing));
        }

        if self.show_presidents_for_chairs {
            pairs.push((FeedbackSourceRole::President, FeedbackTargetRole::Chair));
        }

        if self.show_presidents_for_wings {
            pairs.push((FeedbackSourceRole::President, FeedbackTargetRole::Wing));
        }

        if self.show_teams_for_chairs {
            pairs.push((FeedbackSourceRole::Team, FeedbackTargetRole::Chair));
        }

        if self.show_teams_for_presidents {
            pairs.push((FeedbackSourceRole::Team, FeedbackTargetRole::President));
        }

        if self.show_teams_for_wings {
            pairs.push((FeedbackSourceRole::Team, FeedbackTargetRole::Wing));
        }

        if self.show_non_aligned_for_chairs {
            pairs.push((FeedbackSourceRole::NonAligned, FeedbackTargetRole::Chair));
        }

        if self.show_non_aligned_for_presidents {
            pairs.push((FeedbackSourceRole::NonAligned, FeedbackTargetRole::President));
        }

        if self.show_non_aligned_for_wings {
            pairs.push((FeedbackSourceRole::NonAligned, FeedbackTargetRole::Wing));
        }

        pairs
    }
}

#[async_trait]
impl LoadEntity for FeedbackForm {
    async fn try_get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Option<Self>>, Box<dyn Error>> where C: ConnectionTrait {
        let forms : Vec<Option<schema::feedback_form::Model>> = schema::feedback_form::Entity::batch_load::<_, Uuid>(db, uuids.clone()).await?;
        let mut questions = schema::feedback_form_question::Entity::find()
            .filter(schema::feedback_form_question::Column::FeedbackFormId.is_in(forms.iter().filter_map(|x| x.clone().map(|x : schema::feedback_form::Model| x.uuid.clone())).collect::<Vec<Uuid>>()))
            .all(db).await?.into_iter().into_grouping_map_by(|e| e.feedback_form_id).collect::<Vec<_>>();
        
        forms.into_iter().map(
            |f| {
                match f {
                    Some(f) => {
                        let questions = questions.remove(&f.uuid);
                        FeedbackForm::from_rows(f, questions.unwrap_or_else(Vec::new)).map(|x| Some(x))
                    },
                    None => Ok(None)
                }
            }
        ).collect()
    }
}

#[async_trait]
impl TournamentEntity for FeedbackForm {
    async fn save<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        let existing_form = if guarantee_insert {
            None
        }
        else {
            schema::feedback_form::Entity::find_by_id(self.uuid).one(db).await?
        };

        let mut new_form = schema::feedback_form::ActiveModel {
            uuid: ActiveValue::Set(self.uuid),
            name: ActiveValue::Set(self.name.clone()),
            show_chairs_for_wings: ActiveValue::Set(self.visibility.show_chairs_for_wings),
            show_chairs_for_presidents: ActiveValue::Set(self.visibility.show_chairs_for_presidents),
            show_wings_for_chairs: ActiveValue::Set(self.visibility.show_wings_for_chairs),
            show_wings_for_presidents: ActiveValue::Set(self.visibility.show_wings_for_presidents),
            show_wings_for_wings: ActiveValue::Set(self.visibility.show_wings_for_wings),
            show_presidents_for_chairs: ActiveValue::Set(self.visibility.show_presidents_for_chairs),
            show_presidents_for_wings: ActiveValue::Set(self.visibility.show_presidents_for_wings),
            show_teams_for_chairs: ActiveValue::Set(self.visibility.show_teams_for_chairs),
            show_teams_for_presidents: ActiveValue::Set(self.visibility.show_teams_for_presidents),
            show_teams_for_wings: ActiveValue::Set(self.visibility.show_teams_for_wings),
            show_non_aligned_for_chairs: ActiveValue::Set(self.visibility.show_non_aligned_for_chairs),
            show_non_aligned_for_presidents: ActiveValue::Set(self.visibility.show_non_aligned_for_presidents),
            show_non_aligned_for_wings: ActiveValue::Set(self.visibility.show_non_aligned_for_wings),

            tournament_id: ActiveValue::Set(self.tournament_id),
        };

        let existing_questions = match &existing_form {
            Some(existing_form) => {
                schema::feedback_form_question::Entity::find()
                    .filter(schema::feedback_form_question::Column::FeedbackFormId.eq(existing_form.uuid))
                    .order_by_asc(schema::feedback_form_question::Column::Index)
                    .all(db).await?
            },
            None => Vec::new(),
        };

        if let Some(_) = existing_form {
            new_form.uuid = ActiveValue::Unchanged(self.uuid);
            new_form.update(db).await?;
        }
        else {
            new_form.insert(db).await?;
        }

        let existing_question_id_positions = existing_questions.iter().enumerate().map(|(index, q)| (q.feedback_question_id, index as usize)).collect::<HashMap<Uuid, usize>>();
        
        for (new_index, question) in self.questions.iter().enumerate() {
            match existing_question_id_positions.get(&question) {
                Some(index) => {
                    if *index != new_index {
                        schema::feedback_form_question::ActiveModel {
                            feedback_form_id: ActiveValue::Unchanged(self.uuid),
                            feedback_question_id: ActiveValue::Unchanged(*question),
                            index: ActiveValue::Set(new_index as i32),
                        }.update(db).await?;
                    }
                },
                None => {
                    schema::feedback_form_question::ActiveModel {
                        feedback_form_id: ActiveValue::Set(self.uuid),
                        feedback_question_id: ActiveValue::Set(*question),
                        index: ActiveValue::Set(new_index as i32),
                    }.insert(db).await?;
                }
            }
        }
            
        Ok(())
    }

    async fn get_tournament<C>(&self, _db: &C) -> Result<Option<Uuid>, Box<dyn Error>> where C: ConnectionTrait {
        Ok(self.tournament_id)
    }
}

impl FeedbackForm {
    pub async fn get_all_in_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Vec<Self>, Box<dyn Error>> where C: ConnectionTrait {
        let forms = schema::feedback_form::Entity::find()
            .filter(schema::feedback_form::Column::TournamentId.eq(tournament_id))
            .all(db).await?;

        let mut questions = schema::feedback_form_question::Entity::find()
            .filter(schema::feedback_form_question::Column::FeedbackFormId.is_in(forms.iter().map(|x| x.uuid).collect::<Vec<Uuid>>()))
            .all(db).await?.into_iter().into_grouping_map_by(|e| e.feedback_form_id).collect::<Vec<_>>();
        
        forms.into_iter().map(
            |f| {
                let questions = questions.remove(&f.uuid);
                FeedbackForm::from_rows(f, questions.unwrap_or_else(Vec::new))
            }
        ).collect()
    }
    fn from_rows(form: schema::feedback_form::Model, questions: Vec<schema::feedback_form_question::Model>) -> Result<Self, Box<dyn Error>> {
        let questions = questions.into_iter().sorted_by_key(|x| x.index).map(|q| {
            q.feedback_question_id
        }).collect();

        Ok(
            FeedbackForm {
                uuid: form.uuid,
                name: form.name,
                visibility: FeedbackFormVisibility {
                    show_chairs_for_wings: form.show_chairs_for_wings,
                    show_chairs_for_presidents: form.show_chairs_for_presidents,
                    show_wings_for_chairs: form.show_wings_for_chairs,
                    show_wings_for_presidents: form.show_wings_for_presidents,
                    show_wings_for_wings: form.show_wings_for_wings,
                    show_presidents_for_chairs: form.show_presidents_for_chairs,
                    show_presidents_for_wings: form.show_presidents_for_wings,
                    show_teams_for_chairs: form.show_teams_for_chairs,
                    show_teams_for_presidents: form.show_teams_for_presidents,
                    show_teams_for_wings: form.show_teams_for_wings,
                    show_non_aligned_for_chairs: form.show_non_aligned_for_chairs,
                    show_non_aligned_for_presidents: form.show_non_aligned_for_presidents,
                    show_non_aligned_for_wings: form.show_non_aligned_for_wings
                },
                questions,
                tournament_id: form.tournament_id
            }
        )
    }

    pub fn is_valid_for_direction(&self, source_role: FeedbackSourceRole, target_role: FeedbackTargetRole) -> bool {
        match source_role {
            FeedbackSourceRole::Chair => {
                match target_role {
                    FeedbackTargetRole::Chair => false,
                    FeedbackTargetRole::Wing => self.visibility.show_chairs_for_wings,
                    FeedbackTargetRole::President => self.visibility.show_chairs_for_presidents,
                }
            },
            FeedbackSourceRole::Wing => {
                match target_role {
                    FeedbackTargetRole::Chair => self.visibility.show_wings_for_chairs,
                    FeedbackTargetRole::Wing => self.visibility.show_wings_for_wings,
                    FeedbackTargetRole::President => self.visibility.show_wings_for_presidents,
                }
            },
            FeedbackSourceRole::President => {
                match target_role {
                    FeedbackTargetRole::Chair => self.visibility.show_presidents_for_chairs,
                    FeedbackTargetRole::Wing => self.visibility.show_presidents_for_wings,
                    FeedbackTargetRole::President => false,
                }
            },
            FeedbackSourceRole::Team => {
                match target_role {
                    FeedbackTargetRole::Chair => self.visibility.show_teams_for_chairs,
                    FeedbackTargetRole::Wing => self.visibility.show_teams_for_wings,
                    FeedbackTargetRole::President => self.visibility.show_teams_for_presidents,
                }
            },
            FeedbackSourceRole::NonAligned => {
                match target_role {
                    FeedbackTargetRole::Chair => self.visibility.show_non_aligned_for_chairs,
                    FeedbackTargetRole::Wing => self.visibility.show_non_aligned_for_wings,
                    FeedbackTargetRole::President => self.visibility.show_non_aligned_for_presidents,
                }
            },
        }
    }
}
