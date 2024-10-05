use proc_macro::{Ident, TokenStream};
use quote::ToTokens;
use syn::Path;

pub fn find_path_name_value_attr(attrs: &Vec<syn::Attribute>, name: &str) -> Option<Path> {
    attrs.iter()
        .find_map(|attr| {
            if attr.path().is_ident(name) {
                match &attr.meta {
                    syn::Meta::NameValue(val) => {
                        let path = match &val.value {
                            syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(e), ..}) => e,
                            _ => panic!("{} attribute must be a string", name)
                        };
                        let path : Result<Path, _> = path.parse();

                        path.ok()
                    },
                    _ => panic!("Only name value attributes are supported")
                }
            } else {
                None
            }
        })
}


pub fn find_skip_values(attrs: &Vec<syn::Attribute>) -> Vec<proc_macro2::Ident> {
    attrs.iter()
        .filter_map(|attr| {
            if attr.path().is_ident("skip_field") {
                match &attr.meta {
                    syn::Meta::NameValue(val) => {
                        let field_name = match &val.value {
                            syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(e), ..}) => e,
                            _ => panic!("skip_field attribute must be a string")
                        };

                        let val: String = field_name.value();
                        let val = proc_macro2::Ident::new(&val, field_name.span());
                        Some(val)
                    },
                    _ => panic!("Only name value attributes are supported")
                }
            } else {
                None
            }
        }).collect()
}