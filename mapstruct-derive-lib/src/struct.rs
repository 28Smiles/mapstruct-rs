use syn::{Data, DeriveInput, GenericParam};
use syn::parse::Parse;

use crate::generic::GenericChange;
use crate::struct_change::StructChange;
use crate::transformer::Transformer;

pub struct MapStruct {
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    ident: syn::Ident,
    generics: Vec<GenericChange>,
    changes: StructChange,
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

        let changes = input.parse()?;

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
    pub(crate) fn transform(self, input: DeriveInput) -> syn::Result<DeriveInput> {
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
            Data::Struct(data) => {
                let fields_named = match &mut data.fields {
                    syn::Fields::Named(fields_named) => fields_named,
                    syn::Fields::Unnamed(_) => return Err(syn::Error::new_spanned(input, "tuple fields not supported"))?,
                    syn::Fields::Unit => return Err(syn::Error::new_spanned(input, "unit fields not supported"))?,
                };

                self.changes.transform(fields_named)?;
            }
            _ => return Err(syn::Error::new_spanned(input, "only structs are supported"))?,
        }

        Ok(input)
    }
}
