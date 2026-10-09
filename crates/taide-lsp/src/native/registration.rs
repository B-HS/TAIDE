use std::collections::{BTreeMap, BTreeSet};

use lsp_types::{DocumentFilter, Registration, RegistrationParams, UnregistrationParams};
use serde_json::{json, Value};

use super::{capabilities, selector::Filter, DocumentMirror, Failure};

pub const MAX_DYNAMIC_REGISTRATIONS: usize = 256;
pub const MAX_REGISTRATION_BYTES: usize = 1024 * 1024;
const MAX_SELECTOR_FILTERS: usize = 256;

struct Registered {
    method: String,
    capabilities: Value,
    selector: Selector,
    bytes: usize,
}

enum Selector {
    Workspace,
    Client,
    Filters(Vec<Filter>),
}

impl Selector {
    fn matches(&self, document: Option<&DocumentMirror>) -> bool {
        match (self, document) {
            (Self::Workspace, _) => true,
            (_, None) => false,
            (Self::Client, Some(_)) => true,
            (Self::Filters(filters), Some(document)) => {
                filters.iter().any(|filter| filter.matches(document))
            }
        }
    }

    fn filter_count(&self) -> usize {
        match self {
            Self::Filters(filters) => filters.len(),
            _ => 0,
        }
    }
}

#[derive(Default)]
pub(crate) struct Registry {
    entries: BTreeMap<String, Registered>,
    bytes: usize,
    filters: usize,
    revision: u64,
}

pub(crate) fn feature(method: &str) -> Option<(&str, &str, &str, &str)> {
    let (method, field, provider, domain) = match method {
        "textDocument/hover" => (
            "textDocument/hover",
            "hover",
            "hoverProvider",
            "textDocument",
        ),
        "textDocument/definition" => (
            "textDocument/definition",
            "definition",
            "definitionProvider",
            "textDocument",
        ),
        "textDocument/references" => (
            "textDocument/references",
            "references",
            "referencesProvider",
            "textDocument",
        ),
        "textDocument/rename" | "textDocument/prepareRename" => (
            "textDocument/rename",
            "rename",
            "renameProvider",
            "textDocument",
        ),
        "textDocument/formatting" => (
            "textDocument/formatting",
            "formatting",
            "documentFormattingProvider",
            "textDocument",
        ),
        "textDocument/rangeFormatting" => (
            "textDocument/rangeFormatting",
            "rangeFormatting",
            "documentRangeFormattingProvider",
            "textDocument",
        ),
        "textDocument/inlayHint" => (
            "textDocument/inlayHint",
            "inlayHint",
            "inlayHintProvider",
            "textDocument",
        ),
        "textDocument/documentSymbol" => (
            "textDocument/documentSymbol",
            "documentSymbol",
            "documentSymbolProvider",
            "textDocument",
        ),
        "textDocument/documentHighlight" => (
            "textDocument/documentHighlight",
            "documentHighlight",
            "documentHighlightProvider",
            "textDocument",
        ),
        "textDocument/selectionRange" => (
            "textDocument/selectionRange",
            "selectionRange",
            "selectionRangeProvider",
            "textDocument",
        ),
        "textDocument/codeAction" | "codeAction/resolve" => (
            "textDocument/codeAction",
            "codeAction",
            "codeActionProvider",
            "textDocument",
        ),
        "textDocument/foldingRange" => (
            "textDocument/foldingRange",
            "foldingRange",
            "foldingRangeProvider",
            "textDocument",
        ),
        "textDocument/implementation" => (
            "textDocument/implementation",
            "implementation",
            "implementationProvider",
            "textDocument",
        ),
        "textDocument/typeDefinition" => (
            "textDocument/typeDefinition",
            "typeDefinition",
            "typeDefinitionProvider",
            "textDocument",
        ),
        "textDocument/declaration" => (
            "textDocument/declaration",
            "declaration",
            "declarationProvider",
            "textDocument",
        ),
        "textDocument/completion" => (
            "textDocument/completion",
            "completion",
            "completionProvider",
            "textDocument",
        ),
        "textDocument/onTypeFormatting" => (
            "textDocument/onTypeFormatting",
            "onTypeFormatting",
            "documentOnTypeFormattingProvider",
            "textDocument",
        ),
        "textDocument/signatureHelp" => (
            "textDocument/signatureHelp",
            "signatureHelp",
            "signatureHelpProvider",
            "textDocument",
        ),
        "textDocument/diagnostic" => (
            "textDocument/diagnostic",
            "diagnostic",
            "diagnosticProvider",
            "textDocument",
        ),
        "textDocument/codeLens" | "codeLens/resolve" => (
            "textDocument/codeLens",
            "codeLens",
            "codeLensProvider",
            "textDocument",
        ),
        "textDocument/semanticTokens"
        | "textDocument/semanticTokens/full"
        | "textDocument/semanticTokens/full/delta" => (
            "textDocument/semanticTokens",
            "semanticTokens",
            "semanticTokensProvider",
            "textDocument",
        ),
        "workspace/symbol" => (
            "workspace/symbol",
            "symbol",
            "workspaceSymbolProvider",
            "workspace",
        ),
        "workspace/executeCommand" => (
            "workspace/executeCommand",
            "executeCommand",
            "executeCommandProvider",
            "workspace",
        ),
        _ => return None,
    };
    Some((method, field, provider, domain))
}

