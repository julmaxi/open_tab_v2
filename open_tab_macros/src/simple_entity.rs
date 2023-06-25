extern crate proc_macro;

use crate::utilities::find_path_name_value_attr;
use core::panic;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput, Path, Type};

fn check_is_option(ty: &Type) -> bool {
    match ty {
        Type::Path(p) => {
            p.path.segments.len() == 1 && p.path.segments[0].ident == "Option"
        },
        _ => false
    }
}

pub fn simple_entity_derive_impl(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Used to store all the field names
    let field_names_and_types = match input.data {
        syn::Data::Struct(s) => {
            match s.fields {
                syn::Fields::Named(n) => {
                    n.named.iter().map(|f| (
                        f.ident.clone().unwrap(),
                        f.ty.clone(),
                        f.attrs.iter().any(|attr| attr.path().is_ident("serialize"))
                    )).collect::<Vec<_>>()
                },
                _ => panic!("Only named fields are supported")
            }
        }
        _ => panic!("Only structs are supported")
    };
    let name = input.ident;
    let sea_orm_mod_path: Path = find_path_name_value_attr(&input.attrs, "module_path").expect("No module_path attribute found");
    let get_many_tournaments_func : Option<Path> = find_path_name_value_attr(&input.attrs, "get_many_tournaments_func");
    let tournament_id : Option<Path> = find_path_name_value_attr(&input.attrs, "tournament_id");
    
    let get_tournaments_func = match (tournament_id, get_many_tournaments_func) {
        (None, None) => panic!("Must have either tournament_id or get_many_tournaments_func attributes"),
        (Some(tournament_id), None) => {
            let (_, tournament_type, _) = field_names_and_types.iter().find(|(f, _, _)| f.to_string() == tournament_id.segments.last().unwrap().into_token_stream().to_string()).expect("tournament_id not found in fields");
            if tournament_type.into_token_stream().to_string().starts_with("Option") {
                quote! {
                    async fn get_many_tournaments<C>(_db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, Box<dyn std::error::Error>> where C: ConnectionTrait {
                        return Ok(entities.iter().map(|tournament| {
                            tournament.#tournament_id
                        }).collect());
                    }
                }    
            }
            else {
                quote! {
                    async fn get_many_tournaments<C>(_db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, Box<dyn std::error::Error>> where C: ConnectionTrait {
                        return Ok(entities.iter().map(|tournament| {
                            Some(tournament.#tournament_id)
                        }).collect());
                    }
                }    
            }
        },
        (None, Some(get_many_tournaments_func)) => {
            quote! {
                async fn get_many_tournaments<C>(db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, Box<dyn std::error::Error>> where C: ConnectionTrait {
                    Self::#get_many_tournaments_func(db, entities).await
                }
            }
        },
        (_, _) => panic!("Cannot have both tournament_id and get_many_tournaments_func attributes"),
    };

    let active_value_assignment = field_names_and_types.iter().map(|(f, ty, serialize)| {
        if *serialize {
            if check_is_option(ty) {
                quote! {
                    #f: sea_orm::ActiveValue::Set(serde_json::to_string(&self.#f).ok())
                }    
            }
            else {
                quote! {
                    #f: sea_orm::ActiveValue::Set(serde_json::to_string(&self.#f).unwrap())
                }
            }
        }
        else {
            quote! {
                #f: sea_orm::ActiveValue::Set(self.#f.clone().try_into().unwrap())
            }    
        }
    }).collect::<Vec<_>>();

    let raw_attr_assignment = field_names_and_types.iter().map(|(f, ty, serialize)| {
        if *serialize {
            if check_is_option(ty) {
                quote! {
                    #f: model.#f.map(|v| serde_json::from_str(&v).ok()).flatten()
                }    
            }
            else {
                quote! {
                    #f: serde_json::from_str(&model.#f).unwrap()
                }
            }
        }
        else {
            quote! {
                #f: model.#f.clone().try_into().unwrap()
            }
        }
    }).collect::<Vec<_>>();

    let active_model_path = quote! {
        #sea_orm_mod_path::ActiveModel
    };

    let entity_path = quote! {
        #sea_orm_mod_path::Entity
    };

    let model_path = quote! {
        #sea_orm_mod_path::Model
    };

    let uuid_col_path = quote! {
        #sea_orm_mod_path::Column::Uuid
    };

    // Generate the output
    let expanded = quote! {
        impl #name {
            pub fn into_active_model(&self) -> #active_model_path {
                #active_model_path {
                    #(#active_value_assignment),*
                }
            }

            pub fn from_model(model: #model_path) -> Self {
                Self {
                    #(#raw_attr_assignment),*
                }
            }    
        }

        #[async_trait]
        impl crate::domain::entity::TournamentEntity for #name {
            async fn save<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), Box<dyn std::error::Error>> where C: ConnectionTrait {
                let model = self.into_active_model();
                if guarantee_insert {
                    model.insert(db).await?;
                }
                else {
                    let existing_model = #entity_path::find().filter(#uuid_col_path.eq(self.uuid)).one(db).await?;
                    if let Some(_) = existing_model {
                        model.update(db).await?;
                    }
                    else {
                        model.insert(db).await?;
                    }
                };
        
                Ok(())
            }

            #get_tournaments_func
        }

        #[async_trait]
        impl crate::domain::entity::LoadEntity for #name {
            async fn try_get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Option<Self>>, Box<dyn std::error::Error>> where C: ConnectionTrait {
                let models: Vec<Option<#model_path>> =  <#entity_path as crate::utilities::BatchLoad>::batch_load(db, uuids.clone()).await?;
                Ok(models.into_iter().map(|model| {
                    match model {
                        Some(model) => Some(Self::from_model(model)),
                        None => None
                    }
                }).collect())
            }
        }
    };

    TokenStream::from(expanded)
}
