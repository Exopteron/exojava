use proc_macro::{TokenStream};
use proc_macro2::Span;
use quote::quote;
use syn::Ident;

#[proc_macro]
pub fn identifierification(v: TokenStream) -> TokenStream {
    let i = Ident::new(&v.to_string(), Span::call_site());
    quote! {
        #i
    }.into()
}