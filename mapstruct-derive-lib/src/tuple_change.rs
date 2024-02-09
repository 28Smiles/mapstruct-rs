use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use crate::transformer::Transformer;
use crate::unnamed_field_change::UnnamedFieldChange;

pub struct TupleChange {
    changes: Vec<UnnamedFieldChange>,
}

impl Parse for TupleChange {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);
        let changes = content.parse_terminated(
            UnnamedFieldChange::parse,
            syn::Token![,]
        )?;
        let changes = changes.into_iter().collect();
        Ok(TupleChange { changes })
    }
}

impl Transformer for TupleChange {
    type Item = syn::FieldsUnnamed;
    type CreateIter = std::option::IntoIter<Self::Item>;

    fn create(&self) -> syn::Result<Self::CreateIter> {
        Ok(None.into_iter())
    }

    fn transform(&self, item: &mut Self::Item) -> syn::Result<bool> {
        let mut new_fields = Vec::new();
        let mut old_fields = item.unnamed.iter();
        let mut changes = self.changes.iter();
        while let Some(change) = changes.next() {
            match change {
                UnnamedFieldChange::Add { visibility, ty } => {
                    new_fields.push(syn::Field {
                        attrs: Vec::new(),
                        vis: visibility.clone(),
                        ty: ty.clone(),
                        ident: None,
                        mutability: syn::FieldMutability::None,
                        colon_token: None,
                    });
                }
                UnnamedFieldChange::Remove { ty } => {
                    if let Some(ty) = ty {
                        if let Some(field) = old_fields.next() {
                            if field.ty.to_token_stream().to_string() != ty.to_token_stream().to_string() {
                                return Err(syn::Error::new_spanned(
                                    field,
                                    "Expected field to be removed but type did not match",
                                ));
                            } else {
                                continue;
                            }
                        }
                    }

                    return Err(syn::Error::new_spanned(
                        item,
                        "Expected field to be removed but there are no more fields",
                    ));
                }
                UnnamedFieldChange::Retype { old_type, new_type } => {
                    if let Some(old_type) = old_type {
                        if let Some(field) = old_fields.next() {
                            if field.ty.to_token_stream().to_string() != old_type.to_token_stream().to_string() {
                                return Err(syn::Error::new_spanned(
                                    field,
                                    "Expected field to be retyped but original type did not match",
                                ));
                            } else {
                                new_fields.push(syn::Field {
                                    ty: new_type.clone(),
                                    ..field.clone()
                                });
                                continue;
                            }
                        }
                    } else {
                        if let Some(field) = old_fields.next() {
                            new_fields.push(syn::Field {
                                ty: new_type.clone(),
                                ..field.clone()
                            });
                            continue;
                        }
                    }

                    return Err(syn::Error::new_spanned(
                        item,
                        "Expected field to be retyped but there are no more fields",
                    ));
                }
                UnnamedFieldChange::Match { ty } => {
                    if let Some(field) = old_fields.next() {
                        if let Some(ty) = ty {
                            if field.ty.to_token_stream().to_string() != ty.to_token_stream().to_string() {
                                return Err(syn::Error::new_spanned(
                                    field,
                                    "Expected field to match but type did not match",
                                ));
                            } else {
                                new_fields.push(field.clone());
                                continue;
                            }
                        } else {
                            new_fields.push(field.clone());
                            continue;
                        }
                    }

                    return Err(syn::Error::new_spanned(
                        item,
                        "Expected field to match but there are no more fields",
                    ));
                }
            }
        }

        if let Some(field) = old_fields.next() {
            return Err(syn::Error::new_spanned(
                field,
                "Expected no more fields but there are more fields",
            ));
        }

        item.unnamed = syn::punctuated::Punctuated::from_iter(new_fields);

        Ok(true)
    }

    fn remove(&self, _: &Self::Item) -> syn::Result<bool> {
        Ok(false)
    }
}