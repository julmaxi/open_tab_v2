extern crate proc_macro;

use crate::utilities::{find_path_name_value_attr, find_skip_values};
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

    let skip_fields = find_skip_values(&input.attrs);

    let name = input.ident;
    let sea_orm_mod_path: Path = find_path_name_value_attr(&input.attrs, "module_path").expect("No module_path attribute found");
    let get_many_tournaments_func : Option<Path> = find_path_name_value_attr(&input.attrs, "get_many_tournaments_func");
    let tournament_id : Option<Path> = find_path_name_value_attr(&input.attrs, "tournament_id");
    
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
    })
    .chain(
        skip_fields.iter().map(|f| {
            quote! {
                #f: sea_orm::ActiveValue::NotSet
            }
        })
    )
    .collect::<Vec<_>>();

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

    let get_related_uuid_fields = field_names_and_types.iter().filter(
        |(_, t, _)| t.into_token_stream().to_string() == "Uuid"
    ).map(|(f, _, _)| f).collect::<Vec<_>>();

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
        impl<C> crate::domain::entity::BoundTournamentEntityTrait<C> for #name  where C: sea_orm::ConnectionTrait {
            async fn save(&self, db: &C, guarantee_insert: bool) -> Result<(), anyhow::Error> {
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

            async fn delete_many(db: &C, uuids: Vec<Uuid>) -> Result<(), anyhow::Error> {
                #entity_path::delete_many().filter(
                    #uuid_col_path.is_in(uuids)
                ).exec(db).await?;
                Ok(())
            }
        }

        impl crate::domain::entity::TournamentEntityTrait for #name {
            fn get_related_uuids(&self) -> Vec<Uuid> {
                vec![#(self.#get_related_uuid_fields),*]
            }
        }

        #[async_trait]
        impl crate::domain::entity::LoadEntity for #name {
            async fn try_get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Option<Self>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
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
