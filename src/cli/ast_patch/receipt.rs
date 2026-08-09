use std::fs;
use std::ops::Range;
use std::path::Path;

use serde_json::{Value, json};

use super::core::{
    AST_PATCH_PROTOCOL_ID, AstPatchMode, RECEIPT_SCHEMA_ID, SUPPORTED_OPERATIONS, SourceRead,
    byte_range_for_line_range, format_candidate_source, formatter_failure_kind,
    insert_module_declaration, is_rust_identifier, is_schema_operation, item_identity,
    module_name_from_destination, normalized_replacement, normalized_split_destination_source,
    operation_max_edits, operation_string_field, parse_selected_rust_items, validate_item_identity,
    validate_project_path, validate_selected_item_identity, write_split_owner_files,
};

pub(super) fn render_ast_patch_receipt(
    mode: AstPatchMode,
    packet_text: &str,
    project_root: &Path,
) -> String {
    struct ResolvedTarget {
        read: SourceRead,
        byte_range: Range<usize>,
        item: syn::Item,
    }

    fn target_identity(target: &Value, replacement: &syn::Item) -> Option<(&'static str, String)> {
        match (
            target.get("itemKind").and_then(Value::as_str),
            target.get("itemName").and_then(Value::as_str),
        ) {
            (Some(kind), Some(name)) => Some((
                Box::leak(kind.to_string().into_boxed_str()),
                name.to_string(),
            )),
            _ => item_identity(replacement),
        }
    }

    fn resolved_read(path: &str, start_line: usize, end_line: usize) -> SourceRead {
        SourceRead {
            raw: format!("{path}:{start_line}:{end_line}"),
            path: path.to_string(),
            start_line,
            end_line,
        }
    }

    fn item_line_range(item: &syn::Item) -> Result<(usize, usize), String> {
        let span = syn::spanned::Spanned::span(item);
        let start = span.start();
        let end = span.end();
        if start.line == 0 || end.line == 0 || start.line > end.line {
            return Err("item span did not resolve to a source line range".to_string());
        }
        Ok((start.line, end.line))
    }

    fn item_from_read(source: &str, read: SourceRead) -> Result<ResolvedTarget, String> {
        let byte_range = byte_range_for_line_range(source, read.start_line, read.end_line)?;
        let target_source = &source[byte_range.clone()];
        let item = syn::parse_str::<syn::Item>(target_source)
            .map_err(|error| format!("selected target did not parse as Rust item: {error}"))?;
        Ok(ResolvedTarget {
            read,
            byte_range,
            item,
        })
    }

    fn validate_target_identity(target: &Value, existing: &syn::Item) -> Result<(), String> {
        let existing_identity = item_identity(existing);
        if let Some(expected_name) = target.get("itemName").and_then(Value::as_str) {
            let Some((_, existing_name)) = &existing_identity else {
                return Err("target item identity is unavailable".to_string());
            };
            if existing_name != expected_name {
                return Err(format!(
                    "target item name mismatch: expected {expected_name}, got {existing_name}"
                ));
            }
        }
        if let Some(expected_kind) = target.get("itemKind").and_then(Value::as_str) {
            let Some((existing_kind, _)) = &existing_identity else {
                return Err("target item identity is unavailable".to_string());
            };
            if *existing_kind != expected_kind {
                return Err(format!(
                    "target item kind mismatch: expected {expected_kind}, got {existing_kind}"
                ));
            }
        }
        Ok(())
    }

    fn resolve_current_item(
        source: &str,
        read_path: &str,
        expected_kind: &str,
        expected_name: &str,
    ) -> Result<ResolvedTarget, String> {
        let file = crate::parser::parse_rust_source_syntax(source)
            .map_err(|error| format!("current file did not parse as Rust source: {error}"))?;
        let mut matches = Vec::new();
        for item in file.items {
            let Some((kind, name)) = item_identity(&item) else {
                continue;
            };
            if kind != expected_kind || name != expected_name {
                continue;
            }
            let (start_line, end_line) = item_line_range(&item)?;
            let read = resolved_read(read_path, start_line, end_line);
            matches.push(item_from_read(source, read)?);
        }
        match matches.len() {
            1 => Ok(matches.remove(0)),
            0 => Err(format!(
                "no current Rust item matched identity {expected_kind}:{expected_name}"
            )),
            count => Err(format!(
                "{count} current Rust items matched identity {expected_kind}:{expected_name}"
            )),
        }
    }

    let packet: Value = match serde_json::from_str(packet_text) {
        Ok(packet) => packet,
        Err(error) => {
            return failure_receipt(
                mode,
                None,
                project_root,
                Vec::new(),
                "invalid-packet",
                format!("invalid ast patch packet json: {error}"),
                false,
            )
            .to_string();
        }
    };

    macro_rules! fail {
        ($root:expr, $verification:expr, $kind:expr, $message:expr) => {{
            return failure_receipt(
                mode,
                Some(&packet),
                $root,
                $verification,
                $kind,
                $message,
                true,
            )
            .to_string();
        }};
    }

    let mut verification = Vec::new();
    verification.push("packet-parsed");

    let target = packet.get("target").unwrap_or(&Value::Null);
    let operation = packet.get("operation").unwrap_or(&Value::Null);
    let operation_name = match operation.get("op").and_then(Value::as_str) {
        Some("replace_item") => "replace_item",
        Some("split_owner_items") => "split_owner_items",
        Some(value) => fail!(
            project_root,
            verification,
            "unsupported-operation",
            format!(
                "rust provider ast-patch supports replace_item and split_owner_items only, got {value}"
            )
        ),
        None => fail!(
            project_root,
            verification,
            "invalid-packet",
            "operation.op is required".to_string()
        ),
    };
    verification.push("operation-supported");

    let read = match target.get("read").and_then(Value::as_str) {
        Some(value) => match SourceRead::parse(value) {
            Ok(read) => read,
            Err(error) => fail!(project_root, verification, "target-read-invalid", error),
        },
        None => fail!(
            project_root,
            verification,
            "target-read-invalid",
            "target.read is required".to_string()
        ),
    };
    verification.push("target-read-valid");

    if operation_name == "split_owner_items" {
        return split_owner_items_receipt(
            mode,
            &packet,
            target,
            operation,
            read,
            project_root,
            verification,
        )
        .to_string();
    }

    let snippet = match operation.get("snippet").and_then(Value::as_str) {
        Some(snippet) => snippet,
        _ => fail!(
            project_root,
            verification,
            "snippet-missing",
            format!("operation {operation_name} requires operation.snippet")
        ),
    };
    let replacement_item = match syn::parse_str::<syn::Item>(snippet) {
        Ok(item) => item,
        Err(error) => fail!(
            project_root,
            verification,
            "snippet-parse-error",
            format!("replacement snippet did not parse as Rust item: {error}")
        ),
    };
    verification.push("snippet-parsed");

    let project_root = match project_root.canonicalize() {
        Ok(path) => path,
        Err(error) => fail!(
            project_root,
            verification,
            "project-root-invalid",
            format!("failed to resolve project root: {error}")
        ),
    };
    verification.push("project-root-resolved");

    let requested_source_path = project_root.join(&read.path);
    let source_path = match requested_source_path.canonicalize() {
        Ok(path) if path.starts_with(&project_root) => path,
        Ok(_) => fail!(
            &project_root,
            verification,
            "target-outside-project",
            format!("target path escapes project root: {}", read.path)
        ),
        Err(error) => fail!(
            &project_root,
            verification,
            "source-read-error",
            format!("failed to resolve target path {}: {error}", read.path)
        ),
    };
    verification.push("target-path-resolved");

    let source = match fs::read_to_string(&source_path) {
        Ok(source) => source,
        Err(error) => fail!(
            &project_root,
            verification,
            "source-read-error",
            format!("failed to read {}: {error}", read.path)
        ),
    };

    let initial = item_from_read(&source, read.clone()).and_then(|target_item| {
        validate_target_identity(target, &target_item.item).map(|_| target_item)
    });

    let resolved = match initial {
        Ok(target_item) => target_item,
        Err(original_error) => {
            if let Ok(byte_range) =
                byte_range_for_line_range(&source, read.start_line, read.end_line)
            {
                let selected = &source[byte_range];
                if let Ok(file) = crate::parser::parse_rust_source_syntax(selected)
                    && file.items.len() > 1
                {
                    verification.push("target-range-resolved");
                    if let Some(expected) = operation.get("expectedSnippet").and_then(Value::as_str)
                        && selected.contains(expected)
                    {
                        verification.push("expected-snippet-matched");
                    }
                    fail!(
                        &project_root,
                        verification,
                        "target-item-parse-error",
                        format!(
                            "target.read selected {} Rust items; expected exactly one",
                            file.items.len()
                        )
                    );
                }
            }
            let Some((expected_kind, expected_name)) = target_identity(target, &replacement_item)
            else {
                fail!(
                    &project_root,
                    verification,
                    "target-item-parse-error",
                    original_error
                )
            };
            verification.push("target-locator-stale");
            match resolve_current_item(&source, &read.path, expected_kind, &expected_name) {
                Ok(target_item) => {
                    if let Err(error) = validate_target_identity(target, &target_item.item) {
                        fail!(&project_root, verification, "target-item-mismatch", error);
                    }
                    verification.push("target-re-resolved");
                    target_item
                }
                Err(resolve_error) => fail!(
                    &project_root,
                    verification,
                    "target-locator-stale",
                    format!(
                        "target locator is stale and could not be re-resolved: {original_error}; {resolve_error}"
                    )
                ),
            }
        }
    };
    verification.push("target-range-resolved");

    if let Some(expected) = operation.get("expectedSnippet").and_then(Value::as_str) {
        let current = &source[resolved.byte_range.clone()];
        if !current.contains(expected) {
            fail!(
                &project_root,
                verification,
                "target-preimage-mismatch",
                "target source did not contain operation.expectedSnippet".to_string()
            );
        }
        verification.push("expected-snippet-matched");
    }

    verification.push("target-item-parsed");

    if let Err(error) = validate_item_identity(target, &resolved.item, &replacement_item) {
        fail!(&project_root, verification, "target-item-mismatch", error);
    }
    verification.push("target-identity-verified");

    let replacement = normalized_replacement(snippet);
    let mut next_source = String::with_capacity(
        source.len() - (resolved.byte_range.end - resolved.byte_range.start) + replacement.len(),
    );
    next_source.push_str(&source[..resolved.byte_range.start]);
    next_source.push_str(&replacement);
    next_source.push_str(&source[resolved.byte_range.end..]);

    if let Err(error) = crate::parser::parse_rust_source_syntax(&next_source) {
        fail!(
            &project_root,
            verification,
            "file-reparse-failed",
            format!("patched file did not parse as Rust source: {error}")
        )
    }
    verification.push("file-reparsed");

    let formatted = match format_candidate_source(&source_path, &next_source) {
        Ok(formatted) => formatted,
        Err((kind, error)) => {
            let failure_kind = match kind {
                "rustfmt-error" => "formatter-failed",
                "formatter-parse-error" => "formatter-output-reparse-failed",
                other => other,
            };
            fail!(&project_root, verification, failure_kind, error)
        }
    };
    verification.push("formatter-output-reparsed");

    if mode.writes_files() {
        if let Err(error) = fs::write(&source_path, formatted) {
            fail!(
                &project_root,
                verification,
                "source-write-error",
                format!("failed to write {}: {error}", read.path)
            )
        }
        verification.push("source-written");
    }

    let mut receipt_packet = packet.clone();
    if let Some(target) = receipt_packet
        .get_mut("target")
        .and_then(Value::as_object_mut)
    {
        target.insert("read".to_string(), Value::String(resolved.read.raw.clone()));
    }

    success_receipt(
        mode,
        Some(&receipt_packet),
        &project_root,
        &resolved.read,
        operation,
        verification,
    )
    .to_string()
}

