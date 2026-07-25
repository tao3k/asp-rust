use super::RustItemProjectionNodeSyntax;
use super::labels::{
    binary_assignment_projection_label, break_projection_label, condition_projection_summary,
    fn_signature_projection_label, item_kind, item_projection_header, local_projection_label,
    macro_call_projection_label, return_projection_label, struct_field_projection_label,
    tail_expression_projection_label,
};
use super::token_compact::compact_tokens;
use syn::spanned::Spanned;

fn append_trait_projection_nodes(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    item_trait: &syn::ItemTrait,
    depth: usize,
) {
    let trait_owner = item_trait.ident.to_string();
    for trait_item in &item_trait.items {
        if let syn::TraitItem::Fn(method) = trait_item {
            let identity =
                crate::semantic_identity::canonical_item_identity::CanonicalItemIdentityV1::new(
                    "rust",
                    "trait-function",
                    method.sig.ident.to_string(),
                )
                .with_scope("trait-owner", "trait", trait_owner.clone());
            push_canonical_projection_node(
                nodes,
                method,
                "fn",
                "declaration",
                fn_signature_projection_label(&method.sig),
                depth,
                identity,
            );
            if let Some(block) = method.default.as_ref() {
                append_block_projection_nodes(nodes, block, depth + 1);
            }
        }
    }
}

pub(super) fn item_projection_nodes(item: &syn::Item) -> Vec<RustItemProjectionNodeSyntax> {
    fn append_child_nodes(
        nodes: &mut Vec<RustItemProjectionNodeSyntax>,
        item: &syn::Item,
        depth: usize,
    ) {
        match item {
            syn::Item::Fn(item_fn) => {
                append_block_projection_nodes(nodes, &item_fn.block, depth);
            }
            syn::Item::Impl(item_impl) => {
                append_impl_projection_nodes(nodes, item_impl, depth);
            }
            syn::Item::Trait(item_trait) => {
                append_trait_projection_nodes(nodes, item_trait, depth);
            }
            syn::Item::Struct(item_struct) => {
                append_struct_field_projection_nodes(nodes, &item_struct.fields, depth);
            }
            syn::Item::Mod(item_mod) => {
                append_module_nodes(nodes, item_mod, depth);
            }
            _ => {}
        }
    }

    fn append_module_nodes(
        nodes: &mut Vec<RustItemProjectionNodeSyntax>,
        item_mod: &syn::ItemMod,
        depth: usize,
    ) {
        if let Some((_, items)) = &item_mod.content {
            for item in items {
                push_projection_node(
                    nodes,
                    item,
                    item_kind(item),
                    "declaration",
                    item_projection_header(item),
                    depth,
                );
                append_child_nodes(nodes, item, depth + 1);
            }
        }
    }

    let mut nodes = vec![RustItemProjectionNodeSyntax {
        line: item.span().start().line.max(1),
        end_line: item.span().end().line.max(item.span().start().line.max(1)),
        kind: item_kind(item),
        role: "declaration",
        label: item_projection_header(item),
        depth: 0,
        canonical_item_identity: None,
    }];
    append_child_nodes(&mut nodes, item, 1);
    nodes
        .into_iter()
        .filter(|node| !node.label.trim().is_empty())
        .take(64)
        .collect()
}

fn append_struct_field_projection_nodes(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    fields: &syn::Fields,
    depth: usize,
) {
    match fields {
        syn::Fields::Named(fields) => {
            for (index, field) in fields.named.iter().enumerate() {
                push_projection_node(
                    nodes,
                    field,
                    "field",
                    "field",
                    struct_field_projection_label(field, index),
                    depth,
                );
            }
        }
        syn::Fields::Unnamed(fields) => {
            for (index, field) in fields.unnamed.iter().enumerate() {
                push_projection_node(
                    nodes,
                    field,
                    "field",
                    "field",
                    struct_field_projection_label(field, index),
                    depth,
                );
            }
        }
        syn::Fields::Unit => {}
    }
}

fn append_impl_projection_nodes(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    item_impl: &syn::ItemImpl,
    depth: usize,
) {
    let implementation_owner = quote::ToTokens::to_token_stream(item_impl.self_ty.as_ref())
        .to_string()
        .replace(' ', "");
    let trait_owner = item_impl.trait_.as_ref().map(|(_, path, _)| {
        quote::ToTokens::to_token_stream(path)
            .to_string()
            .replace(' ', "")
    });
    for impl_item in &item_impl.items {
        match impl_item {
            syn::ImplItem::Fn(method) => {
                let mut identity =
                    crate::semantic_identity::canonical_item_identity::CanonicalItemIdentityV1::new(
                        "rust",
                        "method",
                        method.sig.ident.to_string(),
                    )
                    .with_scope(
                        "implementation-owner",
                        "type",
                        implementation_owner.clone(),
                    );
                if let Some(trait_owner) = trait_owner.as_deref() {
                    identity = identity.with_scope("trait-owner", "trait", trait_owner);
                }
                push_canonical_projection_node(
                    nodes,
                    method,
                    "fn",
                    "declaration",
                    fn_signature_projection_label(&method.sig),
                    depth,
                    identity,
                );
                append_block_projection_nodes(nodes, &method.block, depth + 1);
            }
            syn::ImplItem::Const(item_const) => push_projection_node(
                nodes,
                item_const,
                "const",
                "declaration",
                format!("const {}", item_const.ident),
                depth,
            ),
            syn::ImplItem::Type(item_type) => push_projection_node(
                nodes,
                item_type,
                "type",
                "declaration",
                format!("type {};", item_type.ident),
                depth,
            ),
            _ => {}
        }
    }
}

