
use syn::{parse::Parse};



enum ParseStmt {
    A
}

impl Parse for ParseStmt {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Err(syn::Error::new(input.span(), "a"))
    }
}

struct ParseableDecl {
    pub ident: syn::Ident,
    pub block: Vec<ParseStmt>
}

impl Parse for ParseableDecl {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<syn::Ident>()?;
        let content;
        syn::braced!(content in input);


        let mut block = vec![];
        
        loop {
            match content.parse::<ParseStmt>() {
                Ok(v) => block.push(v),
                Err(e) => {
                    if content.is_empty() {
                        break;
                    } else {
                        return Err(e);
                    }
                },
            }
        }

        Ok(Self {
            ident,
            block
        })

    }
}


