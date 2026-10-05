use serde_json::Value;

fn is_enabled(value: Option<&Value>) -> bool {
    matches!(value, Some(Value::Bool(true) | Value::Object(_)))
}

fn is_options(value: Option<&Value>) -> bool {
    value.is_some_and(Value::is_object)
}

fn is_nested_flag(capabilities: &Value, provider: &str, flag: &str) -> bool {
    capabilities
        .get(provider)
        .and_then(Value::as_object)
        .and_then(|options| options.get(flag))
        .and_then(Value::as_bool)
        == Some(true)
}

pub(crate) fn supports(capabilities: &Value, method: &str) -> bool {
    let provider = match method {
        "textDocument/hover" => "hoverProvider",
        "textDocument/definition" => "definitionProvider",
        "textDocument/references" => "referencesProvider",
        "textDocument/rename" => "renameProvider",
        "textDocument/formatting" => "documentFormattingProvider",
        "textDocument/rangeFormatting" => "documentRangeFormattingProvider",
        "textDocument/inlayHint" => "inlayHintProvider",
        "textDocument/documentSymbol" => "documentSymbolProvider",
        "textDocument/documentHighlight" => "documentHighlightProvider",
        "textDocument/selectionRange" => "selectionRangeProvider",
        "textDocument/codeAction" => "codeActionProvider",
        "textDocument/foldingRange" => "foldingRangeProvider",
        "textDocument/implementation" => "implementationProvider",
        "textDocument/typeDefinition" => "typeDefinitionProvider",
        "textDocument/declaration" => "declarationProvider",
        "workspace/symbol" => "workspaceSymbolProvider",
        "textDocument/completion" => return is_options(capabilities.get("completionProvider")),
        "textDocument/onTypeFormatting" => {
            return is_options(capabilities.get("documentOnTypeFormattingProvider"))
        }
        "textDocument/signatureHelp" => {
            return is_options(capabilities.get("signatureHelpProvider"))
        }
        "textDocument/diagnostic" => return is_options(capabilities.get("diagnosticProvider")),
        "textDocument/codeLens" => return is_options(capabilities.get("codeLensProvider")),
        "workspace/executeCommand" => {
            return is_options(capabilities.get("executeCommandProvider"))
        }
        "textDocument/prepareRename" => {
            return is_nested_flag(capabilities, "renameProvider", "prepareProvider")
        }
        "codeAction/resolve" => {
            return is_nested_flag(capabilities, "codeActionProvider", "resolveProvider")
        }
        "codeLens/resolve" => {
            return is_nested_flag(capabilities, "codeLensProvider", "resolveProvider")
        }
        "textDocument/semanticTokens/full" => {
            return is_enabled(
                capabilities
                    .get("semanticTokensProvider")
                    .and_then(|provider| provider.get("full")),
            );
        }
        "textDocument/semanticTokens/full/delta" => {
            return capabilities
                .get("semanticTokensProvider")
                .and_then(|provider| provider.get("full"))
                .and_then(Value::as_object)
                .and_then(|options| options.get("delta"))
                .and_then(Value::as_bool)
                == Some(true);
        }
        _ => return true,
    };
    is_enabled(capabilities.get(provider))
}
