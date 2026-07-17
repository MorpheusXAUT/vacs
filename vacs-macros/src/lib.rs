use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Fields, GenericArgument, Ident, ItemFn, PathArguments, Type,
    parse_macro_input,
};

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
                let __res = (|| #block)();
                match __res {
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

/// How a field is mapped from a persisted config struct to its camelCase `Frontend*` mirror.
enum FrontendKind {
    /// Moved as-is; the frontend field keeps the backend type.
    Plain,
    /// `Option<Code>` on the backend, `Option<String>` on the frontend. Converted via
    /// `Code::to_string` and `crate::keybinds::parse_key_code`.
    Key,
    /// `Option<Inner>` whose element carries its own `Frontend` derive; becomes
    /// `Option<FrontendInner>`.
    Nested,
    /// Backend-only field: excluded from the frontend struct and defaulted when converting back.
    Skip,
}

/// Derives a camelCase `Frontend*` mirror of a config struct together with the boilerplate
/// conversions between the two: an infallible `From<Backend>` and a fallible `TryFrom<Frontend>`
/// (error type `crate::error::Error`), plus a `Default` that delegates to the backend default.
///
/// Fields are plain by default; annotate the exceptions:
/// - `#[frontend(key)]` - `Option<Code>` <-> `Option<String>`.
/// - `#[frontend(nested)]` - `Option<Inner>` <-> `Option<FrontendInner>`.
/// - `#[frontend(skip)]` - backend-only field, omitted from the frontend struct.
#[proc_macro_derive(Frontend, attributes(frontend))]
pub fn derive_frontend(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    match expand_frontend(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_frontend(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let backend = &input.ident;
    let frontend = format_ident!("Frontend{}", backend);
    let vis = &input.vis;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    backend,
                    "Frontend can only be derived for structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                backend,
                "Frontend can only be derived for structs",
            ));
        }
    };

    let mut struct_fields = Vec::new();
    let mut to_frontend = Vec::new();
    let mut to_backend = Vec::new();

    for field in fields {
        let name = field.ident.as_ref().expect("named field");
        let fvis = &field.vis;
        match frontend_kind(field)? {
            FrontendKind::Plain => {
                let ty = &field.ty;
                struct_fields.push(quote! { #fvis #name: #ty });
                to_frontend.push(quote! { #name: value.#name });
                to_backend.push(quote! { #name: value.#name });
            }
            FrontendKind::Key => {
                struct_fields
                    .push(quote! { #fvis #name: ::core::option::Option<::std::string::String> });
                to_frontend.push(quote! { #name: value.#name.map(|code| code.to_string()) });
                to_backend.push(quote! { #name: crate::keybinds::parse_key_code(value.#name)? });
            }
            FrontendKind::Nested => {
                let inner = option_inner_ident(&field.ty)?;
                let frontend_inner = format_ident!("Frontend{}", inner);
                struct_fields.push(quote! { #fvis #name: ::core::option::Option<#frontend_inner> });
                to_frontend.push(quote! { #name: value.#name.map(::core::convert::Into::into) });
                to_backend.push(quote! {
                    #name: value.#name.map(::core::convert::TryInto::try_into).transpose()?
                });
            }
            FrontendKind::Skip => {
                to_backend.push(quote! { #name: ::core::default::Default::default() });
            }
        }
    }

    Ok(quote! {
        #[derive(::core::fmt::Debug, ::core::clone::Clone, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #vis struct #frontend {
            #(#struct_fields,)*
        }

        impl ::core::default::Default for #frontend {
            fn default() -> Self {
                <#backend as ::core::default::Default>::default().into()
            }
        }

        impl ::core::convert::From<#backend> for #frontend {
            fn from(value: #backend) -> Self {
                Self {
                    #(#to_frontend,)*
                }
            }
        }

        impl ::core::convert::TryFrom<#frontend> for #backend {
            type Error = crate::error::Error;

            fn try_from(value: #frontend) -> ::core::result::Result<Self, Self::Error> {
                ::core::result::Result::Ok(Self {
                    #(#to_backend,)*
                })
            }
        }
    })
}

fn frontend_kind(field: &syn::Field) -> syn::Result<FrontendKind> {
    let mut kind = FrontendKind::Plain;
    for attr in &field.attrs {
        if !attr.path().is_ident("frontend") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("key") {
                kind = FrontendKind::Key;
            } else if meta.path.is_ident("nested") {
                kind = FrontendKind::Nested;
            } else if meta.path.is_ident("skip") {
                kind = FrontendKind::Skip;
            } else {
                return Err(meta.error("expected one of `key`, `nested`, `skip`"));
            }
            Ok(())
        })?;
    }
    Ok(kind)
}

/// Extract `Inner` from an `Option<Inner>` field type.
fn option_inner_ident(ty: &Type) -> syn::Result<Ident> {
    if let Type::Path(path) = ty
        && let Some(segment) = path.path.segments.last()
        && segment.ident == "Option"
        && let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(Type::Path(inner))) = args.args.first()
        && let Some(inner_segment) = inner.path.segments.last()
    {
        return Ok(inner_segment.ident.clone());
    }
    Err(syn::Error::new_spanned(
        ty,
        "#[frontend(nested)] requires a field of type `Option<Inner>`",
    ))
}
