use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use spacetime_bindings_macro_input::table::TableArgs;
use syn::{DeriveInput, Error};

/// Add `#[test]` to your structs with `#[spacetimedb::table]`
/// to interact in a more ergonomic way than SpacetimeDB allows you by default.
#[proc_macro_attribute]
pub fn test(_args: TokenStream, item: TokenStream) -> TokenStream {
    ok_or_compile_error(|| {
        // Parse the input tokens into a syntax tree
        let item: DeriveInput = syn::parse(item)?;

        let input = get_table_attribute_macro(&item, "table")?;

        let _ = TableArgs::parse(input, &item)?;

        Ok(proc_macro2::TokenStream::default())
    })
}

fn get_table_attribute_macro(
    item: &DeriveInput,
    path: &str,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut table = None;

    for attr in item.attrs.iter() {
        match attr.meta.require_list() {
            Ok(list) => {
                if list.path.to_token_stream().to_string().eq(path) {
                    // It's really important that list.tokens is converted to TokenStream and not attr or attr.meta!
                    table = Some(list.tokens.to_token_stream());
                }
            }
            Err(_) => {}
        }
    }

    match table {
        Some(table) => Ok(table),
        None => Err(Error::new(
            Span::call_site(),
            format!("Haven't found #[{path}] attribute macro!"),
        )),
    }
}

fn ok_or_compile_error<Res: Into<proc_macro::TokenStream>>(
    f: impl FnOnce() -> syn::Result<Res>,
) -> proc_macro::TokenStream {
    match f() {
        Ok(ok) => ok.into(),
        Err(e) => e.into_compile_error().into(),
    }
}
