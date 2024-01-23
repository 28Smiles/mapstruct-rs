use proc_macro::TokenStream;

#[proc_macro_derive(MapStruct, attributes(mapstruct))]
pub fn derive(input: TokenStream) -> TokenStream {
    mapstruct_derive_lib::derive(input.into()).into()
}
