use quote::ToTokens;
use syn::FieldMutability;
use syn::parse::Parse;

use crate::transformer::Transformer;

/// Represents a change to a named field in a struct.
pub enum NamedFieldChange {
    /// Add a new field to the struct.
    Add {
        /// The visibility of the new field. This is the `pub` or `pub(crate)` part of the field.
        visibility: syn::Visibility,
        /// The identifier of the new field.
        ident: syn::Ident,
        /// The type of the new field.
        ty: syn::Type,
    },
    /// Remove a field from the struct.
    Remove {
        /// The identifier of the field to remove.
        ident: syn::Ident,
        /// The type of the field to remove. This is optional.
        ty: Option<syn::Type>,
    },
    /// Change a field.
    Change {
        /// The visibility of the new field. This is the `pub` or `pub(crate)` part of the field.
        visibility: syn::Visibility,
        /// The identifier of the field to change.
        ident: syn::Ident,
        /// The new identifier of the field if it is being renamed.
        to: Option<syn::Ident>,
        /// The new type of the field if it is being retyped.
        ty: Option<syn::Type>,
    },
}

impl Parse for NamedFieldChange {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::Token![+]) {
            input.parse::<syn::Token![+]>()?;
            let visibility = input.parse()?;
            let name = input.parse()?;
            input.parse::<syn::Token![:]>()?;
            let ty = input.parse()?;

            return Ok(NamedFieldChange::Add {
                visibility,
                ident: name,
                ty,
            })
        }

        if input.peek(syn::Token![-]) {
            input.parse::<syn::Token![-]>()?;
            let name = input.parse()?;

            if input.peek(syn::Token![:]) {
                input.parse::<syn::Token![:]>()?;
                let ty = input.parse()?;
                return Ok(NamedFieldChange::Remove {
                    ident: name,
                    ty: Some(ty),
                })
            }

            return Ok(NamedFieldChange::Remove {
                ident: name,
                ty: None,
            })
        }

        if input.peek(syn::Token![~]) {
            input.parse::<syn::Token![~]>()?;
            let visibility = input.parse()?;
            let from = input.parse()?;

            if input.peek(syn::Token![:]) {
                input.parse::<syn::Token![:]>()?;
                let ty = input.parse()?;

                return Ok(NamedFieldChange::Change {
                    visibility,
                    ident: from,
                    to: None,
                    ty: Some(ty),
                })
            }

            if input.peek(syn::Token![->]) {
                input.parse::<syn::Token![->]>()?;

                if !matches!(visibility, syn::Visibility::Inherited) {
                    return Err(input.error("expected visibility to be provided after `->` and not after `~`"))
                }

                let visibility = input.parse()?;
                let to = input.parse()?;

                if input.peek(syn::Token![:]) {
                    input.parse::<syn::Token![:]>()?;
                    let ty = input.parse()?;

                    return Ok(NamedFieldChange::Change {
                        visibility,
                        ident: from,
                        to: Some(to),
                        ty: Some(ty),
                    })
                }

                return Ok(NamedFieldChange::Change {
                    visibility,
                    ident: from,
                    to: Some(to),
                    ty: None,
                })
            }

            return Ok(NamedFieldChange::Change {
                visibility,
                ident: from,
                to: None,
                ty: None,
            })
        }

        Err(input.error("expected one of +, -, ~"))
    }
}

impl Transformer for NamedFieldChange {
    type Item = syn::Field;
    type CreateIter = std::option::IntoIter<Self::Item>;

    fn create(&self) -> syn::Result<Self::CreateIter> {
        match self {
            NamedFieldChange::Add { visibility, ident, ty } => {
                let field = syn::Field {
                    attrs: Vec::new(),
                    vis: visibility.clone(),
                    mutability: FieldMutability::None,
                    ident: Some(ident.clone()),
                    colon_token: None,
                    ty: ty.clone(),
                };

                Ok(Some(field).into_iter())
            }
            _ => Ok(None.into_iter()),
        }
    }

    fn remove(&self, field: &Self::Item) -> syn::Result<bool> {
        match self {
            NamedFieldChange::Remove { ident, ty } => {
                if field.ident.as_ref().unwrap() == ident {
                    if let Some(ty) = ty {
                        if field.ty.to_token_stream().to_string() == ty.to_token_stream().to_string() {
                            return Ok(true)
                        }
                    } else {
                        return Ok(true)
                    }
                }

                Ok(false)
            },
            _ => Ok(false),
        }
    }

    fn transform(&self, field: &mut Self::Item) -> syn::Result<bool> {
        match self {
            NamedFieldChange::Change { visibility, ident, to, ty } if field.ident.as_ref().unwrap() == ident => {
                if let Some(to) = to {
                    field.ident = Some(to.clone());
                }

                if let Some(ty) = ty {
                    field.ty = ty.clone();
                }

                field.vis = visibility.clone();
                Ok(true)
            },
            _ => Ok(false),
        }
    }
}


impl NamedFieldChange {
    pub fn span(&self) -> proc_macro2::Span {
        use syn::spanned::Spanned;

        match self {
            NamedFieldChange::Add { visibility, ident: _, ty } => {
                visibility.span().join(ty.span()).unwrap_or_else(|| visibility.span())
            },
            NamedFieldChange::Remove { ident, ty } => {
                let mut span = ident.span();

                if let Some(ty) = ty {
                    span = span.join(ty.span()).unwrap_or_else(|| span);
                }

                span
            },
            NamedFieldChange::Change { visibility, ident, to, ty } => {
                let mut span = visibility.span().join(ident.span()).unwrap_or_else(|| visibility.span());

                if let Some(to) = to {
                    span = span.join(to.span()).unwrap_or_else(|| span);
                }

                if let Some(ty) = ty {
                    span = span.join(ty.span()).unwrap_or_else(|| span);
                }

                span
            },
        }
    }
}