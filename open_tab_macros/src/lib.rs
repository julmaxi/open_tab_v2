mod utilities;
mod simple_entity;
mod entity_group;

extern crate proc_macro;

use proc_macro::TokenStream;

use simple_entity::simple_entity_derive_impl;
use entity_group::entity_group_derive_impl;


#[proc_macro_derive(SimpleEntity, attributes(module_path, get_many_tournaments_func, tournament_id, serialize, skip_field))]
pub fn simple_entity_derive(input: TokenStream) -> TokenStream {
    simple_entity_derive_impl(input)
}

#[proc_macro_derive(EntityCollection)]
pub fn entity_group_derive(input: TokenStream) -> TokenStream {
    entity_group_derive_impl(input)
}