fn split_owner_items_receipt(
    mode: AstPatchMode,
    packet: &Value,
    target: &Value,
    operation: &Value,
    read: SourceRead,
    project_root: &Path,
    mut verification: Vec<&'static str>,
) -> Value {
    macro_rules! fail {
        ($root:expr, $verification:expr, $kind:expr, $message:expr) => {{
            return failure_receipt(
                mode,
                Some(packet),
                $root,
                $verification,
                $kind,
                $message,
                true,
            );
        }};
    }

    if operation
        .get("mutationSource")
        .and_then(Value::as_str)
        .is_some_and(|value| value != "provider-native")
    {
        fail!(
            project_root,
            verification,
            "invalid-packet",
            "split_owner_items requires operation.mutationSource=provider-native".to_string()
        );
    }
    if operation
        .get("snippetRequired")
        .and_then(Value::as_bool)
        .is_some_and(|value| value)
    {
        fail!(
            project_root,
            verification,
            "invalid-packet",
            "split_owner_items requires operation.snippetRequired=false".to_string()
        );
    }
    if operation
        .get("codeInPrompt")
        .and_then(Value::as_bool)
        .is_some_and(|value| value)
    {
        fail!(
            project_root,
            verification,
            "invalid-packet",
            "split_owner_items requires operation.codeInPrompt=false".to_string()
        );
    }
    verification.push("provider-native-operation");

    let destination_path = match operation_string_field(operation, "destinationPath") {
        Some(value) => {
            if let Err(error) = validate_project_path(value, "operation.fields.destinationPath") {
                fail!(project_root, verification, "destination-invalid", error);
            }
            value
        }
        None => fail!(
            project_root,
            verification,
            "destination-invalid",
            "split_owner_items requires operation.fields.destinationPath".to_string()
        ),
    };
    if !destination_path.ends_with(".rs") {
        fail!(
            project_root,
            verification,
            "destination-invalid",
            "operation.fields.destinationPath must point to a Rust .rs file".to_string()
        );
    }
    let module_name = operation_string_field(operation, "moduleName")
        .map(str::to_string)
        .or_else(|| module_name_from_destination(destination_path));
    let Some(module_name) = module_name else {
        fail!(
            project_root,
            verification,
            "destination-invalid",
            "split_owner_items requires operation.fields.moduleName or a file-stem destination"
                .to_string()
        );
    };
    if !is_rust_identifier(&module_name) {
        fail!(
            project_root,
            verification,
            "destination-invalid",
            format!("operation.fields.moduleName is not a Rust identifier: {module_name}")
        );
    }
    let module_visibility = operation_string_field(operation, "moduleVisibility").unwrap_or("");
    if !matches!(module_visibility, "" | "pub" | "pub(crate)" | "pub(super)") {
        fail!(
            project_root,
            verification,
            "destination-invalid",
            "operation.fields.moduleVisibility must be pub, pub(crate), pub(super), or omitted"
                .to_string()
        );
    }
    verification.push("destination-validated");

    let max_edits = operation_max_edits(operation);
    let project_root = match project_root.canonicalize() {
        Ok(path) => path,
        Err(error) => fail!(
            project_root,
            verification,
            "project-root-invalid",
            format!("failed to resolve project root: {error}")
        ),
    };
    verification.push("project-root-resolved");

    let source_path = match project_root.join(&read.path).canonicalize() {
        Ok(path) if path.starts_with(&project_root) => path,
        Ok(_) => fail!(
            &project_root,
            verification,
            "target-outside-project",
            format!("target path escapes project root: {}", read.path)
        ),
        Err(error) => fail!(
            &project_root,
            verification,
            "source-read-error",
            format!("failed to resolve target path {}: {error}", read.path)
        ),
    };
    let destination_abs = project_root.join(destination_path);
    if destination_abs == source_path {
        fail!(
            &project_root,
            verification,
            "destination-invalid",
            "split_owner_items destination must differ from the owner path".to_string()
        );
    }
    if destination_abs.exists() {
        fail!(
            &project_root,
            verification,
            "destination-exists",
            format!("split_owner_items destination already exists: {destination_path}")
        );
    }
    let Some(destination_parent) = destination_abs.parent() else {
        fail!(
            &project_root,
            verification,
            "destination-invalid",
            format!("destination has no parent directory: {destination_path}")
        );
    };
    if !destination_parent.is_dir() {
        fail!(
            &project_root,
            verification,
            "destination-invalid",
            format!("destination parent directory does not exist: {destination_path}")
        );
    }
    verification.push("target-path-resolved");

    let source = match fs::read_to_string(&source_path) {
        Ok(source) => source,
        Err(error) => fail!(
            &project_root,
            verification,
            "source-read-error",
            format!("failed to read {}: {error}", read.path)
        ),
    };
    let byte_range = match byte_range_for_line_range(&source, read.start_line, read.end_line) {
        Ok(range) => range,
        Err(error) => fail!(&project_root, verification, "target-range-invalid", error),
    };
    let selected_source = &source[byte_range.clone()];
    let selected_items = match parse_selected_rust_items(selected_source) {
        Ok(items) => items,
        Err(error) => fail!(
            &project_root,
            verification,
            "target-item-parse-error",
            error
        ),
    };
    let estimated_edits = selected_items.len().saturating_add(1);
    if estimated_edits as u64 > max_edits {
        fail!(
            &project_root,
            verification,
            "target-range-invalid",
            format!(
                "split_owner_items estimated {estimated_edits} structural edits, exceeding maxEdits={max_edits}"
            )
        );
    }
    if selected_items.len() > 1
        && !operation
            .get("allowLargeMechanicalEdit")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        fail!(
            &project_root,
            verification,
            "invalid-packet",
            "split_owner_items multi-item ranges require allowLargeMechanicalEdit=true".to_string()
        );
    }
    if let Err(error) = validate_selected_item_identity(target, &selected_items) {
        fail!(&project_root, verification, "target-item-mismatch", error);
    }
    verification.push("target-items-parsed");

    if let Some(expected) = operation.get("expectedSnippet").and_then(Value::as_str) {
        if !selected_source.contains(expected) {
            fail!(
                &project_root,
                verification,
                "target-preimage-mismatch",
                "target source did not contain operation.expectedSnippet".to_string()
            );
        }
        verification.push("expected-snippet-matched");
    }

    let destination_source = normalized_split_destination_source(selected_source);
    if let Err(error) = crate::parser::parse_rust_source_syntax(&destination_source) {
        fail!(
            &project_root,
            verification,
            "file-reparse-failed",
            format!("destination module did not parse as Rust source: {error}")
        );
    }

    let mut owner_source = String::with_capacity(source.len() + module_name.len() + 8);
    owner_source.push_str(&source[..byte_range.start]);
    owner_source.push_str(&source[byte_range.end..]);
    insert_module_declaration(&mut owner_source, &module_name, module_visibility);
    if let Err(error) = crate::parser::parse_rust_source_syntax(&owner_source) {
        fail!(
            &project_root,
            verification,
            "file-reparse-failed",
            format!("owner module did not parse after split_owner_items: {error}")
        );
    }
    verification.push("file-reparsed");

    let formatted_owner = match format_candidate_source(&source_path, &owner_source) {
        Ok(formatted) => formatted,
        Err((kind, error)) => fail!(
            &project_root,
            verification,
            formatter_failure_kind(kind),
            error
        ),
    };
    let formatted_destination = match format_candidate_source(&destination_abs, &destination_source)
    {
        Ok(formatted) => formatted,
        Err((kind, error)) => fail!(
            &project_root,
            verification,
            formatter_failure_kind(kind),
            error
        ),
    };
    verification.push("formatter-output-reparsed");

    if mode.writes_files() {
        if let Err(error) = write_split_owner_files(
            &source_path,
            &formatted_owner,
            &destination_abs,
            &formatted_destination,
        ) {
            fail!(
                &project_root,
                verification,
                "source-write-error",
                format!("failed to commit split_owner_items files: {error}")
            )
        }
        verification.push("source-written");
    }

    let destination_end_line = formatted_destination.lines().count().max(1);
    split_owner_items_success_receipt(SplitOwnerItemsSuccessReceipt {
        mode,
        packet,
        project_root: &project_root,
        read: &read,
        destination_path,
        destination_end_line,
        item_count: selected_items.len(),
        estimated_edits,
        max_edits,
        source_bytes_read_local: source.len(),
        prompt_bytes_avoided: selected_source.len(),
        verification,
    })
}

