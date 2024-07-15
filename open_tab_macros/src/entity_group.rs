use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{parse_macro_input, DeriveInput, Ident, Type};

use quote::quote;


pub fn entity_group_derive_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let _name = quote!{input.ident};

    let variants_with_content_type = match input.data.clone() {
        syn::Data::Enum(e) => {
            e.variants.iter().map(|v| {
                assert!(v.fields.len() == 1, "Only enums with one field are supported");
                let field = &v.fields.iter().next().expect("No fields found");
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
                vec!['_', c.to_lowercase().next().expect("Making Ident Failed")]
            }
            else {
                vec![c.to_lowercase().next().expect("Making Ident Failed")]
            }
        }).chain("s".chars()).collect();

        let vec_ident = Ident::new(&format!("{}", entity_vec_ident), variant.span());
        (variant_str, vec_ident)
    }).collect::<HashMap<_, _>>();

    let entity_vec_declarations = 
        variants_with_content_type.iter().enumerate().map(|(_i, (variant, content_type))| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).expect("No vec ident found");
        quote! {
            pub #vec_ident : Vec<#content_type>
        }
    });

    let group_ident = Ident::new(&format!("EntityGroup"), input.ident.span());

    let entity_type_enum = proc_macro2::TokenStream::from(derive_entity_type_enum(&variants_with_content_type));

    let struct_declaration = quote! {
        pub struct #group_ident {
            #(#entity_vec_declarations),*,

            pub deletions: std::collections::HashSet<(EntityType, Uuid)>,

            pub versions: HashMap<(EntityType, Uuid), Uuid>,
            pub insertion_order: Vec<(EntityType, Uuid)>,        
        }
    };

    let new_fn_assignments = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).expect("No vec ident found");
        quote! {
            #vec_ident: Vec::new()
        }
    });

    let new_fn = quote! {
        fn new() -> Self {
            Self {
                #(#new_fn_assignments),*,
                versions: HashMap::new(),
                deletions: std::collections::HashSet::new(),
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
            self.insertion_order.push((entity.get_type().clone(), entity.get_uuid()));
            self.deletions.remove(&(entity.get_type().clone(), entity.get_uuid()));
            match entity {
                #(#add_match_fn),*
            }
        }
    };

    let get_all_tournament_extends = variants_with_content_type.iter().map(|(variant, content_type)| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).expect("No vec ident found");
        quote! {
            let entity_type_tournaments = <#content_type as crate::domain::entity::BoundTournamentEntityTrait<C>>::get_many_tournaments(db, &self.#vec_ident.iter().collect()).await?;
            out.extend(entity_type_tournaments.into_iter());
        }
    });

    let get_all_tournaments_fn = quote! {
        async fn get_all_tournaments<C>(&self, db: &C) -> Result<Vec<Option<sea_orm::prelude::Uuid>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
            let mut out = Vec::new();
            #(#get_all_tournament_extends)*
            Ok(out)
        }
    };

    let save_all_statements = variants_with_content_type.iter().map(|(variant, content_type)| {
        let variant_name : String = variant.to_string();
        let vec_ident = entity_vec_idents.get(&variant_name).expect("No vec ident found");
        quote! {
            <#content_type as crate::domain::entity::BoundTournamentEntityTrait<C>>::save_many(db, guarantee_insert, &self.#vec_ident.iter().filter(
                |p| !self.deletions.contains(&(EntityType::#variant, p.uuid.clone()))
            ).collect()).await?;
        }
    });

    let delete_statements = variants_with_content_type.iter().map(|(variant, _content_type)| {
        quote! {
            let v = Vec::new();
            let delete_uuids = delete_map.get(&EntityType::#variant).unwrap_or(&v);
            if !delete_uuids.is_empty() {
                <#variant as crate::domain::entity::BoundTournamentEntityTrait<C>>::delete_many(db, delete_uuids.clone()).await?;
            }
        }
    });

    let save_all_fn = quote! {
        async fn save_all_with_options<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
            let delete_map = self.deletions.clone().into_iter().into_group_map();
            #(#delete_statements)*
            #(#save_all_statements)*
            Ok(())
        }
    };

    let get_entity_statements = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).unwrap();
        quote! {
            .chain(self.#vec_ident.iter().map(|p| (EntityType::#variant, p.uuid.clone())))
        }
    });

    let get_entity_fn = quote! {
        fn get_entity_ids(&self) -> Vec<(EntityType, Uuid)> {
            std::iter::empty::<(EntityType, Uuid)>()
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
        impl From<Vec<VersionedEntity<#id, EntityType>>> for #group_ident {
            fn from(entities: Vec<VersionedEntity<#id, EntityType>>) -> Self {
                let mut groups = #group_ident::new();
        
                for e in entities {
                    let version = e.version;
                    match e.entity {
                        EntityState::Exists(entity) => {
                            groups.add_versioned(entity, version);
                        },
                        EntityState::Deleted{type_, uuid} => {
                            groups.delete_versioned(type_, uuid, version);
                        }
                    }
                }
        
                groups
            }
        }
    };

    let from_state_impl = quote! {
        impl From<Vec<EntityState<#id, EntityType>>> for #group_ident {
            fn from(entities: Vec<EntityState<#id, EntityType>>) -> Self {
                let mut groups = #group_ident::new();
        
                for e in entities {
                    match e {
                        EntityState::Exists(entity) => {
                            groups.add(entity);
                        },
                        EntityState::Deleted{type_, uuid} => {
                            groups.delete(type_, uuid);
                        }
                    }
                }
        
                groups
            }
        }
    };

    let get_many_with_type_arms = variants_with_content_type.iter().map(|(variant, content_type)| {
        quote! {
            EntityType::#variant => {
                <#content_type as crate::domain::entity::LoadEntity>::get_many(db, ids).await?.into_iter().map(|e| Entity::#variant(e)).collect()                
            }
        }
    });

    let get_many_fn = quote! {
        async fn get_many_with_type<C>(db: &C, entity_type: Self::TypeId, ids: Vec<Uuid>) -> Result<Vec<Entity>, anyhow::Error> where C: sea_orm::ConnectionTrait {
            Ok(match entity_type {
                #(#get_many_with_type_arms),*,
                _ => panic!("Unknown Entity Type {:?}", entity_type)
            })
        }
    };

    let try_get_many_with_type_arms = variants_with_content_type.iter().map(|(variant, content_type)| {
        let _variant_as_str = variant.to_string();
        quote! {
            EntityType::#variant => {
                #content_type::try_get_many(db, ids).await?.into_iter().map(|e| e.map(Entity::#variant)).collect()                
            }
        }
    });

    let try_get_many_fn = quote! {
        async fn try_get_many_with_type<C>(db: &C, entity_type: Self::TypeId, ids: Vec<Uuid>) -> Result<Vec<Option<Entity>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
            Ok(match entity_type {
                #(#try_get_many_with_type_arms),*,
                _ => panic!("Unknown Entity Type {:?}", entity_type)
            })
        }
    };

    let delete_fn = quote! {
        fn delete(&mut self, type_: Self::TypeId, uuid: Uuid) {
            self.insertion_order.push((type_.clone(), uuid));
            self.deletions.insert((type_, uuid));
        }
    };

    let delete_versioned_fn = quote! {
        fn delete_versioned(&mut self, type_: Self::TypeId, uuid: Uuid, version: Uuid) {
            self.versions.insert((type_.clone(), uuid), version);
            self.delete(type_, uuid);
        }
    };

    let entity_impl = proc_macro2::TokenStream::from(derive_entity_impl(&input, &variants_with_content_type, group_ident.clone()));

    let get_deletion_tournaments_fn = quote! {
        async fn get_all_deletion_tournaments<C>(&self, db: &C) -> Result<Vec<Option<Uuid>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
            let mut out : Vec<Option<Uuid>> = Vec::new();
            for (type_, ids) in self.deletions.iter().into_group_map_by(|e| e.0) {
                let ids = ids.into_iter().map(|e| e.1).collect_vec();
                out.extend(EntityType::try_get_tournaments_with_type(db, type_, ids).await?)
            }
            Ok(out)
        }
    };

    let expanded = quote! {        
        #entity_impl

        #struct_declaration
        #entity_type_enum


        #[async_trait::async_trait]
        impl crate::group::EntityGroupTrait for #group_ident {
            type TypeId = EntityType;
            #new_fn
            #add_fn
            #delete_fn
            #delete_versioned_fn
            #get_all_tournaments_fn
            #get_deletion_tournaments_fn
            #save_all_fn
            #get_entity_fn

            #get_many_fn
            #try_get_many_fn

            fn add_versioned(&mut self, e: Entity, version: Uuid) {
                self.versions.insert((e.get_type(), e.get_uuid()), version);
                self.add(e);
            }

            async fn save_log_with_tournament_id<C>(&self, transaction: &C, tournament_id: Uuid) -> Result<Uuid, anyhow::Error> where C: sea_orm::ConnectionTrait {
                let last_log_entry = crate::schema::tournament_log::Entity::find()
                .filter(crate::schema::tournament_log::Column::TournamentId.eq(tournament_id))
                .order_by_desc(crate::schema::tournament_log::Column::SequenceIdx)
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
        
                let new_entries = self.insertion_order.iter().map(|e| e.clone()).enumerate().map(|(idx, (type_, uuid))| {
                    let version_uuid = self.versions.get(&(type_.clone(), uuid.clone())).map(|u| *u).unwrap_or_else(Uuid::new_v4);
                    crate::schema::tournament_log::ActiveModel {
                        uuid: ActiveValue::Set(version_uuid),
                        timestamp: ActiveValue::Set(chrono::offset::Local::now().naive_local()),
                        sequence_idx: ActiveValue::Set(last_sequence_idx + 1 + idx as i32),
                        tournament_id: ActiveValue::Set(tournament_id),
                        target_type: ActiveValue::Set(type_.as_str().to_string()),
                        target_uuid: ActiveValue::Set(uuid)
                    }
                }).collect_vec();
        
                if new_entries.len() > 0 {
                    log_head = new_entries[new_entries.len() - 1].uuid.clone().unwrap();
                    crate::schema::tournament_log::Entity::insert_many(new_entries).exec(transaction).await?;
                }
        
                Ok(log_head)
            }
        }

        #from_impl
        #from_versioned_impl
        #from_state_impl
    };

    let group_map = proc_macro2::TokenStream::from(derive_grouped_entity_map_impl(&variants_with_content_type));

    let delete_map: proc_macro2::TokenStream = proc_macro2::TokenStream::from(derive_deletion_map_impl(&variants_with_content_type));
    
    let entity = proc_macro2::TokenStream::from(derive_entity_impl(&input, &variants_with_content_type, group_ident.clone()));

    let get_many_with_type_arms = variants_with_content_type.iter().map(|(variant, content_type)| {
        quote! {
            EntityType::#variant => {
                <#content_type as crate::domain::entity::LoadEntity>::get_many(db, ids).await?.into_iter().map(|e| Entity::#variant(e)).collect()                
            }
        }
    });

    let get_many_fn = quote! {
        async fn get_many_with_type<C>(db: &C, entity_type: Self::TypeId, ids: Vec<Uuid>) -> Result<Vec<Entity>, anyhow::Error> where C: sea_orm::ConnectionTrait {
            Ok(match entity_type {
                #(#get_many_with_type_arms),*,
                _ => panic!("Unknown Entity Type {:?}", entity_type)
            })
        }
    };

    let try_get_many_with_type_arms = variants_with_content_type.iter().map(|(variant, content_type)| {
        let _variant_as_str = variant.to_string();
        quote! {
            EntityTypeId::#variant => {
                #content_type::try_get_many(db, ids).await?.into_iter().map(|e| e.map(Entity::#variant)).collect()                
            }
        }
    });

    let try_get_many_fn = quote! {
        async fn try_get_many_with_type<C>(db: &C, entity_type: EntityTypeId, ids: Vec<Uuid>) -> Result<Vec<Option<Entity>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
            Ok(match entity_type {
                #(#try_get_many_with_type_arms),*,
                _ => panic!("Unknown Entity Type {:?}", entity_type)
            })
        }
    };

    let expanded = quote!{
        impl Entity {
            #try_get_many_fn
        }
        #entity_type_enum
        #delete_map
        #group_map

        #entity
    };

    TokenStream::from(expanded)
}

