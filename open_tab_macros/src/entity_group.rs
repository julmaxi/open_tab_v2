use std::collections::HashMap;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Ident, Type};

use quote::quote;


pub fn entity_group_derive_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let _name = quote!{input.ident};

    let variants_with_content_type = match input.data.clone() {
        syn::Data::Enum(e) => {
            e.variants.iter().map(|v| {
                assert!(v.fields.len() == 1, "Only enums with one field are supported");
                let field = &v.fields.iter().next().unwrap();
                assert!(field.ident.is_none(), "Only enums with unnamed fields are supported");
                return (v.ident.clone(), field.ty.clone());
            }).collect::<Vec<_>>()
        },
        _ => panic!("Only enums are supported")
    };

    let entity_vec_idents = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let variant_str = variant.to_string();
        let entity_vec_ident : String = variant_str.chars().enumerate().flat_map(|(idx, c)| {
            if idx > 0 && c.is_uppercase() {
                vec!['_', c.to_lowercase().next().unwrap()]
            }
            else {
                vec![c.to_lowercase().next().unwrap()]
            }
        }).chain("s".chars()).collect();

        let vec_ident = Ident::new(&format!("{}", entity_vec_ident), variant.span());
        (variant_str, vec_ident)
    }).collect::<HashMap<_, _>>();

    let entity_vec_declarations = 
        variants_with_content_type.iter().enumerate().map(|(_i, (variant, content_type))| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).unwrap();
        quote! {
            pub #vec_ident : Vec<#content_type>
        }
    });

    let group_ident = Ident::new(&format!("EntityGroup"), input.ident.span());

    let struct_declaration = quote! {
        pub struct #group_ident {
            #(#entity_vec_declarations),*,

            pub versions: HashMap<(String, Uuid), Uuid>,
            pub insertion_order: Vec<(String, Uuid)>,        
        }
    };

    let new_fn_assignments = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).unwrap();
        quote! {
            #vec_ident: Vec::new()
        }
    });

    let new_fn = quote! {
        fn new() -> Self {
            Self {
                #(#new_fn_assignments),*,
                versions: HashMap::new(),
                insertion_order: Vec::new(),
            }
        }
    };
    let id = input.ident.clone();

    let add_match_fn = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).expect("No vec ident found");
        quote! {
            #id::#variant(entity) => {
                self.#vec_ident.push(entity);
            }
        }
    });

    let add_fn = quote! {
        fn add(&mut self, entity: #id) {
            self.insertion_order.push((entity.get_name().clone(), entity.get_uuid()));
            match entity {
                #(#add_match_fn),*
            }
        }
    };

    let get_all_tournament_extends = variants_with_content_type.iter().map(|(variant, content_type)| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).unwrap();
        quote! {
            out.extend(<#content_type as crate::domain::entity::TournamentEntity>::get_many_tournaments(db, &self.#vec_ident.iter().collect()).await?.into_iter());
        }
    });

    let get_all_tournaments_fn = quote! {
        async fn get_all_tournaments<C>(&self, db: &C) -> Result<Vec<Option<sea_orm::prelude::Uuid>>, Box<dyn std::error::Error>> where C: sea_orm::ConnectionTrait {
            let mut out = Vec::new();
            #(#get_all_tournament_extends)*
            Ok(out)
        }
    };

    let save_all_statements = variants_with_content_type.iter().map(|(variant, content_type)| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).unwrap();
        quote! {
            <#content_type as crate::domain::entity::TournamentEntity>::save_many(db, guarantee_insert, &self.#vec_ident.iter().collect()).await?;
        }
    });

    let save_all_fn = quote! {
        async fn save_all_with_options<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), Box<dyn std::error::Error>> where C: sea_orm::ConnectionTrait {
            #(#save_all_statements)*
            Ok(())
        }
    };

    let get_entity_statements = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).unwrap();
        let name_as_string = format!("\"{}\"", variant);
        quote! {
            .chain(self.#vec_ident.iter().map(|p| (#name_as_string.to_string(), p.uuid.clone())))
        }
    });

    let get_entity_fn = quote! {
        fn get_entity_ids(&self) -> Vec<(String, Uuid)> {
            std::iter::empty::<(String, Uuid)>()
            #(#get_entity_statements)*
            .collect()
        }
    };

    let from_impl = quote! {
        impl From<Vec<#id>> for #group_ident {
            fn from(entities: Vec<#id>) -> Self {
                let mut groups = #group_ident::new();
        
                for e in entities {
                    groups.add(e);
                }
        
                groups
            }
        }
    };

    let from_versioned_impl = quote! {
        impl From<Vec<VersionedEntity<#id>>> for #group_ident {
            fn from(entities: Vec<VersionedEntity<#id>>) -> Self {
                let mut groups = #group_ident::new();
        
                for e in entities {
                    groups.add_versioned(e.entity, e.version);
                }
        
                groups
            }
        }
    };

    let get_many_with_type_arms = variants_with_content_type.iter().map(|(variant, content_type)| {
        let variant_as_str = variant.to_string();
        quote! {
            #variant_as_str => {
                #content_type::get_many(db, ids).await?.into_iter().map(|e| Entity::#variant(e)).collect()                
            }
        }
    });

    let get_many_fn = quote! {
        async fn get_many_with_type<C>(db: &C, entity_type: &str, ids: Vec<Uuid>) -> Result<Vec<Entity>, Box<dyn Error>> where C: sea_orm::ConnectionTrait {
            Ok(match entity_type {
                #(#get_many_with_type_arms),*,
                _ => panic!("Unknown Entity Type {}", entity_type)
            })
        }
    };

    let entity_impl = proc_macro2::TokenStream::from(derive_entity_impl(&input, &variants_with_content_type));

    let expanded = quote! {        
        #entity_impl

        #struct_declaration

        #[async_trait::async_trait]
        impl crate::group::EntityGroupTrait for #group_ident {
            #new_fn
            #add_fn
            #get_all_tournaments_fn
            #save_all_fn
            #get_entity_fn
            #get_many_fn

            fn add_versioned(&mut self, e: Entity, version: Uuid) {
                self.versions.insert((e.get_name(), e.get_uuid()), version);
                self.add(e);
            }

            async fn save_log_with_tournament_id<C>(&self, transaction: &C, tournament_id: Uuid) -> Result<Uuid, Box<dyn Error>> where C: sea_orm::ConnectionTrait {
                let last_log_entry = tournament_log::Entity::find()
                .filter(tournament_log::Column::TournamentId.eq(tournament_id))
                .order_by_desc(tournament_log::Column::SequenceIdx)
                .limit(1)
                .one(transaction)
                .await?;
        
                let last_sequence_idx = match &last_log_entry {
                    Some(entry) => entry.sequence_idx,
                    None => 0,
                };
                let mut log_head = match &last_log_entry {
                    Some(entry) => entry.uuid,
                    None => Uuid::nil(),
                };
        
                let new_entries = self.insertion_order.iter().map(|e| e.clone()).enumerate().map(|(idx, (name, uuid))| {
                    let version_uuid = self.versions.get(&(name.clone(), uuid.clone())).map(|u| *u).unwrap_or_else(Uuid::new_v4);
                    tournament_log::ActiveModel {
                        uuid: ActiveValue::Set(version_uuid),
                        timestamp: ActiveValue::Set(chrono::offset::Local::now().naive_local()),
                        sequence_idx: ActiveValue::Set(last_sequence_idx + 1 + idx as i32),
                        tournament_id: ActiveValue::Set(tournament_id),
                        target_type: ActiveValue::Set(name),
                        target_uuid: ActiveValue::Set(uuid)
                    }
                }).collect_vec();
        
                if new_entries.len() > 0 {
                    log_head = new_entries[new_entries.len() - 1].uuid.clone().unwrap();
                    tournament_log::Entity::insert_many(new_entries).exec(transaction).await?;
                }
        
                Ok(log_head)
            }
        }

        #from_impl
        #from_versioned_impl
    };

    TokenStream::from(expanded)
}

