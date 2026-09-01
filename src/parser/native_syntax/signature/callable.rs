use quote::ToTokens;
use syn::spanned::Spanned;

use crate::parser::native_syntax::data_shape::is_public_visibility;
use crate::parser::native_syntax::signature::contract_type::{
    dynamic_json_api_type_name, flag_contract_type_name, primitive_contract_type_name,
    tuple_api_contract_types,
};
use crate::parser::native_syntax::signature::error_boundary::application_error_return_type;
use crate::parser::native_syntax::signature::{
    RustFunctionDynamicJsonApiSyntax, RustFunctionParamSyntax, RustFunctionReturnSyntax,
    RustFunctionTupleApiSyntax,
};

pub(crate) fn public_function_param_syntax(item: &syn::Item) -> Vec<RustFunctionParamSyntax> {
    match item {
        syn::Item::Fn(item_fn) => public_item_function_param_syntax(item_fn, false),
        syn::Item::Impl(item_impl) => public_impl_function_param_syntax(item_impl),
        _ => Vec::new(),
    }
}

pub(crate) fn public_function_return_syntax(item: &syn::Item) -> Vec<RustFunctionReturnSyntax> {
    match item {
        syn::Item::Fn(item_fn) => public_item_function_return_syntax(item_fn, false)
            .into_iter()
            .collect(),
        syn::Item::Impl(item_impl) => public_impl_function_return_syntax(item_impl),
        _ => Vec::new(),
    }
}

pub(crate) fn public_function_tuple_api_syntax(
    item: &syn::Item,
) -> Vec<RustFunctionTupleApiSyntax> {
    match item {
        syn::Item::Fn(item_fn) => public_item_function_tuple_api_syntax(item_fn, false),
        syn::Item::Impl(item_impl) => public_impl_function_tuple_api_syntax(item_impl),
        _ => Vec::new(),
    }
}

pub(crate) fn public_function_dynamic_json_api_syntax(
    item: &syn::Item,
) -> Vec<RustFunctionDynamicJsonApiSyntax> {
    match item {
        syn::Item::Fn(item_fn) => public_item_function_dynamic_json_api_syntax(item_fn, false),
        syn::Item::Impl(item_impl) => public_impl_function_dynamic_json_api_syntax(item_impl),
        _ => Vec::new(),
    }
}

fn public_item_function_param_syntax(
    item_fn: &syn::ItemFn,
    inherited_test_context: bool,
) -> Vec<RustFunctionParamSyntax> {
    if !is_public_visibility(&item_fn.vis) {
        return Vec::new();
    }
    signature_param_syntax(
        &item_fn.sig,
        inherited_test_context || attrs_have_cfg_test(&item_fn.attrs),
    )
}

fn public_impl_function_param_syntax(item_impl: &syn::ItemImpl) -> Vec<RustFunctionParamSyntax> {
    let inherited_test_context = attrs_have_cfg_test(&item_impl.attrs);
    item_impl
        .items
        .iter()
        .filter_map(|impl_item| {
            let syn::ImplItem::Fn(method) = impl_item else {
                return None;
            };
            impl_method_is_public_api(item_impl, method).then_some(method)
        })
        .flat_map(|method| {
            signature_param_syntax(
                &method.sig,
                inherited_test_context || attrs_have_cfg_test(&method.attrs),
            )
        })
        .collect()
}

fn signature_param_syntax(
    signature: &syn::Signature,
    is_test_context: bool,
) -> Vec<RustFunctionParamSyntax> {
    let function_name = signature.ident.to_string();
    let function_line = signature.ident.span().start().line.max(1);
    signature
        .inputs
        .iter()
        .filter_map(|arg| {
            let syn::FnArg::Typed(pat_type) = arg else {
                return None;
            };
            let syn::Pat::Ident(pat_ident) = pat_type.pat.as_ref() else {
                return None;
            };
            Some(RustFunctionParamSyntax {
                line: pat_ident.span().start().line.max(1),
                function_line,
                function_name: function_name.clone(),
                param_name: pat_ident.ident.to_string(),
                type_text: pat_type.ty.to_token_stream().to_string(),
                primitive_contract_type: primitive_contract_type_name(&pat_type.ty),
                flag_contract_type: flag_contract_type_name(&pat_type.ty),
                is_test_context,
            })
        })
        .collect()
}