pub fn derive_entity_type_enum(variants_with_content_type: &Vec<(Ident, Type)>) -> TokenStream {
    let variants = variants_with_content_type.iter().map(|(variant, _content_type)| {
        quote! {
            #variant
        }
    });

    let from_str_arms = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let variant_as_str = format!("\"{}\"", variant.to_string());
        quote! {
            #variant_as_str => EntityTypeId::#variant
        }
    });

    let as_str_arms = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let variant_as_str = format!("\"{}\"", variant.to_string());
        quote! {
            EntityTypeId::#variant => #variant_as_str
        }
    });

    let try_get_tournaments_arms = variants_with_content_type.iter().map(|(variant, content_type)| {
        quote! {
            EntityTypeId::#variant => {
                let entities : Vec<Option<#content_type>> = <#content_type as crate::domain::entity::LoadEntity>::try_get_many(db, ids).await?;  
                let entities = entities.into_iter().filter_map(|e| e).collect_vec();

                <#content_type as crate::domain::entity::BoundTournamentEntityTrait<C>>::get_many_tournaments(db, &entities.iter().collect()).await?
            }           
        }
    });

    let get_tournament_fn: proc_macro2::TokenStream = quote! {
        pub async fn try_get_tournaments_with_type<C>(db: &C, entity_type: Self, ids: Vec<Uuid>, ) -> Result<Vec<Option<Uuid>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
            Ok(match entity_type {
                #(#try_get_tournaments_arms),*,
            })
        }
    };

    let get_proc_order_arms = variants_with_content_type.iter().enumerate().map(|(idx, (variant, _content_type))| {
        let idx_lit = syn::LitInt::new(&idx.to_string(), Span::call_site());
        quote! {
            EntityTypeId::#variant => #idx_lit,
        }
    });

    let get_proc_order_fn = quote! {
        fn get_processing_order(&self) -> u64 {
            match self {
                #(#get_proc_order_arms)*
            }
        }
    };

    let expanded = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Ord)]
        pub enum EntityTypeId {
            #(#variants),*
        }

        impl EntityTypeId {
            pub fn as_str(&self) -> &'static str {
                match self {
                    #(#as_str_arms),*
                }
            }
            
            #get_tournament_fn

            #get_proc_order_fn
        }

        impl PartialOrd for EntityTypeId {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.get_processing_order().cmp(&other.get_processing_order()))
            }
        }

        impl From<String> for EntityTypeId {
            fn from(s: String) -> Self {
                match s.as_str() {
                    #(#from_str_arms),*,
                    _ => panic!("Unknown Entity Type {}", s)
                }
            }
        }

        impl EntityTypeIdTrait for EntityTypeId {
            fn as_str(&self) -> &'static str {
                self.as_str()
            }
        }
    };

    TokenStream::from(expanded)
}

