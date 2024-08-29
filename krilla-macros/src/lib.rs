use proc_macro::TokenStream;
use quote::{format_ident, quote};
use sitro::Renderer;
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

enum SnapshotMode {
    SerializerContext,
    SinglePage,
    Document,
}

#[proc_macro_attribute]
pub fn snapshot(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as AttributeInput);
    let mod_name = attrs.identifiers[0].to_string();
    let mut serialize_settings = Ident::new("settings_1", Span::call_site());
    let mut mode = SnapshotMode::SerializerContext;

    for attr in attrs.identifiers.iter().skip(1) {
        let st = attr.to_string();

        if st.starts_with("settings") {
            serialize_settings = attr.clone();
        } else if st == "single_page" {
            mode = SnapshotMode::SinglePage
        } else if st == "document" {
            mode = SnapshotMode::Document
        } else {
            panic!("unknown setting {}", st);
        }
    }

    let mut input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = input_fn.sig.ident.clone();
    let snapshot_name = format!("{}/{}", mod_name, fn_name.to_string());

    let impl_ident = Ident::new(&format!("{}_impl", fn_name), fn_name.span());
    input_fn.sig.ident = impl_ident.clone();

    let fn_content = match mode {
        SnapshotMode::SerializerContext => {
            quote! {
                let settings = SerializeSettings::#serialize_settings();
                let mut sc = SerializerContext::new(settings);
                #impl_ident(&mut sc);
                check_snapshot(#snapshot_name, sc.finish().as_bytes());
            }
        }
        SnapshotMode::SinglePage => {
            quote! {
                let settings = SerializeSettings::#serialize_settings();
                let mut db = Document::new(settings);
                let mut page = db.start_page(Size::from_wh(200.0, 200.0).unwrap());
                #impl_ident(&mut page);
                page.finish();
                check_snapshot(#snapshot_name, &db.finish());
            }
        }
        SnapshotMode::Document => {
            quote! {
                let settings = SerializeSettings::#serialize_settings();
                let mut db = Document::new(settings);
                #impl_ident(&mut db);
                check_snapshot(#snapshot_name, &db.finish());
            }
        }
    };

    let expanded = quote! {
        #input_fn

        #[test]
        fn #fn_name() {
            #fn_content
        }
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn visreg(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as AttributeInput);
    let serialize_settings = Ident::new("default", Span::call_site());

    let mut pdfium = true;
    let mut mupdf = true;

    let mut input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = input_fn.sig.ident.clone();

    let impl_ident = Ident::new(&format!("{}_impl", fn_name), fn_name.span());
    input_fn.sig.ident = impl_ident.clone();

    let fn_body = quote! {
        let settings = SerializeSettings::#serialize_settings();
            let mut db = Document::new(settings);
            let mut page = db.start_page(Size::from_wh(200.0, 200.0).unwrap());
            let mut surface = page.surface();
            #impl_ident(&mut surface);
            surface.finish();
            page.finish();
            let pdf = db.finish();

            let rendered = render_doc(&pdf, &renderer);
            save_refs(stringify!(#fn_name), &renderer, rendered);
    };

    let pdfium_body = if pdfium {
        let pdfium_name = format_ident!("{}_{}", fn_name.to_string(), "pdfium");
        quote! {
            #[test]
            fn #pdfium_name() {
                let renderer = Renderer::Pdfium;
                #fn_body
            }
        }
    } else {
        quote! {}
    };

    let mupdf_body = if mupdf {
        let pdfium_name = format_ident!("{}_{}", fn_name.to_string(), "mupdf");
        quote! {
            #[test]
            fn #pdfium_name() {
                let renderer = Renderer::Mupdf;
                #fn_body
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #input_fn

        #pdfium_body

        #mupdf_body
    };

    expanded.into()
}
