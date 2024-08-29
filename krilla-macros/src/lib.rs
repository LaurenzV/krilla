use proc_macro::TokenStream;
use quote::quote;
use syn::__private::Span;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Ident, ItemFn, Token};

struct AttributeInput {
    identifiers: Punctuated<Ident, Token![,]>,
}

// Implement the Parse trait for our custom struct
impl Parse for AttributeInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(AttributeInput {
            identifiers: input.parse_terminated(Ident::parse, Token![,])?,
        })
    }
}

#[proc_macro_attribute]
pub fn snapshot(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as AttributeInput);
    let mod_name = attrs.identifiers[0].to_string();
    let mut serialize_settings = Ident::new("settings_1", Span::call_site());

    for attr in attrs.identifiers.iter().skip(1) {
        let st = attr.to_string();

        if st.starts_with("settings") {
            serialize_settings = attr.clone();
        } else {
            panic!("unknown setting {}", st);
        }
    }

    let mut input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = input_fn.sig.ident.clone();
    let snapshot_name = format!("{}/{}", mod_name, fn_name.to_string());

    let impl_ident = Ident::new(&format!("{}_impl", fn_name), fn_name.span());
    input_fn.sig.ident = impl_ident.clone();

    let expanded = quote! {
        #input_fn

        #[test]
        fn #fn_name() {
            let settings = SerializeSettings::#serialize_settings();
            let mut sc = SerializerContext::new(settings);
            #impl_ident(&mut sc);
            check_snapshot(#snapshot_name, sc.finish().as_bytes());
        }
    };

    expanded.into()
}