pub fn derive_grouped_entity_map_impl(variants_with_content_type: &Vec<(Ident, Type)>) -> TokenStream {
    let entity_vec_idents = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let variant_str = variant.to_string();
        let entity_vec_ident : String = variant_str.chars().enumerate().flat_map(|(idx, c)| {
            if idx > 0 && c.is_uppercase() {
                vec!['_', c.to_lowercase().next().expect("Making Ident Failed")]
            }
            else {
                vec![c.to_lowercase().next().expect("Making Ident Failed")]
            }
        }).chain("s".chars()).collect();

        let vec_ident = Ident::new(&format!("{}", entity_vec_ident), variant.span());
        (variant_str, vec_ident)
    }).collect::<HashMap<_, _>>();

    let entity_vec_declarations = 
        variants_with_content_type.iter().enumerate().map(|(_i, (variant, content_type))| {
  
        let vec_ident = entity_vec_idents.get(&variant.to_string()).expect("No vec ident found");
        quote! {
            pub #vec_ident : Vec<#content_type>
        }
    });

    let entity_initializers = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).expect("No vec ident found");
        quote! {
            #vec_ident: Vec::new()
        }
    });

    let add_arms = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).expect("No vec ident found");
        quote! {
            Entity::#variant(e) => self.#vec_ident.push(e),
        }
    });

    let get_group_statements = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).unwrap();
        quote! {
            out.insert(EntityTypeId::#variant, Box::new(self.#vec_ident));
        }
    });

    TokenStream::from(quote!{
        pub struct GroupedEntityMap {
            #(#entity_vec_declarations),*
        }

        impl GroupedEntityMapTrait<EntityTypeId, Entity> for GroupedEntityMap {
            fn new() -> Self {
                Self {
                    #(#entity_initializers),*
                }
            }
            fn add(&mut self, entity: Entity) {
                match entity {
                    #(#add_arms)*
                }
            }
        
            fn into_groups<C>(self) -> HashMap<EntityTypeId, Box<dyn BatchBoundTournamentEntityTrait<C>>> where C: ConnectionTrait {
                let mut out : HashMap<EntityTypeId, Box<dyn BatchBoundTournamentEntityTrait<C>>> = HashMap::new();
                #(#get_group_statements);*
                out
            }
        }        
    })
}

