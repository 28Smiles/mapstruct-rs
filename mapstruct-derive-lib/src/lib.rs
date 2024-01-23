use proc_macro2::{self, TokenStream};
use quote::quote;
use syn::{DeriveInput, parse2, braced, GenericParam, Data, FieldMutability};
use syn::parse::Parse;
use syn::punctuated::Punctuated;

macro_rules! unwrap_one_variant {
    ($expression:expr, $pattern:pat $(if $guard:expr)?, $extract:expr $(,)?) => {
        match $expression {
            $pattern $(if $guard)? => $extract,
            _ => unreachable!(),
        }
    };
}

enum FieldChange {
    Add {
        visibility: syn::Visibility,
        name: syn::Ident,
        ty: syn::Type,
    },
    Remove {
        name: syn::Ident,
    },
    Rename {
        from: syn::Ident,
        visibility: syn::Visibility,
        to: syn::Ident,
    },
    Retype {
        name: syn::Ident,
        ty: syn::Type,
    },
    RenameAndRetype {
        from: syn::Ident,
        visibility: syn::Visibility,
        to: syn::Ident,
        ty: syn::Type,
    },
}

impl Parse for FieldChange {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::Token![+]) {
            input.parse::<syn::Token![+]>()?;
            let visibility = input.parse()?;
            let name = input.parse()?;
            input.parse::<syn::Token![:]>()?;
            let ty = input.parse()?;

            return Ok(FieldChange::Add {
                visibility,
                name,
                ty,
            })
        }
        if input.peek(syn::Token![-]) {
            input.parse::<syn::Token![-]>()?;
            let name = input.parse()?;
            return Ok(FieldChange::Remove {
                name,
            })
        }
        if input.peek(syn::Token![~]) {
            input.parse::<syn::Token![~]>()?;
            let from = input.parse()?;
            return match input.peek(syn::Token![->]) {
                true => {
                    input.parse::<syn::Token![->]>()?;
                    let visibility = input.parse()?;
                    let to = input.parse()?;

                    match input.peek(syn::Token![:]) {
                        true => {
                            input.parse::<syn::Token![:]>()?;
                            let ty = input.parse()?;
                            Ok(FieldChange::RenameAndRetype {
                                from,
                                visibility,
                                to,
                                ty,
                            })
                        },
                        false => Ok(FieldChange::Rename {
                            from,
                            visibility,
                            to,
                        }),
                    }
                },
                false => {
                    input.parse::<syn::Token![:]>()?;
                    let ty = input.parse()?;
                    Ok(FieldChange::Retype {
                        name: from,
                        ty,
                    })
                },
            }
        }

        Err(input.error("expected one of +, -, ~"))
    }
}


enum GenericChange {
    Add(syn::GenericParam),
    Remove(syn::Ident),
}

impl Parse for GenericChange {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::Token![+]) {
            input.parse::<syn::Token![+]>()?;
            let generic_param = input.parse()?;
            return Ok(GenericChange::Add(generic_param));
        }
        if input.peek(syn::Token![-]) {
            input.parse::<syn::Token![-]>()?;
            let lifetime = input.parse()?;
            return Ok(GenericChange::Remove(lifetime));
        }

        Err(input.error("expected one of +, -"))
    }
}


struct MapStruct {
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    ident: syn::Ident,
    generics: Vec<GenericChange>,
    changes: Vec<FieldChange>,
}

impl Parse for MapStruct {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = input.call(syn::Attribute::parse_outer)?;
        let vis = input.parse::<syn::Visibility>()?;
        input.parse::<syn::Token![struct]>()?;
        let ident = input.parse()?;

        let generics = if input.peek(syn::Token![<]) {
            input.parse::<syn::Token![<]>()?;
            let mut lifetimes = Vec::new();
            while !input.peek(syn::Token![>]) {
                lifetimes.push(input.parse()?);
                if !input.peek(syn::Token![,]) || input.peek(syn::Token![>]) {
                    break;
                }
                input.parse::<syn::Token![,]>()?;
            }
            input.parse::<syn::Token![>]>()?;
            lifetimes
        } else {
            Vec::new()
        };

        let content;
        braced!(content in input);

        let changes = Punctuated::<FieldChange, syn::Token![,]>::parse_terminated(&content)?
            .into_iter()
            .collect();

        Ok(MapStruct {
            attrs,
            vis,
            ident,
            generics,
            changes,
        })
    }
}

