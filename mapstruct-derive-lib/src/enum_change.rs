use syn::parse::{Parse, ParseStream};

use crate::transformer::Transformer;
use crate::variant::VariantChange;

pub struct EnumChange {
    changes: Vec<VariantChange>,
}

impl Parse for EnumChange {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::braced!(content in input);
        let changes = content.parse_terminated(VariantChange::parse, syn::Token![,])?;
        let changes = changes.into_iter().collect();
        Ok(EnumChange { changes })
    }
}


impl Transformer for EnumChange {
    type Item = syn::DataEnum;
    type CreateIter = std::option::IntoIter<Self::Item>;

    fn create(&self) -> syn::Result<Self::CreateIter> {
        Ok(None.into_iter())
    }

    fn transform(&self, item: &mut Self::Item) -> syn::Result<bool> {
        #[derive(Copy, Clone, PartialEq)]
        enum VariantChange {
            Original,
            Added,
            Changed,
            Removed,
        }

        let mut new_variants = item.variants.iter()
            .cloned()
            .map(|field| (field, VariantChange::Original))
            .collect::<Vec<_>>();

        for variant_change in &self.changes {
            let mut applied = false;
            for variant in variant_change.create()? {
                new_variants.push((variant, VariantChange::Added));
                applied = true;
            }

            for (variant, change) in &mut new_variants {
                let do_remove = variant_change.remove(variant)?;
                if do_remove && change != &VariantChange::Original {
                    // Already changed
                    return Err(syn::Error::new_spanned(
                        variant,
                        "Cannot change field twice"
                    ));
                }
                let transform = variant_change.transform(variant)?;
                if transform && change != &VariantChange::Original {
                    // Already changed
                    return Err(syn::Error::new_spanned(
                        variant,
                        "Cannot change field twice"
                    ));
                }

                if transform {
                    if do_remove {
                        return Err(syn::Error::new_spanned(
                            variant,
                            "Cannot change field twice"
                        ));
                    } else {
                        *change = VariantChange::Changed;
                        applied = true;
                    }
                } else if do_remove {
                    *change = VariantChange::Removed;
                    applied = true;
                }
            }

            if !applied {
                return Err(syn::Error::new(
                    variant_change.span(),
                    "No variant matched the given change"
                ));
            }
        }

        item.variants = new_variants.into_iter()
            .filter_map(|field| match field {
                (field, VariantChange::Original) => Some(field),
                (field, VariantChange::Added) => Some(field),
                (field, VariantChange::Changed) => Some(field),
                (_, VariantChange::Removed) => None,
            })
            .collect();

        Ok(true)
    }

    fn remove(&self, _: &Self::Item) -> syn::Result<bool> {
        Ok(false)
    }
}