pub fn derive_deletion_map_impl(variants_with_content_type: &Vec<(Ident, Type)>) -> TokenStream {
    let entity_vec_idents = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let variant_str = variant.to_string();
        let entity_vec_ident : String = variant_str.chars().enumerate().flat_map(|(idx, c)| {
            if idx > 0 && c.is_uppercase() {
                vec!['_', c.to_lowercase().next().expect("Making Ident Failed")]
            }
            else {
                vec![c.to_lowercase().next().expect("Making Ident Failed")]
            }
        }).chain("s".chars()).collect();

        let vec_ident = Ident::new(&format!("{}", entity_vec_ident), variant.span());
        (variant_str, vec_ident)
    }).collect::<HashMap<_, _>>();

    let entity_vec_declarations = 
        variants_with_content_type.iter().enumerate().map(|(_i, (variant, content_type))| {
  
        let vec_ident = entity_vec_idents.get(&variant.to_string()).expect("No vec ident found");
        quote! {
            pub #vec_ident : Vec<Uuid>
        }
    });

    let entity_initializers = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).expect("No vec ident found");
        quote! {
            #vec_ident: Vec::new()
        }
    });

    let entity_type_add_map = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).expect("No vec ident found");
        quote! {
            EntityTypeId::#variant => {
                self.#vec_ident.push(uuid);
            }
        }
    });
    
    let delete_statements = variants_with_content_type.iter().map(|(variant, _content_type)| {
        let vec_ident = entity_vec_idents.get(&variant.to_string()).expect("No vec ident found");
        quote! {
            if !self.#vec_ident.is_empty() {
                <#variant as crate::domain::entity::BoundTournamentEntityTrait<C>>::delete_many(db, self.#vec_ident.clone()).await?;
            }
        }
    });

    TokenStream::from(quote!{
        #[derive(Debug)]
        pub struct EntityDeletionGroup {
            #(#entity_vec_declarations),*
        }

        #[async_trait::async_trait]
        impl EntityDeletionGroupTrait<EntityTypeId> for EntityDeletionGroup {
            fn new() -> Self {
                Self {
                    #(#entity_initializers),*
                }
            }

            fn add(&mut self, entity_type: EntityTypeId, uuid: Uuid) {
                match entity_type {
                    #(#entity_type_add_map)*
                }
            }

            async fn execute<C>(&self, db: &C) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
                #(#delete_statements)*
                Ok(())
            }
        }
    })
}