fn append_block_projection_nodes(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    block: &syn::Block,
    depth: usize,
) {
    for statement in &block.stmts {
        append_statement_projection_nodes(nodes, statement, depth);
    }
}

fn append_statement_projection_nodes(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    statement: &syn::Stmt,
    depth: usize,
) {
    match statement {
        syn::Stmt::Local(local) => {
            let low_value_local = is_low_value_local(local);
            if !low_value_local {
                push_projection_node(
                    nodes,
                    local,
                    "let",
                    "mutation",
                    local_projection_label(local),
                    depth,
                );
            }
            if let Some(init) = &local.init
                && (low_value_local || local_initializer_needs_expansion(&init.expr))
            {
                append_expression_projection_nodes(nodes, &init.expr, depth);
            }
        }
        syn::Stmt::Item(item) => push_projection_node(
            nodes,
            item,
            "item",
            "declaration",
            item_projection_header(item),
            depth,
        ),
        syn::Stmt::Expr(expr, _) => append_tail_expression_projection_nodes(nodes, expr, depth),
        syn::Stmt::Macro(mac) => push_projection_node(
            nodes,
            &mac.mac,
            "macro",
            "call",
            macro_call_projection_label(&mac.mac),
            depth,
        ),
    }
}

fn append_expression_projection_nodes(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    expression: &syn::Expr,
    depth: usize,
) {
    match expression {
        syn::Expr::Return(return_expr) => {
            push_projection_node(
                nodes,
                return_expr,
                "return",
                "terminal",
                return_projection_label(return_expr),
                depth,
            );
        }
        syn::Expr::Break(break_expr) => {
            push_projection_node(
                nodes,
                break_expr,
                "break",
                "terminal",
                break_projection_label(break_expr),
                depth,
            );
        }
        syn::Expr::Continue(continue_expr) => {
            push_projection_node(
                nodes,
                continue_expr,
                "continue",
                "terminal",
                "continue;",
                depth,
            );
        }
        syn::Expr::If(if_expr) => {
            push_projection_node(
                nodes,
                if_expr,
                "if",
                "control-flow",
                format!("if {} {{", condition_projection_summary(&if_expr.cond)),
                depth,
            );
            append_block_projection_nodes(nodes, &if_expr.then_branch, depth + 1);
            if let Some((_, else_expr)) = &if_expr.else_branch {
                push_projection_node(nodes, else_expr, "else", "control-flow", "else {", depth);
                append_expression_projection_nodes(nodes, else_expr, depth + 1);
            }
        }
        syn::Expr::Match(match_expr) => {
            push_projection_node(
                nodes,
                match_expr,
                "match",
                "control-flow",
                format!(
                    "match {} {{",
                    condition_projection_summary(&match_expr.expr)
                ),
                depth,
            );
            for arm in &match_expr.arms {
                push_projection_node(
                    nodes,
                    &arm.pat,
                    "case",
                    "control-flow",
                    format!("{} => {{", compact_tokens(&arm.pat)),
                    depth + 1,
                );
                append_tail_expression_projection_nodes(nodes, &arm.body, depth + 2);
            }
        }
        syn::Expr::ForLoop(for_loop) => {
            push_projection_node(
                nodes,
                for_loop,
                "for",
                "control-flow",
                format!(
                    "for {} in {} {{",
                    compact_tokens(&for_loop.pat),
                    compact_tokens(&for_loop.expr)
                ),
                depth,
            );
            append_block_projection_nodes(nodes, &for_loop.body, depth + 1);
        }
        syn::Expr::While(while_expr) => {
            push_projection_node(
                nodes,
                while_expr,
                "while",
                "control-flow",
                format!(
                    "while {} {{",
                    condition_projection_summary(&while_expr.cond)
                ),
                depth,
            );
            append_block_projection_nodes(nodes, &while_expr.body, depth + 1);
        }
        syn::Expr::Loop(loop_expr) => {
            push_projection_node(nodes, loop_expr, "loop", "control-flow", "loop {", depth);
            append_block_projection_nodes(nodes, &loop_expr.body, depth + 1);
        }
        syn::Expr::Call(call) => push_projection_node(
            nodes,
            call,
            "call",
            "call",
            format!("{};", compact_tokens(call)),
            depth,
        ),
        syn::Expr::MethodCall(method_call) => push_projection_node(
            nodes,
            method_call,
            "call",
            "call",
            format!("{};", compact_tokens(method_call)),
            depth,
        ),
        syn::Expr::Macro(macro_expr) => push_projection_node(
            nodes,
            &macro_expr.mac,
            "macro",
            "call",
            macro_call_projection_label(&macro_expr.mac),
            depth,
        ),
        syn::Expr::Assign(assign) => {
            push_projection_node(
                nodes,
                assign,
                "assign",
                "mutation",
                format!(
                    "{} = {};",
                    compact_tokens(&assign.left),
                    compact_tokens(&assign.right)
                ),
                depth,
            );
            append_expression_projection_nodes(nodes, &assign.right, depth + 1);
        }
        syn::Expr::Await(await_expr) => {
            push_projection_node(
                nodes,
                await_expr,
                "await",
                "effect",
                format!("{}.await;", compact_tokens(&await_expr.base)),
                depth,
            );
            append_expression_projection_nodes(nodes, &await_expr.base, depth + 1);
        }
        syn::Expr::Try(try_expr) => {
            push_projection_node(
                nodes,
                try_expr,
                "try",
                "effect",
                format!("{}?;", compact_tokens(&try_expr.expr)),
                depth,
            );
            append_expression_projection_nodes(nodes, &try_expr.expr, depth + 1);
        }
        syn::Expr::Block(block) => append_block_projection_nodes(nodes, &block.block, depth),
        syn::Expr::Closure(closure) => {
            append_expression_projection_nodes(nodes, &closure.body, depth)
        }
        syn::Expr::Binary(binary) => {
            if let Some(label) = binary_assignment_projection_label(binary) {
                push_projection_node(nodes, binary, "assign", "mutation", label, depth);
                append_expression_projection_nodes(nodes, &binary.right, depth + 1);
            } else {
                append_expression_projection_nodes(nodes, &binary.left, depth);
                append_expression_projection_nodes(nodes, &binary.right, depth);
            }
        }
        syn::Expr::Unary(unary) => append_expression_projection_nodes(nodes, &unary.expr, depth),
        syn::Expr::Reference(reference) => {
            append_expression_projection_nodes(nodes, &reference.expr, depth);
        }
        syn::Expr::Paren(paren) => append_expression_projection_nodes(nodes, &paren.expr, depth),
        syn::Expr::Cast(cast) => append_expression_projection_nodes(nodes, &cast.expr, depth),
        syn::Expr::Index(index) => {
            append_expression_projection_nodes(nodes, &index.expr, depth);
            append_expression_projection_nodes(nodes, &index.index, depth);
        }
        syn::Expr::Field(field) => append_expression_projection_nodes(nodes, &field.base, depth),
        syn::Expr::Tuple(tuple) => {
            for element in &tuple.elems {
                append_expression_projection_nodes(nodes, element, depth);
            }
        }
        syn::Expr::Array(array) => {
            for element in &array.elems {
                append_expression_projection_nodes(nodes, element, depth);
            }
        }
        _ => {}
    }
}

