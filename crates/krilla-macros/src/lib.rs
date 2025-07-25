use proc_macro::TokenStream;
use quote::{format_ident, quote};
use sitro::Renderer;
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
    SinglePage,
    Document,
}

const SKIP_SNAPSHOT: Option<&str> = option_env!("SKIP_SNAPSHOT");

#[proc_macro_attribute]
pub fn snapshot(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as AttributeInput);
    let mut serialize_settings = format_ident!("settings_1");
    let mut mode = SnapshotMode::SinglePage;
    let mut ignore = SKIP_SNAPSHOT.is_some();

    for attr in attrs.identifiers {
        let st = attr.to_string();

        if st.starts_with("settings") {
            serialize_settings = attr.clone();
        } else if st == "document" {
            mode = SnapshotMode::Document;
        } else if st == "ignore" {
            ignore = true;
        } else {
            panic!("unknown setting {st}");
        }
    }

    let mut input_fn = parse_macro_input!(item as ItemFn);
    let mut fn_name = input_fn.sig.ident.clone();
    let snapshot_name = fn_name.to_string();

    let impl_ident = Ident::new(&format!("{fn_name}_snapshot_impl"), fn_name.span());
    input_fn.sig.ident = impl_ident.clone();

    fn_name = Ident::new(&format!("{fn_name}_snapshot"), fn_name.span());

    let common = quote! {
        use krilla::SerializeSettings;
        use crate::check_snapshot;
        use krilla::Document;
        use krilla::page::PageSettings;
        use krilla::geom::Size;
    };

    let fn_content = match mode {
        SnapshotMode::SinglePage => {
            quote! {
                #common
                let settings = crate::#serialize_settings();
                let page_settings = PageSettings::new(200.0, 200.0);
                let mut d = Document::new_with(settings);
                let mut page = d.start_page_with(page_settings);
                #impl_ident(&mut page);
                page.finish();
                check_snapshot(#snapshot_name, &d.finish().unwrap(), true);
            }
        }
        SnapshotMode::Document => {
            quote! {
                #common
                let settings = crate::#serialize_settings();
                let mut d = Document::new_with(settings);
                #impl_ident(&mut d);
                check_snapshot(#snapshot_name, &d.finish().unwrap(), true);
            }
        }
    };

    let ignore_snippet = if ignore {
        quote! { #[ignore] }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #input_fn

        #ignore_snippet
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
            Renderer::Pdfbox => quote!(Renderer::Pdfbox),
            Renderer::Ghostscript => quote!(Renderer::Ghostscript),
            _ => unreachable!(),
        }
    }
}

const VISREG: Option<&str> = option_env!("VISREG");
const SKIP_SVG: Option<&str> = option_env!("SKIP_SVG");

#[proc_macro_attribute]
pub fn visreg(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as AttributeInput);
    let mut serialize_settings = format_ident!("default");

    let mut pdfium = false;
    let mut ignore_renderer = false;
    let mut mupdf = false;
    let mut pdfbox = false;
    let mut ghostscript = false;
    let mut poppler = false;
    let mut quartz = false;
    let mut document = false;
    let mut is_svg = false;
    let mut ignore = false;
    let mut only_macos = false;

    if !attrs.identifiers.iter().any(|ident| {
        let string_ident = ident.to_string();
        matches!(
            string_ident.as_str(),
            "pdfium" | "mupdf" | "pdfbox" | "ghostscript" | "poppler" | "quartz" | "all"
        )
    }) {
        pdfium = true;
        ignore_renderer = true;
    }

    for identifier in attrs.identifiers {
        let string_ident = identifier.to_string();

        if string_ident.starts_with("settings") {
            serialize_settings = identifier.clone();
            continue;
        }

        match string_ident.as_str() {
            "pdfium" => pdfium = true,
            "mupdf" => mupdf = true,
            "pdfbox" => pdfbox = true,
            "ghostscript" => ghostscript = true,
            "poppler" => poppler = true,
            "quartz" => quartz = true,
            "document" => document = true,
            "svg" => is_svg = true,
            "ignore" => ignore = true,
            "macos" => only_macos = true,
            "all" => {
                pdfium = true;
                mupdf = true;
                pdfbox = true;
                ghostscript = true;
                poppler = true;
                quartz = true;
            }
            _ => panic!("unknown renderer {}", &string_ident),
        }
    }

    let mut input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = input_fn.sig.ident.clone();

    let impl_ident = Ident::new(&format!("{fn_name}_visreg_impl"), fn_name.span());
    input_fn.sig.ident = impl_ident.clone();

    let fn_body = if is_svg {
        quote! {
            use crate::svg_impl;
            svg_impl(stringify!(#fn_name), renderer, #ignore_renderer);
        }
    } else if document {
        quote! {
            let settings = crate::#serialize_settings();
            let mut d = Document::new_with(settings);
            #impl_ident(&mut d);
            let pdf = d.finish().unwrap();

            let rendered = render_document(&pdf, &renderer);
            check_render(stringify!(#fn_name), None, &renderer, rendered, &pdf, #ignore_renderer);
        }
    } else {
        quote! {
            let settings = crate::#serialize_settings();
            let mut d = Document::new_with(settings);
            let page_settings = PageSettings::new(200.0, 200.0).with_media_box(None);
            let mut page = d.start_page_with(page_settings);
            let mut surface = page.surface();
            #impl_ident(&mut surface);
            surface.finish();
            page.finish();
            let pdf = d.finish().unwrap();

            let rendered = render_document(&pdf, &renderer);
            check_render(stringify!(#fn_name), None, &renderer, rendered, &pdf, #ignore_renderer);
        }
    };

    let renderer_body = |renderer: Renderer, include: bool| {
        let name = format_ident!("{}_visreg_{}", fn_name.to_string(), renderer.name());
        let renderer_ident = renderer.as_token_stream();

        let ignore_snippet = if VISREG.is_none() || ignore || (SKIP_SVG.is_some() && is_svg) {
            quote! { #[ignore] }
        } else {
            quote! {}
        };

        let quartz_snippet = if renderer == Renderer::Quartz || only_macos {
            quote! { #[cfg(target_os = "macos")] }
        } else {
            quote! {}
        };

        if include {
            quote! {
                #ignore_snippet
                #quartz_snippet
                #[test]
                fn #name() {
                    use crate::{render_document, check_render};
                    use krilla::geom::Size;
                    use krilla::{Document, SerializeSettings};
                    use krilla::page::PageSettings;
                    use sitro::Renderer;
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
    let poppler = renderer_body(Renderer::Poppler, poppler);
    let quartz = renderer_body(Renderer::Quartz, quartz);

    let expanded = quote! {
        #input_fn

        #pdfium
        #mupdf
        #ghostscript
        #pdfbox
        #poppler
        #quartz
    };

    expanded.into()
}
