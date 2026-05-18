use crate::{util::extract_method_io, RpcInfo};
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn generate_c_generator(
    info: &RpcInfo,
    outdir_path: &String,
    namespace: &str,
) -> TokenStream {
    let mut gen_types = vec![];
    let mut gen_methods = vec![];
    for method in &info.methods {
        let (is_positional, gen_args, gen_output) =
            extract_method_io(method, &mut gen_types, |n| n);
        let rpc_name = &method.name;
        let is_notification = method.is_notification;
        gen_methods.push(quote!(
            let args: Vec<_> = vec![#(#gen_args),*].into_iter().map(|(n, i)| (n, ::yerpc::c::TypeExpr::from_info(i))).collect();
            let output = (#gen_output).map(::yerpc::c::TypeExpr::from_info);
            let method = ::yerpc::c::Method::new(#rpc_name, args, output, #is_notification, #is_positional);
            methods.push(method);
        ));
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let outdir = std::path::PathBuf::from(&manifest_dir).join(outdir_path);
    let outdir = outdir.to_str().unwrap();

    let mut all_types: Vec<String> = gen_types
        .clone()
        .into_iter()
        .map(|ts| ts.to_string())
        .collect();
    all_types.sort();
    all_types.dedup();
    let all_types: Vec<TokenStream> = all_types.into_iter().map(|s| s.parse().unwrap()).collect();

    quote! {
        /// Generate C bindings for the JSON-RPC API.
        #[cfg(test)]
        #[test]
        fn generate_c_bindings() {
            use ::yerpc::typescript::type_def::{TypeDef, type_expr::TypeInfo};
            use ::yerpc::c;
            use ::std::{fs, path::Path};


            let mut methods = Vec::<c::Method>::new();
            #(#gen_methods)*

            #[derive(TypeDef)]
            struct __AllCTyps(#(#all_types),*);
            let defs = c::collect_type_defs::<__AllCTyps>();
            c::write_files(&Path::new(#outdir), &methods, &defs, #namespace);
        }
    }
}
