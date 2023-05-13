use proc_macro::TokenStream;
use quote::quote;
use syn::{TraitItemFn, parse_macro_input, parse::Parse, parse_quote};

struct Protocol {
    fns: Vec<TraitItemFn>
}

impl Parse for Protocol {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut fns = vec![];
        while !input.is_empty() {
            fns.push(input.parse()?);
        }
        Ok(Protocol { fns })
    }
}

#[proc_macro]
pub fn protocol(tokens: TokenStream) -> TokenStream {
    let Protocol { fns } = parse_macro_input!(tokens as Protocol);
    let fns: Vec<_> = fns.iter().map(|f| match &f.sig.output {
        syn::ReturnType::Default => f.clone(),
        syn::ReturnType::Type(_, ty) => {
            let mut f = f.clone();
            f.sig.output = parse_quote!( -> Result<#ty, protoscheme::protocol::Error> );
            f
        },
    }).collect();
    let signals = fns.iter().filter(|f| match f.sig.output {
        syn::ReturnType::Default => true,
        syn::ReturnType::Type(_, _) => false,
    });
    let methods = fns.iter().filter(|f| match f.sig.output {
        syn::ReturnType::Default => false,
        syn::ReturnType::Type(_, _) => true,
    });
    quote! {

    }.into()
}