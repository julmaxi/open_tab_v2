use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::hash::Hash;

use async_trait::async_trait;
use sea_orm::prelude::*;

use sea_orm::Iterable;

use itertools::Itertools;


#[derive(Debug, PartialEq, Eq, Clone, Hash, Ord, PartialOrd)]
pub enum SortableValueWrapper {
    Uuid(Uuid)
}

impl From<sea_orm::Value> for SortableValueWrapper {
    fn from(value: sea_orm::Value) -> Self {
        match value {
            sea_orm::Value::Uuid(Some(val)) => SortableValueWrapper::Uuid(*val),
            _ => panic!("SortableValueWrapper::from only supports Uuid")
        }
    }
}

impl From<Uuid> for SortableValueWrapper {
    fn from(value: Uuid) -> Self {
        SortableValueWrapper::Uuid(value)
    }
}

pub async fn load_many<E, Conn, Col, M, T>(db: &Conn, values: Vec<T>) -> Result<Vec<Option<M>>, DbErr> where E: EntityTrait<Column=Col, Model=M>, M: ModelTrait<Entity = E>, Conn: ConnectionTrait, Col: ColumnTrait, T: Into<<E::PrimaryKey as PrimaryKeyTrait>::ValueType> + Into<sea_orm::Value> + Into<SortableValueWrapper> + Ord + Eq + Hash + Clone + Send + Sync + 'static {
    let keys : Vec<Col> = E::PrimaryKey::iter().map(|e| e.into_column()).collect();

    if keys.len() != 1 {
        panic!("load_many only supports entities with a single primary key");
    }

    let key = keys[0];

    let s = E::find();
    let s : Select<E> = s.filter(key.is_in(values.clone()));
    let models = s.all(db).await?;

    let new_positions = models.iter().enumerate().map(|(i, model)| (model.get(key).into(), i)).collect::<HashMap<SortableValueWrapper, usize>>();

    let mut out = vec![];

    for value in values {
        let value_pos = new_positions.get(&value.into());
        match value_pos {
            Some(pos) => {
                let model = models[*pos].clone();
                out.push(Some(model));
            },
            None => {
                out.push(None);
            }
        }
    }

   Ok(out)
}

#[derive(Debug)]
pub enum BatchLoadError {
    RowNotFound,
    DbErr(DbErr)
}

impl Display for BatchLoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<DbErr> for BatchLoadError {
    fn from(err: DbErr) -> Self {
        BatchLoadError::DbErr(err)
    }
}

impl Error for BatchLoadError {}


#[async_trait]
pub trait BatchLoad {
    type E: EntityTrait;
    type Col;
    type M;
    async fn batch_load<Conn, T> (db: &Conn, values: Vec<T>) -> Result<Vec<Option<Self::M>>, DbErr> where Conn: sea_orm::ConnectionTrait, T: Into<<<Self::E as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType> + Into<sea_orm::Value> + Into<SortableValueWrapper> + Ord + Eq + Hash + Clone + Send + Sync + 'static;

    async fn batch_load_all<Conn, T> (db: &Conn, values: Vec<T>) -> Result<Vec<Self::M>, BatchLoadError> where Conn: sea_orm::ConnectionTrait, T: Into<<<Self::E as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType> + Into<sea_orm::Value> + Into<SortableValueWrapper> + Ord + Eq + Hash + Clone + Send + Sync + 'static {
        let results = Self::batch_load(db, values).await?;

        results.into_iter().map(|d| {
            d.ok_or(BatchLoadError::RowNotFound)
        }).collect::<Result<Vec<_>, _>>()
    }
}

#[async_trait]
impl<E2: sea_orm::EntityTrait> BatchLoad for E2 {
    type E = E2;
    type Col = E2::Column;
    type M = E2::Model;

    async fn batch_load<Conn, T> (db: &Conn, values: Vec<T>) -> Result<Vec<Option<Self::M>>, DbErr> where Conn: sea_orm::ConnectionTrait, T: Into<<<Self::E as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType> + Into<sea_orm::Value> + Into<SortableValueWrapper> + Ord + Eq + Hash + Clone + Send + Sync + 'static {
        load_many(db, values).await
    } 

}