fn success_receipt(
    mode: AstPatchMode,
    packet: Option<&Value>,
    project_root: &Path,
    read: &SourceRead,
    operation: &Value,
    verification: Vec<&'static str>,
) -> Value {
    json!({
        "schemaId": RECEIPT_SCHEMA_ID,
        "schemaVersion": "1",
        "protocolId": AST_PATCH_PROTOCOL_ID,
        "protocolVersion": "1",
        "status": mode.success_status(),
        "mode": mode.as_str(),
        "capability": mode.capability(),
        "mutationAvailable": mode.mutation_available(),
        "languageId": "rust",
        "target": receipt_target(packet),
        "operation": "replace_item",
        "supportedOperations": SUPPORTED_OPERATIONS,
        "mechanicalEditPlan": {
            "kind": mode.mechanical_plan_kind(),
            "operation": "replace_item",
            "targetRead": read.raw,
            "estimatedEdits": 1,
            "maxEdits": operation_max_edits(operation),
            "safeForLargeChange": false,
            "mutationAvailable": mode.mutation_available(),
            "requiresCodexApplyPatch": false,
            "changedRanges": [read.raw],
            "notes": [
                "Rust provider parsed the replacement as syn::Item",
                "Rust provider reparsed the full file after replacement"
            ]
        },
        "verification": verification,
        "failureKind": null,
        "failures": [],
        "next": success_next_guidance(mode, project_root)
    })
}

