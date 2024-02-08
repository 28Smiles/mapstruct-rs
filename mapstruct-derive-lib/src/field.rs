use syn::FieldMutability;
use syn::parse::Parse;
use crate::transformer::Transformer;

pub enum FieldChange {
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

impl Transformer for FieldChange {
    type Item = syn::Field;
    type CreateIter = std::option::IntoIter<Self::Item>;

    fn create(&self) -> syn::Result<Self::CreateIter> {
        match self {
            FieldChange::Add { visibility, name, ty } => {
                let field = syn::Field {
                    attrs: Vec::new(),
                    vis: visibility.clone(),
                    mutability: FieldMutability::None,
                    ident: Some(name.clone()),
                    colon_token: None,
                    ty: ty.clone(),
                };

                Ok(Some(field).into_iter())
            },
            _ => Ok(None.into_iter()),
        }
    }

    fn remove(&self, field: &Self::Item) -> syn::Result<bool> {
        match self {
            FieldChange::Remove { name } => {
                if field.ident.as_ref().unwrap() == name {
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
            _ => Ok(false),
        }
    }

    fn transform(&self, field: &mut Self::Item) -> syn::Result<bool> {
        match self {
            FieldChange::Rename { from, visibility, to } => {
                if field.ident.as_ref().unwrap() == from {
                    field.ident = Some(to.clone());
                    field.vis = visibility.clone();
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
            FieldChange::Retype { name, ty } => {
                if field.ident.as_ref().unwrap() == name {
                    field.ty = ty.clone();
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
            FieldChange::RenameAndRetype { from, visibility, to, ty } => {
                if field.ident.as_ref().unwrap() == from {
                    field.ident = Some(to.clone());
                    field.vis = visibility.clone();
                    field.ty = ty.clone();
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
            _ => Ok(false),
        }
    }
}