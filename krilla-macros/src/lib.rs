use proc_macro::TokenStream;
use quote::{format_ident, quote};
use sitro::Renderer;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, parse_quote, Ident, ItemFn, Token};

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
    let mut serialize_settings = format_ident!("settings_1");
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

trait RendererExt {
    fn as_token_stream(&self) -> proc_macro2::TokenStream;
}

impl RendererExt for Renderer {
    fn as_token_stream(&self) -> proc_macro2::TokenStream {
        match self {
            Renderer::Pdfium => quote!(Renderer::Pdfium),
            Renderer::Mupdf => quote!(Renderer::Mupdf),
            Renderer::Poppler => quote!(Renderer::Poppler),
            Renderer::Quartz => quote!(Renderer::Quartz),
            Renderer::Pdfjs => quote!(Renderer::Pdfjs),
            Renderer::Pdfbox => quote!(Renderer::Pdfbox),
            Renderer::Ghostscript => quote!(Renderer::Ghostscript),
        }
    }
}

#[proc_macro_attribute]
pub fn visreg(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as AttributeInput);
    let serialize_settings = format_ident!("default");

    let mut pdfium = true;
    let mut mupdf = true;
    let mut pdfbox = true;
    let mut ghostscript = true;
    let mut pdfjs = true;
    let mut poppler = true;
    let mut quartz = true;

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

    let renderer_body = |renderer: Renderer, include: bool| {
        let name = format_ident!("{}_{}", fn_name.to_string(), renderer.name());
        let renderer_ident = renderer.as_token_stream();

        let quartz_snippet = if renderer == Renderer::Quartz {
            quote! { #[cfg(target_os = "macos")] }
        } else {
            quote! {}
        };

        if include {
            quote! {
                #quartz_snippet
                #[test]
                fn #name() {
                    let renderer = #renderer_ident;
                    #fn_body
                }
            }
        } else {
            quote! {}
        }
    };

    let pdfium = renderer_body(Renderer::Pdfium, pdfium);
    let mupdf = renderer_body(Renderer::Mupdf, mupdf);
    let ghostscript = renderer_body(Renderer::Ghostscript, ghostscript);
    let pdfbox = renderer_body(Renderer::Pdfbox, pdfbox);
    let pdfjs = renderer_body(Renderer::Pdfjs, pdfjs);
    let poppler = renderer_body(Renderer::Poppler, poppler);
    let quartz = renderer_body(Renderer::Quartz, quartz);

    let expanded = quote! {
        #input_fn

        #pdfium
        #mupdf
        #ghostscript
        #pdfbox
        #pdfjs
        #poppler
        #quartz
    };

    expanded.into()
}
