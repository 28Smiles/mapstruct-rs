use syn::Fields;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;

use crate::struct_change::StructChange;
use crate::transformer::Transformer;
use crate::tuple_change::TupleChange;

/// A change instruction for a variant.
pub enum VariantChange {
    /// Add a new variant to the enum, indicated by a `+` prefix
    Add(syn::Variant),
    /// Remove a variant from the enum, indicated by a `-` prefix
    Remove(syn::Ident),
    /// Rename a variant, indicated by a `~` prefix
    Rename(syn::Ident, syn::Ident),
    /// Retype a tuple variant, indicated by a `~` prefix
    /// If followed by `->`, the identifier is renamed too.
    TupleRetype(syn::Ident, Option<syn::Ident>, TupleChange),
    /// Retype a struct variant, indicated by a `~` prefix
    /// If followed by `->`, the identifier is renamed too.
    StructRetype(syn::Ident, Option<syn::Ident>, StructChange),
    /// Replace a struct variant, either by providing a variant
    /// with the same name or a new name after `->`.
    Replace(Option<syn::Ident>, syn::Variant),
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
            let to = if input.peek(syn::Token![->]) {
                input.parse::<syn::Token![->]>()?;
                Some(input.parse()?)
            } else {
                None
            };
            if let Ok(types) = input.parse::<TupleChange>() {
                return Ok(VariantChange::TupleRetype(from, to, types));
            }
            if let Ok(fields) = input.parse::<StructChange>() {
                return Ok(VariantChange::StructRetype(from, to, fields));
            }
            if let Some(to) = to {
                return Ok(VariantChange::Rename(from, to));
            }

            return Err(input.error("expected `->` or a change instruction"))
        }

        // Try to parse a replacement
        if input.peek(syn::Ident) && input.peek2(syn::Token![->]) {
            let from = input.parse()?;
            input.parse::<syn::Token![->]>()?;
            let variant = input.parse()?;
            return Ok(VariantChange::Replace(Some(from), variant));
        }

        return Ok(VariantChange::Replace(None, input.parse()?));
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
            VariantChange::TupleRetype(from, to, change) if &variant.ident == from => {
                match &mut variant.fields {
                    Fields::Named(_) => {
                        Err(syn::Error::new_spanned(
                            variant,
                            "cannot retype a struct variant as a tuple variant",
                        ))
                    }
                    Fields::Unnamed(fields) => {
                        if let Some(to) = to {
                            variant.ident = to.clone();
                        }

                        change.transform(fields)
                    }
                    Fields::Unit => {
                        Err(syn::Error::new_spanned(
                            variant,
                            "cannot retype a unit variant as a tuple variant, maybe you want to replace it instead",
                        ))
                    }
                }
            }
            VariantChange::StructRetype(from, to, change) if &variant.ident == from => {
                match &mut variant.fields {
                    syn::Fields::Named(fields) => {
                        if let Some(to) = to {
                            variant.ident = to.clone();
                        }

                        change.transform(fields)
                    }
                    syn::Fields::Unnamed(_) => {
                        Err(syn::Error::new_spanned(
                            variant,
                            "cannot retype a tuple variant as a struct variant",
                        ))
                    }
                    syn::Fields::Unit => {
                        Err(syn::Error::new_spanned(
                            variant,
                            "cannot retype a unit variant as a struct variant, maybe you want to replace it instead",
                        ))
                    }
                }
            }
            VariantChange::Replace(Some(from), to) if &variant.ident == from => {
                *variant = to.clone();

                Ok(true)
            }
            VariantChange::Replace(None, to) if &variant.ident == &to.ident => {
                *variant = to.clone();

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

impl VariantChange {
    pub fn span(&self) -> proc_macro2::Span {
        match self {
            VariantChange::Add(variant) => variant.span(),
            VariantChange::Remove(ident) => ident.span(),
            VariantChange::Rename(from, _) => from.span(),
            VariantChange::TupleRetype(from, _, _) => from.span(),
            VariantChange::StructRetype(from, _, _) => from.span(),
            VariantChange::Replace(Some(from), _) => from.span(),
            VariantChange::Replace(None, variant) => variant.span(),
        }
    }
}
