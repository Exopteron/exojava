use proc_macro2::{TokenStream, Ident};
use quote::{quote, TokenStreamExt};
use syn::{Block, parse::{Parse, ParseStream, ParseBuffer}, token::Paren, ExprMatch, Pat, Token, punctuated::Punctuated, braced, parse_macro_input, Type};
mod parser_macros;

struct SwitchCase {
    ident: Type,
    pat: Pat,
    block: Block
}

impl Parse for SwitchCase {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<Type>()?;

        let content;
        syn::parenthesized!(content in input);

        let pattern = content.parse::<Pat>()?;

        let fat_arrow: Token!(=>) = input.parse()?;

        let block = input.parse::<Block>()?;
        Ok(Self {
            ident,
            pat: pattern,
            block
        })
    }
}


struct SwitchCases {
    cases: Punctuated<SwitchCase, Token![,]>,
}

impl Parse for SwitchCases {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let cases = Punctuated::<SwitchCase, Token![,]>::parse_terminated(input)?;
        Ok(Self {
            cases
        })
    }
}

#[proc_macro]
pub fn multi_choice(s: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: SwitchCases = syn::parse(s).unwrap();

    let first = input.cases.first().unwrap();
    
    let ident = &first.ident;
    let block = &first.block;

    let mut first = quote! {
        let mut greatest;
        
        match s.token::<#ident>() {
            Ok(v) => return #block,
            Err(c) => {
                greatest = Some(c);
            }
        }
    };


    for case in input.cases.iter().skip(1) {

        let ident = &case.ident;
        let block = &case.block;

        let new = quote! {
            #first

            match s.token::<#ident>() {
                Ok(v) => #block,
                Err(c) => {
                    if c.1 > greatest.as_ref().unwrap().1 {
                        greatest = Some(c);
                    }
                }
            }
        };
        first = new;
    }
    
    first = quote! {
        #first

        Err(greatest.unwrap().0)
    };


    
    first.into()
}

