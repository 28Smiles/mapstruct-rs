use syn::parse::Parse;

pub enum GenericChange {
    Add(syn::GenericParam),
    Remove(syn::Ident),
}

impl Parse for GenericChange {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::Token![+]) {
            input.parse::<syn::Token![+]>()?;
            let generic_param = input.parse()?;
            return Ok(GenericChange::Add(generic_param));
        }
        if input.peek(syn::Token![-]) {
            input.parse::<syn::Token![-]>()?;
            let lifetime = input.parse()?;
            return Ok(GenericChange::Remove(lifetime));
        }

        Err(input.error("expected one of +, -"))
    }
}