macro_rules! validate_options {
    ($value:expr, $type:ty) => {
        serde_json::from_value::<$type>($value.clone()).map_err(|_| Failure::MalformedResponse)?
    };
}

impl Registry {
    pub(crate) fn count(&self) -> usize {
        self.entries.len()
    }
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn signature_options(
        &self,
        document: &DocumentMirror,
    ) -> Vec<lsp_types::SignatureHelpOptions> {
        self.entries
            .values()
            .filter(|entry| {
                entry.method == "textDocument/signatureHelp"
                    && entry.selector.matches(Some(document))
            })
            .filter_map(|entry| {
                serde_json::from_value(entry.capabilities["signatureHelpProvider"].clone()).ok()
            })
            .collect()
    }

    pub(crate) fn supports(
        &self,
        method: &str,
        document: Option<&DocumentMirror>,
        command: Option<&str>,
        is_query: bool,
    ) -> bool {
        let Some((base, _, _, _)) = feature(method) else {
            return false;
        };
        self.entries.values().any(|entry| {
            entry.method == base
                && capabilities::supports(&entry.capabilities, method)
                && (is_query || entry.selector.matches(document))
                && (method != "workspace/executeCommand"
                    || is_query
                    || command.is_some_and(|command| {
                        entry.capabilities["executeCommandProvider"]["commands"]
                            .as_array()
                            .is_some_and(|commands| {
                                commands.iter().any(|value| value.as_str() == Some(command))
                            })
                    }))
        })
    }