pub fn derive_entity_impl(input: &DeriveInput, variants_with_content_type: &Vec<(Ident, Type)>, group_ident: Ident) -> TokenStream {
    let entity_ident = &input.ident;

    let get_proc_order_arms = variants_with_content_type.iter().enumerate().map(|(idx, (variant, _content_type))| {
        let idx_lit = syn::LitInt::new(&idx.to_string(), entity_ident.span());
        quote! {
            #entity_ident::#variant(_) => #idx_lit,
        }
    });

    let get_proc_order_fn = quote! {
        fn get_processing_order(&self) -> u64 {
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
        fn get_name(&self) -> String {
            match self {
                #(#get_name_arms)*
            }
        }
    };

    let get_type_arms = variants_with_content_type.iter().map(|(variant, _content_type)| {
        quote! {
            #entity_ident::#variant(_) => EntityTypeId::#variant,
        }
    });

    let get_type_fn = quote! {
        fn get_type(&self) -> EntityTypeId {
            match self {
                #(#get_type_arms)*
            }
        }
    };

    let get_uuid_arms = variants_with_content_type.iter().map(|(variant, _content_type)| {
        quote! {
            #entity_ident::#variant(e) =>  e.uuid,
        }
    });

    let get_uuid_fn = quote! {
        fn get_uuid(&self) -> Uuid {
            match self {
                #(#get_uuid_arms)*
            }
        }
    };

    let get_related_uuids_arms = variants_with_content_type.iter().map(|(variant, content_type)| {
        quote! {
            #entity_ident::#variant(e) => <#content_type as crate::domain::entity::TournamentEntityTrait>::get_related_uuids(e),
        }
    });

    let get_related_uuids_fn = quote! {
        fn get_related_uuids(&self) -> Vec<Uuid> {
            match self {
                #(#get_related_uuids_arms)*
            }
        }
    };

    let expanded = quote! {
        impl EntityGroupEntityTrait<EntityTypeId> for #entity_ident {
            #get_proc_order_fn
            #get_name_fn
            #get_type_fn
            #get_uuid_fn
            #get_related_uuids_fn
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