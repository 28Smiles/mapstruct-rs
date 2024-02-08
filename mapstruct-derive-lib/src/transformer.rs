
pub trait Transformer {
    type Item;
    type CreateIter: Iterator<Item = Self::Item>;

    fn create(&self) -> syn::Result<Self::CreateIter>;

    fn transform(&self, item: &mut Self::Item) -> syn::Result<bool>;

    fn remove(&self, item: &Self::Item) -> syn::Result<bool>;
}
