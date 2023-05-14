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