use sea_orm::prelude::Uuid;
use sea_orm::ActiveValue::Unchanged;

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::{prelude::*, sea_query, QueryOrder, QuerySelect};
use open_tab_entities::{prelude::*, EntityTypeId};

use open_tab_entities::schema::tournament_institution;




use crate::LoadedView;

pub struct LoadedInstitutionsView {
    pub view: InstitutionsView,
    pub tournament_id: Uuid,
    pub include_statistics: bool
}

impl LoadedInstitutionsView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid, include_statistics: bool) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Ok(
            Self {
                tournament_id: tournament_uuid,
                view: InstitutionsView::load_from_tournament(db, tournament_uuid, include_statistics).await?,
                include_statistics: include_statistics
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedInstitutionsView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.has_changes_for_type(EntityTypeId::TournamentInstitution)
            || (self.include_statistics && changes.has_changes_for_type(EntityTypeId::Participant)) {
            self.view = InstitutionsView::load_from_tournament(db, self.tournament_id, self.include_statistics).await?;

            let mut out = HashMap::new();
            out.insert(".".to_string(), serde_json::to_value(&self.view)?);

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
pub struct InstitutionsView {
    institutions: Vec<InstitutionOverview>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstitutionOverview {
    uuid: Uuid,
    name: String,
    official_identifier: Option<String>,
    statistics: Option<InstitutionStatistics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstitutionStatistics {
    num_speakers: u64,
    num_adjudicators: u64,
}


impl InstitutionsView {
    async fn load_from_tournament<C>(db: &C, tournament_uuid: Uuid, include_statistics: bool) -> Result<InstitutionsView, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let institutions = tournament_institution::Entity::find().filter(
            tournament_institution::Column::TournamentId.eq(tournament_uuid)
        ).order_by_asc(tournament_institution::Column::Name).all(db).await?;

        use open_tab_entities::schema::{participant_tournament_institution, participant, adjudicator, speaker};

        let statistics = if include_statistics {
            let query_stats = participant::Entity::find()
                .select_only()
                .column_as(
                    participant_tournament_institution::Column::InstitutionId,
                    "institution_id"
                )
                .column_as(
                    sea_query::Expr::count(speaker::Column::Uuid.into_expr()),
                    "num_speakers"
                )
                .column_as(
                    sea_query::Expr::count(adjudicator::Column::Uuid.into_expr()),
                    "num_adjudicators"
                )
                .left_join(speaker::Entity)
                .left_join(adjudicator::Entity)
                .inner_join(participant_tournament_institution::Entity)
                .group_by(participant_tournament_institution::Column::InstitutionId)
                .filter(participant::Column::TournamentId.eq(tournament_uuid))
                .into_tuple::<(Uuid, i32, i32)>()
                .all(db)
                .await?;

            let mut stats = HashMap::new();

            for (inst_id, speaker_cnt, adjudicator_cnt) in query_stats {
                // Assuming participant type is determined elsewhere, populate counts accordingly
                // For simplicity, this assumes all participants are speakers
                stats.insert(inst_id, (speaker_cnt as u64, adjudicator_cnt as u64));
            }

            Some(stats)
        } else {
            None
        };

        let institution_overviews = institutions.into_iter().map(|institution| {
            InstitutionOverview {
                uuid: institution.uuid,
                name: institution.name,
                official_identifier: institution.official_identifier,
                statistics: statistics.as_ref().map(
                    |s| {
                        InstitutionStatistics {
                            num_speakers: s.get(&institution.uuid).map(|(speakers, _)| *speakers).unwrap_or(0),
                            num_adjudicators: s.get(&institution.uuid).map(|(_, adjudicators)| *adjudicators).unwrap_or(0),
                        }
                    }
                )
            }
        }).collect();

        Ok(
            InstitutionsView {
                institutions: institution_overviews
            }
        )
    }
}