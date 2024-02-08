use syn::{DeriveInput, GenericParam};
use syn::parse::Parse;

use crate::enum_change::EnumChange;
use crate::generic::GenericChange;
use crate::transformer::Transformer;

pub struct MapEnum {
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    ident: syn::Ident,
    generics: Vec<GenericChange>,
    changes: EnumChange,
}

impl Parse for MapEnum {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = input.call(syn::Attribute::parse_outer)?;
        let vis = input.parse::<syn::Visibility>()?;
        input.parse::<syn::Token![enum]>()?;
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

        let changes = input.parse()?;

        Ok(MapEnum {
            attrs,
            vis,
            ident,
            generics,
            changes,
        })
    }
}

impl MapEnum {
    pub fn transform(self, input: DeriveInput) -> syn::Result<DeriveInput> {
        let mut input = input;
        input.attrs = self.attrs;
        input.vis = self.vis;
        input.ident = self.ident;

        let generic_removes = self.generics.iter()
            .filter(|change| matches!(change, GenericChange::Remove(_)))
            .map(|change| crate::unwrap_one_variant!(change, GenericChange::Remove(lifetime), lifetime))
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
            .map(|change| crate::unwrap_one_variant!(change, GenericChange::Add(param), param))
        );

        match &mut input.data {
            syn::Data::Enum(data) => {
                self.changes.transform(data)?;
            },
            _ => return Err(syn::Error::new_spanned(input, "expected enum")),
        }

        Ok(input)
    }
}