struct SplitOwnerItemsSuccessReceipt<'a> {
    mode: AstPatchMode,
    packet: &'a Value,
    project_root: &'a Path,
    read: &'a SourceRead,
    destination_path: &'a str,
    destination_end_line: usize,
    item_count: usize,
    estimated_edits: usize,
    max_edits: u64,
    source_bytes_read_local: usize,
    prompt_bytes_avoided: usize,
    verification: Vec<&'static str>,
}

fn split_owner_items_success_receipt(input: SplitOwnerItemsSuccessReceipt<'_>) -> Value {
    let destination_read = format!(
        "{}:1:{}",
        input.destination_path, input.destination_end_line
    );
    json!({
        "schemaId": RECEIPT_SCHEMA_ID,
        "schemaVersion": "1",
        "protocolId": AST_PATCH_PROTOCOL_ID,
        "protocolVersion": "1",
        "status": input.mode.success_status(),
        "mode": input.mode.as_str(),
        "capability": input.mode.capability(),
        "mutationAvailable": input.mode.mutation_available(),
        "mutationSource": "provider-native",
        "snippetRequired": false,
        "codeInPrompt": false,
        "languageId": "rust",
        "target": receipt_target(Some(input.packet)),
        "operation": "split_owner_items",
        "supportedOperations": SUPPORTED_OPERATIONS,
        "mechanicalEditPlan": {
            "kind": input.mode.mechanical_plan_kind(),
            "operation": "split_owner_items",
            "targetRead": input.read.raw,
            "estimatedEdits": input.estimated_edits,
            "maxEdits": input.max_edits,
            "safeForLargeChange": input.item_count > 1,
            "mutationAvailable": input.mode.mutation_available(),
            "requiresCodexApplyPatch": false,
            "mutationSource": "provider-native",
            "snippetRequired": false,
            "codeInPrompt": false,
            "changedPaths": [input.read.path, input.destination_path],
            "sourceBytesReadLocal": input.source_bytes_read_local,
            "promptBytesAvoided": input.prompt_bytes_avoided,
            "changedRanges": [input.read.raw, destination_read],
            "notes": [
                "Rust provider moved parser-selected top-level items without an agent-authored source hunk",
                "Rust provider reparsed and rustfmt-normalized both owner and destination modules"
            ]
        },
        "verification": input.verification,
        "failureKind": null,
        "failures": [],
        "next": success_next_guidance(input.mode, input.project_root)
    })
}