fn append_tail_expression_projection_nodes(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    expression: &syn::Expr,
    depth: usize,
) {
    match expression {
        syn::Expr::Binary(binary) if binary_assignment_projection_label(binary).is_some() => {
            append_expression_projection_nodes(nodes, expression, depth);
        }
        syn::Expr::Call(_)
        | syn::Expr::Array(_)
        | syn::Expr::Macro(_)
        | syn::Expr::MethodCall(_)
        | syn::Expr::Binary(_)
        | syn::Expr::Path(_)
        | syn::Expr::Lit(_)
        | syn::Expr::Struct(_)
        | syn::Expr::Tuple(_) => {
            push_projection_node(
                nodes,
                expression,
                "return",
                "terminal",
                tail_expression_projection_label(expression),
                depth,
            );
        }
        _ => append_expression_projection_nodes(nodes, expression, depth),
    }
}

fn push_projection_node(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    syntax: &impl Spanned,
    kind: &'static str,
    role: &'static str,
    label: impl Into<String>,
    depth: usize,
) {
    let line = syntax.span().start().line.max(1);
    nodes.push(RustItemProjectionNodeSyntax {
        line,
        end_line: syntax.span().end().line.max(line),
        kind,
        role,
        label: label.into(),
        depth,
        canonical_item_identity: None,
    });
}

fn push_canonical_projection_node(
    nodes: &mut Vec<RustItemProjectionNodeSyntax>,
    syntax: &impl Spanned,
    kind: &'static str,
    role: &'static str,
    label: impl Into<String>,
    depth: usize,
    canonical_item_identity: crate::semantic_identity::canonical_item_identity::CanonicalItemIdentityV1,
) {
    push_projection_node(nodes, syntax, kind, role, label, depth);
    nodes
        .last_mut()
        .expect("projection node was just pushed")
        .canonical_item_identity = Some(canonical_item_identity);
}

fn is_low_value_local(local: &syn::Local) -> bool {
    matches!(&local.pat, syn::Pat::Wild(_))
}

fn local_initializer_needs_expansion(expression: &syn::Expr) -> bool {
    matches!(
        expression,
        syn::Expr::If(_)
            | syn::Expr::Match(_)
            | syn::Expr::ForLoop(_)
            | syn::Expr::While(_)
            | syn::Expr::Loop(_)
            | syn::Expr::Block(_)
            | syn::Expr::Closure(_)
    )
}
