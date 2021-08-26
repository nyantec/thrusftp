use proc_macro::{self, TokenStream};
use quote::{quote, format_ident};
use syn::{parse_macro_input, DeriveInput, Data, Fields};

fn destructure_fields(fields: &Fields) -> proc_macro2::TokenStream {
    match fields {
        Fields::Named(ref named_fields) => {
            let field = named_fields.named.iter().map(|f| &f.ident);
            quote! { { #( #field ),* } }
        },
        Fields::Unnamed(ref unnamed_fields) => {
            let field = (0..unnamed_fields.unnamed.len()).map(|x| format_ident!("field{}", x));
            quote! { ( #( #field ),* ) }
        },
        Fields::Unit => {
            quote! {}
        }
    }
}

fn serialize_fields(fields: &Fields) -> proc_macro2::TokenStream {
    match fields {
        Fields::Named(ref named_fields) => {
            let field = named_fields.named.iter().map(|f| &f.ident);
            quote! {
                #( res.append(&mut Serialize::serialize(#field)?); )*
            }
        },
        Fields::Unnamed(ref unnamed_fields) => {
            let field = (0..unnamed_fields.unnamed.len()).map(|x| format_ident!("field{}", x));
            quote! {
                #( res.append(&mut Serialize::serialize(#field)?); )*
            }
        },
        Fields::Unit => {
            quote! {}
        }
    }
}

#[proc_macro_derive(Serialize)]
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
            let repr: syn::Expr = attrs.iter().filter(|x| x.path.is_ident("repr")).next().unwrap().parse_args().unwrap();
            let variant = enum_data.variants.iter().map(|v| {
                let variantname = &v.ident;
                let variantnum = &v.discriminant.as_ref().expect("need discriminant").1;
                let destructure = destructure_fields(&v.fields);
                let serialize = serialize_fields(&v.fields);
                quote! {
                    #ident::#variantname #destructure => {
                        res.append(&mut <#repr>::serialize(&#variantnum)?);
                        #serialize
                    }
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

#[proc_macro_derive(Deserialize)]
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
            let repr: syn::Expr = attrs.iter().filter(|x| x.path.is_ident("repr")).next().unwrap().parse_args().unwrap();
            let variant = enum_data.variants.iter().map(|v| {
                let variantname = &v.ident;
                let num = &v.discriminant.as_ref().expect("need discriminant").1;
                let f = deserialize_fields(&v.fields);
                quote! {
                    #num => {
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