fn public_item_function_tuple_api_syntax(
    item_fn: &syn::ItemFn,
    inherited_test_context: bool,
) -> Vec<RustFunctionTupleApiSyntax> {
    if !is_public_visibility(&item_fn.vis) {
        return Vec::new();
    }
    signature_tuple_api_syntax(
        &item_fn.sig,
        inherited_test_context || attrs_have_cfg_test(&item_fn.attrs),
    )
}

fn public_item_function_dynamic_json_api_syntax(
    item_fn: &syn::ItemFn,
    inherited_test_context: bool,
) -> Vec<RustFunctionDynamicJsonApiSyntax> {
    if !is_public_visibility(&item_fn.vis) {
        return Vec::new();
    }
    signature_dynamic_json_api_syntax(
        &item_fn.sig,
        inherited_test_context || attrs_have_cfg_test(&item_fn.attrs),
    )
}

fn public_impl_function_tuple_api_syntax(
    item_impl: &syn::ItemImpl,
) -> Vec<RustFunctionTupleApiSyntax> {
    let inherited_test_context = attrs_have_cfg_test(&item_impl.attrs);
    item_impl
        .items
        .iter()
        .filter_map(|impl_item| {
            let syn::ImplItem::Fn(method) = impl_item else {
                return None;
            };
            impl_method_is_public_api(item_impl, method).then_some(method)
        })
        .flat_map(|method| {
            signature_tuple_api_syntax(
                &method.sig,
                inherited_test_context || attrs_have_cfg_test(&method.attrs),
            )
        })
        .collect()
}

fn public_impl_function_dynamic_json_api_syntax(
    item_impl: &syn::ItemImpl,
) -> Vec<RustFunctionDynamicJsonApiSyntax> {
    let inherited_test_context = attrs_have_cfg_test(&item_impl.attrs);
    item_impl
        .items
        .iter()
        .filter_map(|impl_item| {
            let syn::ImplItem::Fn(method) = impl_item else {
                return None;
            };
            impl_method_is_public_api(item_impl, method).then_some(method)
        })
        .flat_map(|method| {
            signature_dynamic_json_api_syntax(
                &method.sig,
                inherited_test_context || attrs_have_cfg_test(&method.attrs),
            )
        })
        .collect()
}

fn signature_tuple_api_syntax(
    signature: &syn::Signature,
    is_test_context: bool,
) -> Vec<RustFunctionTupleApiSyntax> {
    let function_name = signature.ident.to_string();
    let function_line = signature.ident.span().start().line.max(1);
    let mut facts = signature
        .inputs
        .iter()
        .filter_map(|arg| {
            let syn::FnArg::Typed(pat_type) = arg else {
                return None;
            };
            let syn::Pat::Ident(pat_ident) = pat_type.pat.as_ref() else {
                return None;
            };
            let element_contract_types = tuple_api_contract_types(&pat_type.ty)?;
            Some(RustFunctionTupleApiSyntax {
                line: pat_ident.span().start().line.max(1),
                function_line,
                function_name: function_name.clone(),
                surface_name: format!("parameter `{}`", pat_ident.ident),
                type_text: pat_type.ty.to_token_stream().to_string(),
                element_contract_types,
                is_test_context,
            })
        })
        .collect::<Vec<_>>();

    if let syn::ReturnType::Type(_, return_type) = &signature.output
        && let Some(element_contract_types) = tuple_api_contract_types(return_type)
    {
        facts.push(RustFunctionTupleApiSyntax {
            line: function_line,
            function_line,
            function_name,
            surface_name: "return value".to_owned(),
            type_text: return_type.to_token_stream().to_string(),
            element_contract_types,
            is_test_context,
        });
    }

    facts
}

