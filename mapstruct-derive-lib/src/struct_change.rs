use std::vec::IntoIter;

use syn::parse::{Parse, ParseStream};

use crate::named_field_change::NamedFieldChange;
use crate::transformer::Transformer;

pub struct StructChange {
    changes: Vec<NamedFieldChange>,
}

impl Parse for StructChange {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::braced!(content in input);
        let changes = content.parse_terminated(NamedFieldChange::parse, syn::Token![,])?;
        let changes = changes.into_iter().collect();
        Ok(StructChange { changes })
    }
}

impl Transformer for StructChange {
    type Item = syn::FieldsNamed;
    type CreateIter = IntoIter<syn::FieldsNamed>;

    fn create(&self) -> syn::Result<Self::CreateIter> {
        Ok(Vec::new().into_iter())
    }

    fn transform(&self, item: &mut Self::Item) -> syn::Result<bool> {
        #[derive(Copy, Clone, PartialEq)]
        enum FieldChange {
            Original,
            Added,
            Changed,
            Removed,
        }

        let mut new_fields = item.named.iter()
            .cloned()
            .map(|field| (field, FieldChange::Original))
            .collect::<Vec<_>>();

        for field_change in &self.changes {
            let mut applied = false;
            for variant in field_change.create()? {
                new_fields.push((variant, FieldChange::Added));
                applied = true;
            }

            for (field, change) in &mut new_fields {
                let do_remove = field_change.remove(field)?;
                if do_remove && change != &FieldChange::Original {
                    // Already changed
                    return Err(syn::Error::new_spanned(
                        field,
                        "Cannot change field twice"
                    ));
                }
                let transform = field_change.transform(field)?;
                if transform && change != &FieldChange::Original {
                    // Already changed
                    return Err(syn::Error::new_spanned(
                        field,
                        "Cannot change field twice"
                    ));
                }

                if transform {
                    if do_remove {
                        return Err(syn::Error::new_spanned(
                            field,
                            "Cannot change field twice"
                        ));
                    } else {
                        *change = FieldChange::Changed;
                        applied = true;
                    }
                } else if do_remove {
                    *change = FieldChange::Removed;
                    applied = true;
                }
            }

            if !applied {
                return Err(syn::Error::new(
                    field_change.span(),
                    "No changes applied"
                ));
            }
        }

        item.named = new_fields.into_iter()
            .filter_map(|field| match field {
                (field, FieldChange::Original) => Some(field),
                (field, FieldChange::Added) => Some(field),
                (field, FieldChange::Changed) => Some(field),
                (_, FieldChange::Removed) => None,
            })
            .collect();

        Ok(true)
    }

    fn remove(&self, _: &Self::Item) -> syn::Result<bool> {
        Ok(false)
    }
}
