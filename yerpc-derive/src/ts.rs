use crate::{util::extract_method_io, RpcInfo};
use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;
pub(crate) fn generate_typescript_generator(info: &RpcInfo, outdir_path: &String) -> TokenStream {
    let mut gen_types = vec![];
    let mut gen_methods = vec![];
    for method in &info.methods {
        let (is_positional, gen_args, gen_output) =
            extract_method_io(method, &mut gen_types, |n| n.to_case(Case::Camel));
        let ts_name = method.name.to_case(Case::Camel);
        let rpc_name = &method.name;
        let is_notification = method.is_notification;
        let docs = if let Some(docs) = &method.docs {
            quote!(Some(#docs))
        } else {
            quote!(None)
        };
        gen_methods.push(quote!(
                let args = vec![#(#gen_args),*];
                let method = Method::new(#ts_name, #rpc_name, args, #gen_output, #is_notification, #is_positional, #docs);
                out.push_str(&method.to_string(root_namespace));
        ));
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let outdir = std::path::PathBuf::from(&manifest_dir).join(outdir_path);
    let outdir = outdir.to_str().unwrap();

    let ts_base = include_str!("client.ts");

    let mut all_types: Vec<String> = gen_types
        .clone()
        .into_iter()
        .map(|ts| ts.to_string())
        .collect();
    all_types.sort();
    all_types.dedup();
    let all_types: Vec<TokenStream> = all_types.into_iter().map(|s| s.parse().unwrap()).collect();

    quote! {
        /// Generate typescript bindings for the JSON-RPC API.
        #[cfg(test)]
        #[test]
        fn generate_ts_bindings() {
            use ::yerpc::typescript::type_def::{TypeDef, type_expr::TypeInfo, DefinitionFileOptions};
            use ::yerpc::typescript::{typedef_to_expr_string, export_types_to_file, Method};
            use ::std::{fs, path::Path};
            use ::std::io::Write;

            // Create output directory.
            let outdir = Path::new(#outdir);
            fs::create_dir_all(&outdir).expect(&format!("Failed to create directory `{}`", outdir.display()));

            // Create helper type with all exported types.
            // #(#gen_definitions)*
            #[derive(TypeDef)]
            struct __AllTyps(#(#all_types),*);
            // Write typescript types to file.
            export_types_to_file::<__AllTyps>(&outdir.join("types.ts"), None).expect("Failed to write TS out");
            // remove __AllTyps ts type from output,
            // it's only used as a woraround to export all types and is not needed anymore now
            let new_content = {
                let string =
                    ::std::fs::read_to_string(&outdir.join("types.ts")).expect("Failed to find TS out");
                if let Some(index) = string.find("export type __AllTyps") {
                    string[..index].to_string()
                } else {
                    panic!("did not find __AllTyps in TS out");
                }
            };
            ::std::fs::File::create(&outdir.join("types.ts"))
                .expect("failed to open TS out")
                .write_all(new_content.as_bytes())
                .expect("removing __AllTyps from TS failed");
            export_types_to_file::<::yerpc::Message>(&outdir.join("jsonrpc.ts"), None).expect("Failed to write TS out");

            // // Generate a raw client.
            let root_namespace = Some("T");
            let mut out = String::new();
            #(#gen_methods)*
            let ts_module = #ts_base.replace("#methods", &out);
            fs::write(&outdir.join("client.ts"), &ts_module).expect("Failed to write TS bindings");
        }
    }
}
