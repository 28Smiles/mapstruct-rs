use syn::parse::{Parse, ParseStream};

pub enum UnnamedFieldChange {
    Add {
        visibility: syn::Visibility,
        ty: syn::Type,
    },
    Remove {
        ty: Option<syn::Type>,
    },
    Retype {
        old_type: Option<syn::Type>,
        new_type: syn::Type,
    },
    Match {
        ty: Option<syn::Type>,
    },
}

impl Parse for UnnamedFieldChange {
    fn parse(input: ParseStream) -> syn::Result<Self> {

        if input.peek(syn::Token![+]) {
            input.parse::<syn::Token![+]>()?;
            let visibility = input.parse()?;
            let ty = input.parse()?;

            return Ok(UnnamedFieldChange::Add {
                visibility,
                ty,
            });
        }
        if input.peek(syn::Token![-]) {
            input.parse::<syn::Token![-]>()?;
            if input.peek(syn::Token![_]) {
                input.parse::<syn::Token![_]>()?;
                return Ok(UnnamedFieldChange::Remove {
                    ty: None,
                });
            } else {
                let ty = input.parse()?;
                return Ok(UnnamedFieldChange::Remove {
                    ty: Some(ty),
                });
            }
        }
        if input.peek(syn::Token![~]) {
            input.parse::<syn::Token![~]>()?;
            return Ok(UnnamedFieldChange::Retype {
                old_type: None,
                new_type: input.parse()?,
            });
        }

        let from = if input.peek(syn::Token![_]) {
            input.parse::<syn::Token![_]>()?;
            None
        } else {
            Some(input.parse()?)
        };

        if input.peek(syn::Token![->]) {
            input.parse::<syn::Token![->]>()?;
            let new_type = input.parse()?;
            return Ok(UnnamedFieldChange::Retype {
                old_type: from,
                new_type,
            });
        }

        return Ok(UnnamedFieldChange::Match {
            ty: from,
        });
    }
}