fn failure_receipt(
    mode: AstPatchMode,
    packet: Option<&Value>,
    project_root: &Path,
    verification: Vec<&'static str>,
    failure_kind: &str,
    failure: String,
    include_supported_operations: bool,
) -> Value {
    json!({
        "schemaId": RECEIPT_SCHEMA_ID,
        "schemaVersion": "1",
        "protocolId": AST_PATCH_PROTOCOL_ID,
        "protocolVersion": "1",
        "status": "failed",
        "mode": mode.as_str(),
        "capability": mode.capability(),
        "mutationAvailable": false,
        "mutationSource": receipt_mutation_source(packet),
        "snippetRequired": receipt_snippet_required(packet),
        "codeInPrompt": receipt_code_in_prompt(packet),
        "languageId": "rust",
        "target": receipt_target(packet),
        "operation": receipt_operation(packet),
        "supportedOperations": if include_supported_operations {
            json!(SUPPORTED_OPERATIONS)
        } else {
            json!([])
        },
        "mechanicalEditPlan": null,
        "verification": verification,
        "failureKind": failure_kind,
        "failures": [failure],
        "next": failure_next_guidance(project_root, packet)
    })
}

fn receipt_mutation_source(packet: Option<&Value>) -> Value {
    packet
        .and_then(|packet| packet.get("operation"))
        .and_then(|operation| operation.get("mutationSource"))
        .and_then(Value::as_str)
        .filter(|value| {
            matches!(
                *value,
                "provider-native" | "agent-snippet" | "codex-text-fallback"
            )
        })
        .map(|value| json!(value))
        .unwrap_or(Value::Null)
}