pub fn derive_entity_impl(input: &DeriveInput, variants_with_content_type: &Vec<(Ident, Type)>) -> TokenStream {
    let entity_ident = &input.ident;

    let get_proc_order_arms = variants_with_content_type.iter().enumerate().map(|(idx, (variant, _content_type))| {
        let idx_lit = syn::LitInt::new(&idx.to_string(), entity_ident.span());
        quote! {
            #entity_ident::#variant(_) => #idx_lit,
        }
    });

    let get_proc_order_fn = quote! {
        pub fn get_processing_order(&self) -> u64 {
            match self {
                #(#get_proc_order_arms)*
            }
        }
    };

    let get_name_arms = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let name_as_string = format!("{}", variant);
        quote! {
            #entity_ident::#variant(_) => #name_as_string.to_string(),
        }
    });

    let get_name_fn = quote! {
        pub fn get_name(&self) -> String {
            match self {
                #(#get_name_arms)*
            }
        }
    };

    let get_uuid_arms = variants_with_content_type.iter().map(|(variant, _content_type)| {
        quote! {
            #entity_ident::#variant(e) => e.uuid,
        }
    });

    let get_uuid_fn = quote! {
        pub fn get_uuid(&self) -> Uuid {
            match self {
                #(#get_uuid_arms)*
            }
        }
    };

    let expanded = quote! {
        impl #entity_ident {
            #get_proc_order_fn
            #get_name_fn
            #get_uuid_fn
        }

        impl PartialOrd for Entity {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        
        impl Ord for Entity {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                u64::cmp(&self.get_processing_order(), &other.get_processing_order())
            }
        }
    };

    TokenStream::from(expanded)
}