

use async_trait::async_trait;
use itertools::Itertools;
use open_tab_macros::SimpleEntity;
use sea_query::{SimpleExpr, SeaRc, ColumnRef};
use sea_query::Alias;
use sea_orm::{prelude::*, QuerySelect};
use serde::{Serialize, Deserialize};

use crate::schema;
use sea_orm::JoinType;


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, SimpleEntity)]
#[module_path = "crate::schema::participant_clash"]
#[get_many_tournaments_func = "get_many_tournaments_impl"]
pub struct ParticipantClash {
    pub uuid: Uuid,
    pub declaring_participant_id: Uuid,
    pub target_participant_id: Uuid,
    pub clash_severity: u16
}

impl ParticipantClash {
    async fn get_many_tournaments_impl<C>(db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, anyhow::Error> where C: ConnectionTrait {
        let participants = schema::participant::Entity::find()
            .filter(schema::participant::Column::Uuid.is_in(entities.iter().map(|entity| entity.uuid).collect_vec()))
            .all(db)
            .await?;

        let tournament_uuids = participants.into_iter().map(|p| Some(p.tournament_id)).collect_vec();
        Ok(tournament_uuids)
    }

    pub async fn get_all_in_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Vec<Self>, DbErr> where C: ConnectionTrait {
        let p1_alias = Alias::new("p1");
        let p2_alias = Alias::new("p2");

        let rows = schema::participant_clash::Entity::find()
            .join_as(
                JoinType::InnerJoin,
                schema::participant_clash::Relation::Participant1.def(),
                p1_alias.clone()
            )
            .join_as(
                JoinType::InnerJoin,
                schema::participant_clash::Relation::Participant2.def(),
                p2_alias.clone()
            )
            .filter(
                SimpleExpr::Column(
                    ColumnRef::TableColumn(
                        SeaRc::new(p1_alias),
                        SeaRc::new(schema::participant::Column::TournamentId)
                    )
                ).eq(tournament_id).and(
                    SimpleExpr::Column(
                        ColumnRef::TableColumn(
                            SeaRc::new(p2_alias),
                            SeaRc::new(schema::participant::Column::TournamentId)
                        )
                    ).eq(tournament_id)
                )
            ).all(db).await?;
        Ok(rows.into_iter().map(Self::from_model).collect())
    }
}
