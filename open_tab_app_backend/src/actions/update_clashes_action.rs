

use open_tab_entities::{domain::entity::LoadEntity, Entity, EntityGroup, EntityTypeId};
use sea_orm::prelude::Uuid;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::actions::ActionTrait;

use open_tab_entities::domain::participant_clash::ParticipantClash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateClashes {
    pub tournament_id: Uuid,
    #[serde(default)]
    pub updated_clashes: Vec<ClashUpdate>,
    #[serde(default)]
    pub deleted_clashes: Vec<Uuid>   
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClashUpdate {
    pub clash_id: Uuid,
    pub approve: bool
}


#[async_trait]
impl ActionTrait for UpdateClashes {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let all_clashes = ParticipantClash::get_many(db, self.updated_clashes.iter().map(|c| c.clash_id).collect()).await?;
        let mut all_clashes_by_id = all_clashes.into_iter().map(|c| (c.uuid, c)).collect::<std::collections::HashMap<_, _>>();
        let mut g = EntityGroup::new(self.tournament_id);

        for update in self.updated_clashes {
            let clash = all_clashes_by_id.remove(&update.clash_id);
            //We skip clashes that are not found. This allows us to more gracefully deal with
            //duplicated clash ids in the update.
            if let Some(mut clash) = clash {
                clash.is_approved = update.approve;
                clash.was_seen = true;
                g.add(
                    Entity::ParticipantClash(clash)
                );    
            }
        }

        for clash_id in self.deleted_clashes {
            g.delete(
                EntityTypeId::ParticipantClash,
                clash_id
            );
        }
        Ok(
            g
        )       
    }
}