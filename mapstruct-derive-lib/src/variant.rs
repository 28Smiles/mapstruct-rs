use syn::FieldMutability;
use syn::parse::{Parse, ParseStream};

use crate::struct_change::StructChange;
use crate::transformer::Transformer;

pub struct TupleChange {
    types: Vec<syn::Type>,
}

impl Parse for TupleChange {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);
        let types = content.parse_terminated(syn::Type::parse, syn::Token![,])?;
        let types = types.into_iter().collect();
        Ok(TupleChange { types })
    }
}

impl Transformer for TupleChange {
    type Item = syn::FieldsUnnamed;
    type CreateIter = std::option::IntoIter<Self::Item>;

    fn create(&self) -> syn::Result<Self::CreateIter> {
        Ok(None.into_iter())
    }

    fn transform(&self, item: &mut Self::Item) -> syn::Result<bool> {
        item.unnamed.clear();
        item.unnamed.extend(self.types.iter().cloned().map(|ty| syn::Field {
            attrs: Vec::new(),
            vis: syn::Visibility::Inherited,
            mutability: FieldMutability::None,
            ident: None,
            colon_token: None,
            ty,
        }));

        Ok(true)
    }

    fn remove(&self, _: &Self::Item) -> syn::Result<bool> {
        Ok(true)
    }
}

pub enum VariantChange {
    Add(syn::Variant),
    Remove(syn::Ident),
    Rename(syn::Ident, syn::Ident),
    TupleRetype(syn::Ident, TupleChange),
    StructRetype(syn::Ident, StructChange),
    StructReplace(syn::Ident, syn::Ident, StructChange),
    TupleReplace(syn::Ident, syn::Ident, TupleChange),
}

impl Parse for VariantChange {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(syn::Token![+]) {
            input.parse::<syn::Token![+]>()?;
            let variant = input.parse()?;
            return Ok(VariantChange::Add(variant));
        }
        if input.peek(syn::Token![-]) {
            input.parse::<syn::Token![-]>()?;
            let ident = input.parse()?;
            return Ok(VariantChange::Remove(ident));
        }
        if input.peek(syn::Token![~]) {
            input.parse::<syn::Token![~]>()?;
            let from = input.parse()?;
            // Try to parse a rename
            let to = if let Ok(_) = input.parse::<syn::Token![->]>() {
                Some(input.parse()?)
            } else {
                None
            };
            // Try to parse a tuple retype
            if let Ok(types) = input.parse::<TupleChange>() {
                if let Some(to) = to {
                    return Ok(VariantChange::TupleReplace(from, to, types));
                } else {
                    return Ok(VariantChange::TupleRetype(from, types));
                }
            }
            // Try to parse a struct retype
            if let Ok(fields) = input.parse::<StructChange>() {
                if let Some(to) = to {
                    return Ok(VariantChange::StructReplace(from, to, fields));
                } else {
                    return Ok(VariantChange::StructRetype(from, fields));
                }
            }

            if let Some(to) = to {
                return Ok(VariantChange::Rename(from, to));
            }

            return Ok(VariantChange::Rename(from.clone(), from));
        }

        Err(input.error("expected one of +, -, ~"))
    }
}

impl Transformer for VariantChange {
    type Item = syn::Variant;
    type CreateIter = std::option::IntoIter<syn::Variant>;

    fn create(&self) -> syn::Result<Self::CreateIter> {
        match self {
            VariantChange::Add(variant) => Ok(Some(variant.clone()).into_iter()),
            _ => Ok(None.into_iter()),
        }
    }

    fn transform(&self, variant: &mut Self::Item) -> syn::Result<bool> {
        match self {
            VariantChange::Rename(from, to) if &variant.ident == from => {
                variant.ident = to.clone();

                Ok(true)
            }
            VariantChange::TupleRetype(ident, change) if &variant.ident == ident => {
                let mut fields = syn::FieldsUnnamed {
                    paren_token: syn::token::Paren::default(),
                    unnamed: Default::default(),
                };
                change.transform(&mut fields)?;
                variant.fields = syn::Fields::Unnamed(fields);

                Ok(true)
            }
            VariantChange::StructRetype(ident, change) if &variant.ident == ident => {
                match &mut variant.fields {
                    syn::Fields::Named(fields) => {
                        change.transform(fields)?;
                    }
                    _ => {
                        let mut fields = syn::FieldsNamed {
                            brace_token: syn::token::Brace::default(),
                            named: Default::default(),
                        };
                        change.transform(&mut fields)?;
                        variant.fields = syn::Fields::Named(fields);
                    }
                }

                Ok(true)
            }
            VariantChange::StructReplace(from, to, change) if &variant.ident == from => {
                variant.ident = to.clone();
                match &mut variant.fields {
                    syn::Fields::Named(fields) => {
                        change.transform(fields)?;
                    }
                    _ => {
                        let mut fields = syn::FieldsNamed {
                            brace_token: syn::token::Brace::default(),
                            named: Default::default(),
                        };
                        change.transform(&mut fields)?;
                        variant.fields = syn::Fields::Named(fields);
                    }
                }

                Ok(true)
            }
            VariantChange::TupleReplace(from, to, change) if &variant.ident == from => {
                variant.ident = to.clone();
                let mut fields = syn::FieldsUnnamed {
                    paren_token: syn::token::Paren::default(),
                    unnamed: Default::default(),
                };
                change.transform(&mut fields)?;
                variant.fields = syn::Fields::Unnamed(fields);

                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn remove(&self, variant: &Self::Item) -> syn::Result<bool> {
        match self {
            VariantChange::Remove(ident) => Ok(&variant.ident == ident),
            _ => Ok(false),
        }
    }
}