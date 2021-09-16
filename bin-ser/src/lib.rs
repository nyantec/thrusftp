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
                    #( Serialize::serialize(&self.#name, writer)?; )*
                }
            },
            Fields::Unnamed(ref unnamed_fields) => {
                let num = (0..unnamed_fields.unnamed.len()).map(syn::Index::from);
                quote! {
                    #( Serialize::serialize(&self.#num, writer)?; )*
                }
            },
            _ => unimplemented!(),
        },
        Data::Enum(ref enum_data) => {
            let repr = get_attr(&attrs, "repr").expect("need repr attr");
            let variant = enum_data.variants.iter().map(|v| {
                let variantname = &v.ident;
                let variantval = get_attr(&v.attrs, "val").expect("need val attr");
                match v.fields {
                    Fields::Named(ref named_fields) => {
                        let field = named_fields.named.iter().map(|f| &f.ident);
                        let serialize_fields = quote! {
                            #( Serialize::serialize(#field, writer)?; )*
                        };
                        let field = named_fields.named.iter().map(|f| &f.ident);
                        quote! {
                            #ident::#variantname { #( #field ),* } => {
                                <#repr>::serialize(&#variantval, writer)?;
                                #serialize_fields
                            }
                        }
                    },
                    Fields::Unit => {
                        quote! {
                            #ident::#variantname => {
                                <#repr>::serialize(&#variantval, writer)?;
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
            fn serialize(&self, writer: &mut Write) -> anyhow::Result<()> {
                #content
                Ok(())
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
                let variantval = get_attr(&v.attrs, "val").expect("need val attr");
                let f = deserialize_fields(&v.fields);
                quote! {
                    #variantval => {
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
