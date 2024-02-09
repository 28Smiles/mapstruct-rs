use proc_macro2::{self, TokenStream};
use quote::quote;
use syn::{DeriveInput, parse2};
use crate::r#enum::MapEnum;

use crate::r#struct::MapStruct;

mod r#struct;
mod r#enum;
mod named_field_change;
mod generic;
mod variant;
mod transformer;
mod struct_change;
mod enum_change;
mod tuple_change;
mod unnamed_field_change;

#[macro_export]
macro_rules! unwrap_one_variant {
    ($expression:expr, $pattern:pat $(if $guard:expr)?, $extract:expr $(,)?) => {
        match $expression {
            $pattern $(if $guard)? => $extract,
            _ => unreachable!(),
        }
    };
}

pub fn derive(input: TokenStream) -> TokenStream {
    match syn_derive(input) {
        Ok(input) => quote! {
            #(#input)*
        },
        Err(err) => err.to_compile_error(),
    }
}

fn syn_derive(input: TokenStream) -> syn::Result<Vec<DeriveInput>> {
    let input = parse2::<DeriveInput>(input)?;
    let attrs = input.attrs
        .iter()
        .filter(|attr| attr.path().is_ident("mapstruct"))
        .map(|attr| &attr.meta)
        .map(|meta| match meta {
            syn::Meta::List(list) => Ok(list.tokens.clone()),
            _ => Err(syn::Error::new_spanned(meta, "expected #[mapstruct(...)]"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Check if the input is a struct or an enum
    match input.data {
        syn::Data::Struct(_) => {
            attrs.into_iter()
                .map(|tokens| parse2::<MapStruct>(tokens))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|mapstruct| mapstruct.transform(input.clone()))
                .collect()
        },
        syn::Data::Enum(_) => {
            attrs.into_iter()
                .map(|tokens| parse2::<MapEnum>(tokens))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|mapenum| mapenum.transform(input.clone()))
                .collect()
        },
        _ => return Err(syn::Error::new_spanned(input, "expected struct or enum")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_struct() {
        let input = quote! {
            #[mapstruct(
                #[derive(Debug)]
                struct Y<
                    +'a,
                    +T,
                > {
                    ~id -> x_id,
                    ~name: &'a str,
                    ~some: &'a str,
                    +last_name: &'a str,
                    -height,
                    +t: T,
                }
            )]
            struct X {
                id: i64,
                name: String,
                age: i32,
                height: f32,
                some: String,
            }
        };
        let expected = quote! {
            #[derive(Debug)]
            struct Y<'a, T> {
                x_id: i64,
                name: &'a str,
                age: i32,
                some: &'a str,
                last_name: &'a str,
                t: T
            }
        };
        assert_eq!(expected.to_string(), derive(input).to_string());
    }

    #[test]
    fn test_derive_enum_tuple() {
        let input = quote! {
            #[mapstruct(
                #[derive(Debug)]
                enum Y {
                    -A,
                    +D(i8),
                }
            )]
            enum X {
                A(i64),
                B(i32),
                C(i16),
            }
        };
        let expected = quote! {
            #[derive(Debug)]
            enum Y {
                B(i32),
                C(i16),
                D(i8)
            }
        };
        assert_eq!(expected.to_string(), derive(input).to_string());
    }

    #[test]
    fn test_derive_enum_struct() {
        let input = quote! {
            #[mapstruct(
                #[derive(Debug)]
                enum Y {
                    -A,
                    ~B {
                        -name,
                    },
                    ~C {
                        -name,
                    },
                    +D {
                        id: i8,
                    },
                }
            )]
            enum X {
                A {
                    id: i64,
                    name: String,
                },
                B {
                    id: i32,
                    name: String,
                },
                C {
                    id: i16,
                    name: String,
                },
            }
        };
        let expected = quote! {
            #[derive(Debug)]
            enum Y {
                B {
                    id: i32
                },
                C {
                    id: i16
                },
                D {
                    id: i8,
                }
            }
        };
        assert_eq!(expected.to_string(), derive(input).to_string());
    }

    #[test]
    fn test_derive_enum_struct_replace() {
        let input = quote! {
            #[mapstruct(
                #[derive(Debug)]
                enum Y {
                    ~B {
                        -name,
                    },
                    ~C {
                        -name,
                    },
                    ~A -> D {
                        ~id: i8,
                        -name,
                    },
                }
            )]
            enum X {
                A {
                    id: i64,
                    name: String,
                },
                B {
                    id: i32,
                    name: String,
                },
                C {
                    id: i16,
                    name: String,
                },
            }
        };
        let expected = quote! {
            #[derive(Debug)]
            enum Y {
                D {
                    id: i8
                },
                B {
                    id: i32
                },
                C {
                    id: i16
                }
            }
        };
        assert_eq!(expected.to_string(), derive(input).to_string());
    }

    #[test]
    fn test_derive_enum_tuple_replace() {
        let input = quote! {
            #[mapstruct(
                #[derive(Debug)]
                pub enum Y {
                    A(i64),
                    B(i32),
                    C(i16),
                    +D(i8),
                }
            )]
            enum X {
                A {
                    id: i64,
                    name: String,
                },
                B {
                    id: i32,
                    name: String,
                },
                C {
                    id: i16,
                    name: String,
                },
            }
        };
        let expected = quote! {
            #[derive(Debug)]
            pub enum Y {
                A(i64),
                B(i32),
                C(i16),
                D(i8)
            }
        };
        assert_eq!(expected.to_string(), derive(input).to_string());
    }

    #[test]
    fn test_derive_enum_tuple_change() {
        let input = quote! {
            #[mapstruct(
                #[derive(Debug)]
                pub enum Y {
                    ~A(_, _, _, +i64),
                    ~B(_, _, ~i128),
                    ~C(_, _, ~u16, +u8),
                    +E(i8, i16, i32, i64),
                }
            )]
            enum X {
                A(i8, i16, i32),
                B(i32, i64, i16),
                C(i16, i8, i32),
                D(i8, i16, i32, i64),
            }
        };
        let expected = quote! {
            #[derive(Debug)]
            pub enum Y {
                A(i8, i16, i32, i64),
                B(i32, i64, i128),
                C(i16, i8, u16, u8),
                D(i8, i16, i32, i64),
                E(i8, i16, i32, i64)
            }
        };
        assert_eq!(expected.to_string(), derive(input).to_string());
    }
}