impl MapStruct {
    fn transform(self, input: DeriveInput) -> syn::Result<DeriveInput> {
        let mut input = input;
        input.attrs = self.attrs;
        input.vis = self.vis;
        input.ident = self.ident;
        let generic_removes = self.generics.iter()
            .filter(|change| match change {
                GenericChange::Add(_) => false,
                GenericChange::Remove(_) => true,
            })
            .map(|change| match change {
                GenericChange::Add(_) => unreachable!(),
                GenericChange::Remove(ident) => ident,
            })
            .collect::<Vec<_>>();
        input.generics.params = input.generics.params.into_iter()
            .filter(|param| match param {
                GenericParam::Lifetime(param) => {
                    !generic_removes.contains(&&param.lifetime.ident)
                },
                GenericParam::Type(param) => {
                    !generic_removes.contains(&&param.ident)
                },
                GenericParam::Const(param) => {
                    !generic_removes.contains(&&param.ident)
                },
            })
            .collect();
        input.generics.params.extend(self.generics.into_iter()
            .filter(|change| matches!(change, GenericChange::Add(_)))
            .map(|change| unwrap_one_variant!(change, GenericChange::Add(param), param))
        );

        match &mut input.data {
            Data::Struct(data) => {
                let fields_named = match &mut data.fields {
                    syn::Fields::Named(fields_named) => fields_named,
                    syn::Fields::Unnamed(_) => return Err(syn::Error::new_spanned(input, "tuple fields not supported"))?,
                    syn::Fields::Unit => return Err(syn::Error::new_spanned(input, "unit fields not supported"))?,
                };
                let mut added_fields = Vec::new();
                let mut removed_fields = Vec::new();
                let mut renamed_fields = Vec::new();
                let mut retype_fields = Vec::new();
                let mut rename_and_retype_fields = Vec::new();
                for change in self.changes {
                    match change {
                        FieldChange::Add { visibility, name, ty } => {
                            added_fields.push(syn::Field {
                                attrs: Vec::new(),
                                vis: visibility,
                                mutability: FieldMutability::None,
                                ident: Some(name),
                                colon_token: None,
                                ty,
                            });
                        },
                        FieldChange::Remove { name } => {
                            removed_fields.push(name);
                        },
                        FieldChange::Rename { from, visibility, to } => {
                            renamed_fields.push((from, visibility, to));
                        },
                        FieldChange::Retype { name, ty } => {
                            retype_fields.push((name, ty));
                        },
                        FieldChange::RenameAndRetype { from, visibility, to, ty } => {
                            rename_and_retype_fields.push((from, visibility, to, ty));
                        },
                    }
                }

                for field_name in removed_fields.iter() {
                    if fields_named.named.iter().find(|field| field.ident.as_ref().unwrap() == field_name).is_none() {
                        return Err(syn::Error::new_spanned(
                            field_name,
                            "field cannot be removed because it is not present in the original struct",
                        ))?;
                    }
                }
                for (from, _, _) in renamed_fields.iter() {
                    if fields_named.named.iter().find(|field| field.ident.as_ref().unwrap() == from).is_none() {
                        return Err(syn::Error::new_spanned(
                            from,
                            "field cannot be renamed because it is not present in the original struct",
                        ))?;
                    }
                }
                for (name, _) in retype_fields.iter() {
                    if fields_named.named.iter().find(|field| field.ident.as_ref().unwrap() == name).is_none() {
                        return Err(syn::Error::new_spanned(
                            name,
                            "field cannot be retyped because it is not present in the original struct",
                        ))?;
                    }
                }
                for (from, _, _, _) in rename_and_retype_fields.iter() {
                    if fields_named.named.iter().find(|field| field.ident.as_ref().unwrap() == from).is_none() {
                        return Err(syn::Error::new_spanned(
                            from,
                            "field cannot be renamed and retyped because it is not present in the original struct",
                        ))?;
                    }
                }

                let mut fields = fields_named.named.iter()
                    .cloned()
                    .filter(|field| {
                        !removed_fields.contains(&&field.ident.as_ref().unwrap())
                    })
                    .collect::<Vec<_>>();
                for field in fields.iter_mut() {
                    for (from, visibility, to) in renamed_fields.iter() {
                        if field.ident.as_ref().unwrap() == from {
                            field.ident = Some(to.clone());
                            field.vis = visibility.clone();
                        }
                    }
                    for (name, ty) in retype_fields.iter() {
                        if field.ident.as_ref().unwrap() == name {
                            field.ty = ty.clone();
                        }
                    }
                    for (from, visibility, to, ty) in rename_and_retype_fields.iter() {
                        if field.ident.as_ref().unwrap() == from {
                            field.ident = Some(to.clone());
                            field.vis = visibility.clone();
                            field.ty = ty.clone();
                        }
                    }
                }

                fields.extend(added_fields.into_iter());

                fields_named.named = Punctuated::from_iter(fields.into_iter());
            }
            Data::Enum(_) => return Err(syn::Error::new_spanned(input, "enum not supported"))?,
            Data::Union(_) => return Err(syn::Error::new_spanned(input, "union not supported"))?,
        }

        Ok(input)
    }
}

pub fn derive(input: TokenStream) -> TokenStream {
    let input = match parse2::<DeriveInput>(input) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error(),
    };
    match input.attrs
        .iter()
        .filter(|attr| attr.path().is_ident("mapstruct"))
        .map(|attr| &attr.meta)
        .map(|meta| match meta {
            syn::Meta::List(list) => Ok(list.tokens.clone()),
            _ => Err(syn::Error::new_spanned(meta, "expected #[mapstruct(...)]"))
        })
        .collect::<Result<Vec<_>, _>>() {
        Ok(tokens) => match tokens.into_iter()
            .map(|tokens| parse2::<MapStruct>(tokens))
            .collect::<Result<Vec<_>, _>>() {
            Ok(mapstructs) => mapstructs.into_iter()
                .map(|mapstruct| mapstruct.transform(input.clone()))
                .map(|result| match result {
                    Ok(input) => quote! { #input },
                    Err(err) => err.to_compile_error(),
                })
                .collect(),
            Err(err) => err.to_compile_error(),
        },
        Err(err) => return err.to_compile_error(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive() {
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
}
