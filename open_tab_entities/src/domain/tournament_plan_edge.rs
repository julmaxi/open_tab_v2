use std::collections::HashMap;

use async_trait::async_trait;
use itertools::Itertools;
use open_tab_macros::SimpleEntity;
use sea_orm::{prelude::*, QuerySelect};
use serde::{Serialize, Deserialize};

use crate::schema;


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, SimpleEntity)]
#[module_path = "crate::schema::tournament_plan_edge"]
#[get_many_tournaments_func = "get_many_tournaments_impl"]
pub struct TournamentPlanEdge {
    pub uuid: Uuid,
    pub source_id: Uuid,
    pub target_id: Uuid,
}


impl TournamentPlanEdge {
    pub fn new(source_id: Uuid, target_id: Uuid) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            source_id,
            target_id
        }
    }

    pub async fn get_all_for_sources<C>(db: &C, node_uuids: Vec<Uuid>) -> Result<Vec<Self>, DbErr> where C: sea_orm::ConnectionTrait {
        let edges = schema::tournament_plan_edge::Entity::find()
            .filter(schema::tournament_plan_edge::Column::SourceId.is_in(node_uuids))
            .all(db)
            .await?;

        Ok(edges.into_iter().map(Self::from_model).collect_vec())
    }

    async fn get_many_tournaments_impl<C>(db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let node_tournaments : HashMap<Uuid, Uuid> = schema::tournament_plan_node::Entity::find()
            .select_only()
            .column(schema::tournament_plan_edge::Column::Uuid)
            .column(schema::tournament_plan_node::Column::TournamentId)
            .join(
                sea_query::JoinType::InnerJoin,
                schema::tournament_plan_node::Entity::belongs_to(schema::tournament_plan_edge::Entity)
                .from(schema::tournament_plan_node::Column::Uuid)
                .to(schema::tournament_plan_edge::Column::SourceId)
                .into()
            ).into_tuple().all(db).await?.into_iter().collect();

        entities.iter().map(|b| {
            Ok(node_tournaments.get(&b.uuid).cloned())
        }).collect()
    }
}