    pub(crate) fn register(
        &mut self,
        params: RegistrationParams,
        client: &Value,
    ) -> Result<(), Failure> {
        if self
            .entries
            .len()
            .checked_add(params.registrations.len())
            .is_none_or(|count| count > MAX_DYNAMIC_REGISTRATIONS)
        {
            return Err(Failure::Capacity);
        }
        let mut staged = BTreeMap::new();
        let mut bytes = self.bytes;
        let mut filters = self.filters;
        for registration in params.registrations {
            let Registration {
                id,
                method,
                register_options,
            } = registration;
            let Some((base, field, provider, domain)) = feature(&method) else {
                return Err(Failure::UnsupportedCapability);
            };
            if base != method || client[domain][field]["dynamicRegistration"] != true {
                return Err(Failure::UnsupportedCapability);
            }
            if id.is_empty() || self.entries.contains_key(&id) || staged.contains_key(&id) {
                return Err(Failure::MalformedResponse);
            }
            let options = register_options.unwrap_or_else(|| json!({}));
            if !options.is_object() {
                return Err(Failure::MalformedResponse);
            }
            let capabilities = json!({provider:options});
            let selector = if domain == "textDocument" {
                let selector = options
                    .get("documentSelector")
                    .ok_or(Failure::MalformedResponse)?;
                if selector.is_null() {
                    Selector::Client
                } else {
                    let values = selector.as_array().ok_or(Failure::MalformedResponse)?;
                    if filters
                        .checked_add(values.len())
                        .is_none_or(|count| count > MAX_SELECTOR_FILTERS)
                    {
                        return Err(Failure::Capacity);
                    }
                    let mut compiled = Vec::new();
                    for value in values {
                        if !value.is_object()
                            || ["language", "scheme", "pattern"]
                                .iter()
                                .any(|field| value.get(field).is_some_and(Value::is_null))
                        {
                            return Err(Failure::MalformedResponse);
                        }
                        compiled.push(Filter::compile(
                            serde_json::from_value::<DocumentFilter>(value.clone())
                                .map_err(|_| Failure::MalformedResponse)?,
                        )?);
                    }
                    Selector::Filters(compiled)
                }
            } else {
                Selector::Workspace
            };
            match base {
                "textDocument/completion" => {
                    validate_options!(options, lsp_types::CompletionRegistrationOptions);
                }
                "textDocument/signatureHelp" => {
                    validate_options!(options, lsp_types::SignatureHelpRegistrationOptions);
                    validate_options!(options, lsp_types::SignatureHelpOptions);
                }
                "textDocument/onTypeFormatting" => {
                    validate_options!(
                        options,
                        lsp_types::DocumentOnTypeFormattingRegistrationOptions
                    );
                }
                "textDocument/semanticTokens" => {
                    validate_options!(options, lsp_types::SemanticTokensRegistrationOptions);
                }
                "textDocument/diagnostic" => {
                    validate_options!(options, lsp_types::DiagnosticRegistrationOptions);
                }
                "workspace/executeCommand" => {
                    validate_options!(options, lsp_types::ExecuteCommandOptions);
                }
                _ => {}
            }
            for field in ["prepareProvider", "resolveProvider", "workDoneProgress"] {
                if options.get(field).is_some_and(|value| !value.is_boolean()) {
                    return Err(Failure::MalformedResponse);
                }
            }
            let length =
                serde_json::to_vec(&json!({"id":id,"method":method,"registerOptions":options}))
                    .map_err(|_| Failure::MalformedResponse)?
                    .len();
            bytes = bytes
                .checked_add(length)
                .filter(|bytes| *bytes <= MAX_REGISTRATION_BYTES)
                .ok_or(Failure::Capacity)?;
            filters = filters
                .checked_add(selector.filter_count())
                .filter(|count| *count <= MAX_SELECTOR_FILTERS)
                .ok_or(Failure::Capacity)?;
            staged.insert(
                id,
                Registered {
                    method,
                    capabilities,
                    selector,
                    bytes: length,
                },
            );
        }
        if staged.is_empty() {
            return Ok(());
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(Failure::CounterOverflow)?;
        self.entries.extend(staged);
        self.bytes = bytes;
        self.filters = filters;
        self.revision = revision;
        Ok(())
    }

    pub(crate) fn unregister(&mut self, params: UnregistrationParams) -> Result<(), Failure> {
        let mut ids = BTreeSet::new();
        for registration in params.unregisterations {
            if !ids.insert(registration.id.clone())
                || self
                    .entries
                    .get(&registration.id)
                    .is_none_or(|entry| entry.method != registration.method)
            {
                return Err(Failure::MalformedResponse);
            }
        }
        if ids.is_empty() {
            return Ok(());
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(Failure::CounterOverflow)?;
        for id in ids {
            let entry = self.entries.remove(&id).ok_or(Failure::MalformedResponse)?;
            self.bytes -= entry.bytes;
            self.filters -= entry.selector.filter_count();
        }
        self.revision = revision;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_동적등록은_잘못된_trigger_형식을_원자적으로_거절한다() {
        let client = json!({"textDocument":{"signatureHelp":{"dynamicRegistration":true}}});
        for field in ["triggerCharacters", "retriggerCharacters"] {
            for value in [json!(["(", 1]), json!(1), json!({})] {
                let mut registry = Registry::default();
                let mut options = json!({"documentSelector":null});
                options[field] = value;
                let params = serde_json::from_value(json!({"registrations":[{
                    "id":"signature", "method":"textDocument/signatureHelp", "registerOptions":options
                }]})).unwrap();
                assert_eq!(
                    registry.register(params, &client),
                    Err(Failure::MalformedResponse)
                );
                assert_eq!(registry.count(), 0);
                assert_eq!(registry.revision(), 0);
            }
        }
    }
}