fn signature_dynamic_json_api_syntax(
    signature: &syn::Signature,
    is_test_context: bool,
) -> Vec<RustFunctionDynamicJsonApiSyntax> {
    let function_name = signature.ident.to_string();
    let function_line = signature.ident.span().start().line.max(1);
    let mut facts = signature
        .inputs
        .iter()
        .filter_map(|arg| {
            let syn::FnArg::Typed(pat_type) = arg else {
                return None;
            };
            let syn::Pat::Ident(pat_ident) = pat_type.pat.as_ref() else {
                return None;
            };
            let json_type_name = dynamic_json_api_type_name(&pat_type.ty)?;
            Some(RustFunctionDynamicJsonApiSyntax {
                line: pat_ident.span().start().line.max(1),
                function_line,
                function_name: function_name.clone(),
                surface_name: format!("parameter `{}`", pat_ident.ident),
                type_text: pat_type.ty.to_token_stream().to_string(),
                json_type_name,
                is_test_context,
            })
        })
        .collect::<Vec<_>>();

    if let syn::ReturnType::Type(_, return_type) = &signature.output
        && let Some(json_type_name) = dynamic_json_api_type_name(return_type)
    {
        facts.push(RustFunctionDynamicJsonApiSyntax {
            line: function_line,
            function_line,
            function_name,
            surface_name: "return value".to_owned(),
            type_text: return_type.to_token_stream().to_string(),
            json_type_name,
            is_test_context,
        });
    }

    facts
}

fn public_item_function_return_syntax(
    item_fn: &syn::ItemFn,
    inherited_test_context: bool,
) -> Option<RustFunctionReturnSyntax> {
    if !is_public_visibility(&item_fn.vis) {
        return None;
    };
    signature_return_syntax(
        &item_fn.sig,
        inherited_test_context || attrs_have_cfg_test(&item_fn.attrs),
        None,
        None,
    )
}

fn public_impl_function_return_syntax(item_impl: &syn::ItemImpl) -> Vec<RustFunctionReturnSyntax> {
    let inherited_test_context = attrs_have_cfg_test(&item_impl.attrs);
    let impl_type = Some(item_impl.self_ty.to_token_stream().to_string());
    let trait_path = item_impl
        .trait_
        .as_ref()
        .map(|(path, _)| path.to_token_stream().to_string());
    item_impl
        .items
        .iter()
        .filter_map(|impl_item| {
            let syn::ImplItem::Fn(method) = impl_item else {
                return None;
            };
            if !impl_method_is_public_api(item_impl, method) {
                return None;
            }
            signature_return_syntax(
                &method.sig,
                inherited_test_context || attrs_have_cfg_test(&method.attrs),
                impl_type.clone(),
                trait_path.clone(),
            )
        })
        .collect()
}

fn signature_return_syntax(
    signature: &syn::Signature,
    is_test_context: bool,
    impl_type: Option<String>,
    trait_path: Option<String>,
) -> Option<RustFunctionReturnSyntax> {
    let syn::ReturnType::Type(_, return_type) = &signature.output else {
        return None;
    };
    Some(RustFunctionReturnSyntax {
        line: signature.ident.span().start().line.max(1),
        function_name: signature.ident.to_string(),
        type_text: return_type.to_token_stream().to_string(),
        is_async: signature.asyncness.is_some(),
        is_unsafe: matches!(&signature.safety, syn::Safety::Unsafe(_)),
        receiver: signature_receiver(signature),
        impl_type,
        trait_path,
        application_error_boundary: application_error_return_type(return_type),
        is_test_context,
    })
}

fn impl_method_is_public_api(item_impl: &syn::ItemImpl, method: &syn::ImplItemFn) -> bool {
    item_impl.trait_.is_some() || is_public_visibility(&method.vis)
}

fn signature_receiver(signature: &syn::Signature) -> Option<String> {
    signature.inputs.iter().find_map(|arg| {
        let syn::FnArg::Receiver(receiver) = arg else {
            return None;
        };
        Some(match &receiver.kind {
            syn::ReceiverKind::Reference(_, _, Some(_)) => "&mut-self".to_string(),
            syn::ReceiverKind::Reference(_, _, None) => "&self".to_string(),
            syn::ReceiverKind::Value if receiver.mutability.is_some() => "mut-self".to_string(),
            syn::ReceiverKind::Value | syn::ReceiverKind::Typed(_, _) => "self".to_string(),
            _ => "self".to_string(),
        })
    })
}

fn attrs_have_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(attribute_has_cfg_test)
}

fn attribute_has_cfg_test(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("cfg") && attr.to_token_stream().to_string().contains("test")
}
