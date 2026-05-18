use crate::{parse::RemoteProcedure, Inputs};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{GenericArgument, PathArguments, Type};

pub fn is_result_ty(ty: &Type) -> bool {
    if let Type::Path(path) = ty {
        if let Some(last) = path.path.segments.last() {
            if last.ident == "Result" {
                return true;
            }
        }
    }
    false
}

pub fn extract_result_ty(ty: &Type) -> &Type {
    if let Type::Path(path) = ty {
        if let Some(last) = path.path.segments.last() {
            if last.ident == "Result" {
                if let PathArguments::AngleBracketed(ref generics) = last.arguments {
                    if let Some(GenericArgument::Type(inner_ty)) = generics.args.first() {
                        return inner_ty;
                    }
                }
            }
        }
    }
    ty
}

pub(crate) fn extract_method_io(
    method: &RemoteProcedure,
    gen_types: &mut Vec<TokenStream>,
    name_fn: impl Fn(String) -> String,
) -> (bool, Vec<TokenStream>, TokenStream) {
    let (is_positional, gen_args) = match &method.input {
        Inputs::Positional(ref inputs) => {
            let mut gen_args = vec![];
            for (i, input) in inputs.iter().enumerate() {
                let ty = input.ty;
                let name = name_fn(
                    input
                        .ident
                        .map_or_else(|| format!("arg{}", i + 1), ToString::to_string),
                );
                gen_types.push(quote!(#ty));
                gen_args.push(quote!((#name.to_string(), &<#ty as TypeDef>::INFO)));
            }
            (true, gen_args)
        }
        Inputs::Structured(Some(input)) => {
            let ty = input.ty;
            let name = name_fn(
                input
                    .ident
                    .map_or_else(|| "params".to_string(), ToString::to_string),
            );
            gen_types.push(quote!(#ty));
            (
                false,
                vec![quote!((#name.to_string(), &<#ty as TypeDef>::INFO))],
            )
        }
        Inputs::Structured(None) => (false, vec![]),
    };
    let gen_output = match (method.output, method.is_notification) {
        (_, true) | (None, _) => quote!(None),
        (Some(ty), false) => {
            let ty = extract_result_ty(ty);
            gen_types.push(quote!(#ty));
            quote!(Some(&<#ty as TypeDef>::INFO))
        }
    };
    (is_positional, gen_args, gen_output)
}