fn receipt_snippet_required(packet: Option<&Value>) -> Value {
    packet
        .and_then(|packet| packet.get("operation"))
        .and_then(|operation| operation.get("snippetRequired"))
        .and_then(Value::as_bool)
        .map(|value| json!(value))
        .unwrap_or(Value::Null)
}

fn receipt_code_in_prompt(packet: Option<&Value>) -> Value {
    packet
        .and_then(|packet| packet.get("operation"))
        .and_then(|operation| operation.get("codeInPrompt"))
        .and_then(Value::as_bool)
        .map(|value| json!(value))
        .unwrap_or(Value::Null)
}

fn receipt_target(packet: Option<&Value>) -> Value {
    let owner_path = packet
        .and_then(|packet| packet.get("target"))
        .and_then(|target| target.get("ownerPath"))
        .and_then(Value::as_str)
        .filter(|value| validate_project_path(value, "target.ownerPath").is_ok());
    let locator = packet
        .and_then(|packet| packet.get("target"))
        .and_then(|target| target.get("locator"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());
    let read = packet
        .and_then(|packet| packet.get("target"))
        .and_then(|target| target.get("read"))
        .and_then(Value::as_str)
        .filter(|value| SourceRead::parse(value).is_ok());
    json!({
        "ownerPath": owner_path,
        "locator": locator,
        "read": read
    })
}

fn receipt_operation(packet: Option<&Value>) -> Value {
    packet
        .and_then(|packet| packet.get("operation"))
        .and_then(|operation| operation.get("op"))
        .and_then(Value::as_str)
        .filter(|operation| is_schema_operation(operation))
        .map(|operation| json!(operation))
        .unwrap_or(Value::Null)
}

fn success_next_guidance(mode: AstPatchMode, project_root: &Path) -> String {
    let project_root = project_root.display();
    match mode {
        AstPatchMode::DryRun => format!(
            "provider dry-run verified AST patch; apply with `asp rust ast-patch apply --packet semantic-ast-patch.json {project_root}`; check `asp rust check --changed {project_root}`"
        ),
        AstPatchMode::Apply => {
            format!("provider apply completed; check `asp rust check --changed {project_root}`")
        }
    }
}

fn failure_next_guidance(project_root: &Path, packet: Option<&Value>) -> String {
    let owner = packet
        .and_then(|packet| packet.get("target"))
        .and_then(|target| target.get("ownerPath"))
        .and_then(Value::as_str)
        .unwrap_or("<owner-path>");
    let read = packet
        .and_then(|packet| packet.get("target"))
        .and_then(|target| target.get("read"))
        .and_then(Value::as_str)
        .unwrap_or("<path:start:end>");
    let project_root = project_root.display();
    format!(
        "rust provider supports provider-native split_owner_items and snippet-verified replace_item; inspect exact source with `asp rust query --selector {read} --workspace {project_root} --projection source`; build an AST packet with `asp ast-patch template --language rust --owner {owner} --read {read} --op split_owner_items --field destinationPath=<new-module.rs> --field moduleName=<module_name> > semantic-ast-patch.json`; verify with `asp rust ast-patch dry-run --packet semantic-ast-patch.json {project_root}`; apply natively with `asp rust ast-patch apply --packet semantic-ast-patch.json {project_root}`; check `asp rust check --changed {project_root}`"
    )
}
