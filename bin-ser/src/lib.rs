use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Expr, Attribute, ExprAssign, Path};

fn parse_attr(attr: &Attribute) -> (Path, Expr) {
    let e: ExprAssign = attr.parse_args().unwrap();
    let path = if let Expr::Path(path) = *e.left { path.path } else { panic!(); };
    (path, *e.right)
}

fn get_attr(attrs: &Vec<Attribute>, ident: &str) -> Option<Expr> {
    attrs.iter()
        .filter(|x| x.path.is_ident("bin_ser"))
        .map(parse_attr)
        .filter(|(path, _)| path.is_ident(ident))
        .map(|(_, lit)| lit)
        .next()
}

#[proc_macro_derive(Serialize, attributes(bin_ser))]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, attrs, .. } = parse_macro_input!(input);

    let content = match data {
        Data::Struct(ref struct_data) => match struct_data.fields {
            Fields::Named(ref named_fields) => {
                let name = named_fields.named.iter().map(|f| &f.ident);
                quote! {
                    #( res.append(&mut Serialize::serialize(&self.#name)?); )*
                }
            },
            Fields::Unnamed(ref unnamed_fields) => {
                let num = (0..unnamed_fields.unnamed.len()).map(syn::Index::from);
                quote! {
                    #( res.append(&mut Serialize::serialize(&self.#num)?); )*
                }
            },
            _ => unimplemented!(),
        },
        Data::Enum(ref enum_data) => {
            let repr = get_attr(&attrs, "repr").expect("need repr attr");
            let variant = enum_data.variants.iter().map(|v| {
                let variantname = &v.ident;
                let variantnum = get_attr(&v.attrs, "num").expect("need num attr");
                match v.fields {
                    Fields::Named(ref named_fields) => {
                        let field = named_fields.named.iter().map(|f| &f.ident);
                        let serialize_fields = quote! {
                            #( res.append(&mut Serialize::serialize(#field)?); )*
                        };
                        let field = named_fields.named.iter().map(|f| &f.ident);
                        quote! {
                            #ident::#variantname { #( #field ),* } => {
                                res.append(&mut <#repr>::serialize(&#variantnum)?);
                                #serialize_fields
                            }
                        }
                    },
                    Fields::Unit => {
                        quote! {
                            #ident::#variantname => {
                                res.append(&mut <#repr>::serialize(&#variantnum)?);
                            }
                        }
                    }
                    _ => unimplemented!(),
                }
            });
            quote! {
                match self {
                    #( #variant ),*
                }
            }
        },
        Data::Union(ref _union_data) => unimplemented!(),
    };

    let output = quote! {
        impl Serialize for #ident {
            fn serialize(&self) -> anyhow::Result<Vec<u8>> {
                let mut res = Vec::new();

                #content

                Ok(res)
            }
        }
    };
    output.into()
}

fn deserialize_fields(fields: &Fields) -> proc_macro2::TokenStream {
    match fields {
        Fields::Named(ref named_fields) => {
            let name = named_fields.named.iter().map(|f| &f.ident);
            quote! {
                {
                    #( #name: Deserialize::deserialize(input)? ),*
                }
            }
        },
        Fields::Unnamed(ref unnamed_fields) => {
            let name = unnamed_fields.unnamed.iter().map(|_| quote!(Deserialize::deserialize(input)?));
            quote! {
                (
                    #( #name ),*
                )
            }
        },
        Fields::Unit => {
            quote! {}
        }
    }
}

#[proc_macro_derive(Deserialize, attributes(bin_ser))]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, attrs, .. } = parse_macro_input!(input);

    let content = match data {
        Data::Struct(ref struct_data) => {
            let f = deserialize_fields(&struct_data.fields);
            quote! {
                Self #f
            }
        },
        Data::Enum(ref enum_data) => {
            let repr = get_attr(&attrs, "repr").expect("need repr attr");
            let variant = enum_data.variants.iter().map(|v| {
                let variantname = &v.ident;
                let variantnum = get_attr(&v.attrs, "num").expect("need num attr");
                let f = deserialize_fields(&v.fields);
                quote! {
                    #variantnum => {
                        #ident::#variantname #f
                    }
                }
            });
            quote! {
                match <#repr>::deserialize(input)? {
                    #( #variant ),*
                    _ => panic!("unknown enum variant"),
                }
            }
        },
        Data::Union(ref _union_data) => unimplemented!(),
    };

    let output = quote! {
        impl Deserialize for #ident {
            fn deserialize(input: &mut &[u8]) ->  anyhow::Result<Self> {
                Ok(#content)
            }
        }
    };
    output.into()
}
