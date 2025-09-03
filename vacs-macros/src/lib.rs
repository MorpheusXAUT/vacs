use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn log_err(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = input_fn;

    let fn_name = &sig.ident;
    let is_async = sig.asyncness.is_some();

    // Wrap the original body in a match { Ok, Err } block
    let wrapped_body = if is_async {
        quote! {
            {
                match (async #block).await {
                    Ok(val) => Ok(val),
                    Err(err) => {
                        log::error!(target: concat!(module_path!(), "::", stringify!(#fn_name)), "{:?}", err);
                        Err(err)
                    }
                }
            }
        }
    } else {
        quote! {
            {
                match #block {
                    Ok(val) => Ok(val),
                    Err(err) => {
                        log::error!(target: concat!(module_path!(), "::", stringify!(#fn_name)), "{:?}", err);
                        Err(err)
                    }
                }
            }
        }
    };

    let output = quote! {
        #(#attrs)*
        #vis #sig #wrapped_body
    };

    output.into()
}
