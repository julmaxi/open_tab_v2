use async_trait::async_trait;
use itertools::Itertools;
use std::iter::zip;

use sea_orm::{prelude::Uuid};

#[async_trait]
pub trait TournamentEntity: Send + Sync {
    async fn save<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
        Self::save_many(db, guarantee_insert, &vec![self]).await
    }
    
    async fn save_many<C>(db: &C, guarantee_insert: bool, entities: &Vec<&Self>) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
        for entity in entities.iter() {
            entity.save(db, guarantee_insert).await?;
        }
        Ok(())
    }

    async fn delete<C>(db: &C, uuid: Uuid) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
        Self::delete_many(db, vec![uuid]).await
    }
    
    async fn delete_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait;
    /*
     where C: sea_orm::ConnectionTrait {
        for uuid in uuids.iter() {
            Self::delete(db, *uuid).await?;
        }
        Ok(())
    } */

    async fn get_tournament<C>(&self, db: &C) -> Result<Option<Uuid>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Ok(Self::get_many_tournaments(db, &vec![self]).await?[0])
    }

    async fn get_many_tournaments<C>(db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut out = vec![];

        for entity in entities {
            out.push(entity.get_tournament(db).await?);
        }
        Ok(out)
    }
}

#[async_trait]
pub trait LoadEntity: Sized {
    async fn try_get<C>(db: &C, uuid: Uuid) -> Result<Option<Self>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Self::try_get_many(db, vec![uuid]).await?.pop().ok_or(LoadError::EmptyVec.into())
    }

    async fn get<C>(db: &C, uuid: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Self::get_many(db, vec![uuid]).await?.pop().ok_or(LoadError::TooManyElements.into())
    }

    async fn try_get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Option<Self>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut out = vec![];

        for uuid in uuids {
            out.push(Self::try_get(db, uuid).await?);
        }
        Ok(out)
    }

    async fn get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Self>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let vals = Self::try_get_many(db, uuids.clone()).await?;

        let missing_uuids = zip(uuids.iter(), vals.iter()).filter_map(
            |(uuid, val)| {
                match val {
                    Some(_) => None,
                    None => Some(uuid)
                }
            }
        ).collect_vec();

        if missing_uuids.len() > 0 {
            return Err(anyhow::Error::new(LoadError::EntitiesNotFound {uuids: missing_uuids.into_iter().map(|u| u.clone()).collect()}));
        }

        Ok(vals.into_iter().map(|val| val.unwrap()).collect())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum LoadError {
    #[error("Entity not found: {}", uuids.iter().map(|uuid| uuid.to_string()).collect::<Vec<String>>().join(", "))]
    EntitiesNotFound {uuids: Vec<Uuid>},
    #[error("Empty vec")]
    EmptyVec,
    #[error("Too many elements")]
    TooManyElements
}