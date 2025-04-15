use crate::{assets::save_named_asset, state::{self, AppState}};
use sea_orm::{ActiveModelTrait, EntityTrait, Set, TransactionTrait};
use std::fs::File;
use std::path::Path;
use csv::ReaderBuilder;
use sea_orm::prelude::Uuid;
use open_tab_entities::schema::{self, asset, institution_alias, well_known_institution};

#[derive(clap::Subcommand)]
pub enum Command {
    AddInstitutions {
        path: String
    },
    AddAwardSeries {
        path: String
    }
}

impl Command {
    pub async fn run(&self, app_state: AppState) -> anyhow::Result<()> {
        match self {
            Command::AddInstitutions { path } => {
                let csv_path = Path::new(path).join("institutions.csv");
                dbg!(&csv_path);
                let mut reader = ReaderBuilder::new().from_path(&csv_path)?;

                let mut to_insert_institutions = vec![];
                let mut to_insert_aliases = vec![];


                for record in reader.records() {
                    let record = record?;
                    let id = record.get(0);
                    let name = record.get(1);
                    let aliases = record.get(2);
                    let icon_name = record.get(3).map(|s| s.trim()).filter(|s| s.len() > 0);
                    let header_name = record.get(4).map(|s| s.trim()).filter(|s| s.len() > 0);

                    match (id, name, aliases) {
                        (Some(id), Some(name), Some(aliases)) => {
                            let icon_uuid: Option<Uuid> = if let Some(icon_name) = icon_name {
                                let content = std::fs::read(Path::new(path).join(icon_name)).inspect_err(|e| eprintln!("Error while reader icon: {}", e))?;
                                let filetype = crate::assets::AssetFileType::from_filename(icon_name)?;
                                Some(save_named_asset(
                                    &app_state,
                                    content,
                                    filetype,
                                    id.to_owned() + "_icon",
                                ).await?)
                            } else {
                                None
                            };
        
                            let header_uuid = if let Some(header_name) = header_name {
                                let content = std::fs::read(Path::new(path).join(header_name)).inspect_err(|e| eprintln!("Error while reader header: {}", e))?;
                                let filetype = crate::assets::AssetFileType::from_filename(header_name)?;
                                Some(save_named_asset(
                                    &app_state,
                                    content,
                                    filetype,
                                    id.to_owned() + "_header",
                                ).await?)
                            } else {
                                None
                            };

                            let alias_list = aliases.split(';').map(|s| s.trim()).collect::<Vec<_>>();

                            let uuid = Uuid::new_v4();
                            let institution = well_known_institution::ActiveModel {
                                uuid: Set(uuid),
                                name: Set(name.to_string()),
                                short_name: Set(id.to_string()),
                                tiny_image: Set(icon_uuid),
                                header_image: Set(header_uuid),
                            };

                            to_insert_institutions.push(institution);
                            for alias in alias_list {
                                let alias = institution_alias::ActiveModel {
                                    id: sea_orm::ActiveValue::NotSet,
                                    institution: sea_orm::ActiveValue::Set(uuid),
                                    alias: sea_orm::ActiveValue::Set(alias.to_string()),
                                };
                                to_insert_aliases.push(alias);
                            }
                        }
                        _ => {}
                    }
                }

                
                let transaction = app_state.db.begin().await?;

                schema::well_known_institution::Entity::insert_many(to_insert_institutions).exec(&transaction).await?;
                schema::institution_alias::Entity::insert_many(to_insert_aliases).exec(&transaction).await?;
                transaction.commit().await?;
                Ok(())
            }
            Command::AddAwardSeries { path } => {
                let csv_path = Path::new(path).join("award_series.csv");
                dbg!(&csv_path);
                let mut reader = ReaderBuilder::new().from_path(&csv_path)?;

                let mut to_insert_award_series = vec![];

                for record in reader.records() {
                    let record = record?;
                    let id = record.get(0);
                    let name = record.get(1);
                    let prestige = record.get(2).and_then(|s| s.parse::<i32>().ok());
                    let image_name = record.get(3).map(|s| s.trim()).filter(|s| s.len() > 0);

                    match (id, name, prestige) {
                        (Some(id), Some(name), Some(prestige)) => {
                            let image_uuid: Option<Uuid> = if let Some(image_name) = image_name {
                                let content = std::fs::read(Path::new(path).join(image_name)).inspect_err(|e| eprintln!("Error while reading image: {}", e))?;
                                let filetype = crate::assets::AssetFileType::from_filename(image_name)?;
                                Some(save_named_asset(
                                    &app_state,
                                    content,
                                    filetype,
                                    id.to_owned() + "_image",
                                ).await?)
                            } else {
                                None
                            };

                            let award_series = schema::award_series::ActiveModel {
                                uuid: Set(Uuid::new_v4()),
                                short_name: Set(id.to_string()),
                                name: Set(name.to_string()),
                                prestige: Set(prestige),
                                image: Set(image_uuid),
                            };
                            to_insert_award_series.push(award_series);
                        }
                        _ => {}
                    }
                }

                let transaction = app_state.db.begin().await?;
                schema::award_series::Entity::insert_many(to_insert_award_series).exec(&transaction).await?;
                transaction.commit().await?;
                Ok(())
            }
        }
    }
}