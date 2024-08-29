use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, Ident, ItemFn, LitStr, Type};

#[proc_macro_attribute]
pub fn snapshot(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mod_name = parse_macro_input!(attr as Ident).to_string();
    let mut input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = input_fn.sig.ident.clone();
    let snapshot_name = format!("{}/{}", mod_name, fn_name.to_string());

    let mut use_sc = false;
    match &input_fn.sig.inputs[0] {
        FnArg::Receiver(_) => panic!(),
        FnArg::Typed(t) => {
            if let Type::Reference(r) = t.ty.as_ref() {
                if let Type::Path(p) = r.elem.as_ref() {
                    if p.path.segments[0].ident.to_string() == "SerializerContext".to_string() {
                        use_sc = true;
                    } else {
                        panic!();
                    }
                } else {
                    panic!();
                }
            } else {
                panic!();
            }
        }
    };

    let impl_ident = syn::Ident::new(&format!("{}_impl", fn_name), fn_name.span());
    input_fn.sig.ident = impl_ident.clone();

    let expanded = quote! {
        #input_fn

        #[test]
        fn #fn_name() {
            let settings = SerializeSettings::default_test();
            let mut sc = SerializerContext::new(settings);
            #impl_ident(&mut sc);
            check_snapshot(#snapshot_name, sc.finish().as_bytes());
        }
    };

    expanded.into()
}
