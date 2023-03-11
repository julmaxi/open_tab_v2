use async_trait::async_trait;
use std::error::Error;
use sea_orm::{prelude::{Uuid, DatabaseConnection}, ConnectionTrait};

#[async_trait]
pub trait TournamentEntity: Send + Sync {
    async fn save<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        Self::save_many(db, guarantee_insert, &vec![self]).await
    }
    async fn save_many<C>(db: &C, guarantee_insert: bool, entities: &Vec<&Self>) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        for entity in entities.iter() {
            entity.save(db, guarantee_insert).await?;
        }
        Ok(())
    }

    async fn get_tournament<C>(&self, db: &C) -> Result<Option<Uuid>, Box<dyn Error>> where C: ConnectionTrait {
        Ok(Self::get_many_tournaments(db, &vec![self]).await?[0])
    }

    async fn get_many_tournaments<C>(db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, Box<dyn Error>> where C: ConnectionTrait {
        let mut out = vec![];

        for entity in entities {
            out.push(entity.get_tournament(db).await?);
        }
        Ok(out)
    }
}
