use crate::diagnostics::Severity;
use crate::document::EditorError;
use crate::json_parser::Node;
use crate::json_schema_associations::Pattern;
use crate::json_schema_scalar::Validator;
use crate::json_value::{Arena, Value};

const CASE_INSENSITIVE_PREFIX: &[u16] = &[0x28, 0x3f, 0x69, 0x29];
const COLON: u16 = 0x3a;
const SLASH: u16 = 0x2f;
const QUESTION: u16 = 0x3f;
const HASH: u16 = 0x23;

include!("json-schema-formats.rs");

pub trait PatternCompiler {
    fn compile(
        &self,
        source: &[u16],
        ignore_case: bool,
        unicode: bool,
    ) -> Result<Option<Box<dyn Pattern>>, EditorError>;
}

pub(crate) fn compile_extended(
    compiler: &dyn PatternCompiler,
    pattern: &[u16],
    remaining: &mut usize,
) -> Result<Option<Box<dyn Pattern>>, EditorError> {
    let (source, ignore_case) = if let Some(source) = pattern.strip_prefix(CASE_INSENSITIVE_PREFIX)
    {
        (source, true)
    } else {
        (pattern, false)
    };
    *remaining = remaining
        .checked_sub(source.len() + 1)
        .ok_or(EditorError::Capacity)?;
    if let Some(regex) = compiler.compile(source, ignore_case, true)? {
        return Ok(Some(regex));
    }
    *remaining = remaining
        .checked_sub(source.len() + 1)
        .ok_or(EditorError::Capacity)?;
    compiler.compile(source, ignore_case, false)
}

impl Validator<'_> {
    pub(crate) fn string_patterns(
        &mut self,
        node: &Node,
        value: &[u16],
        schema: &Arena,
        compiler: &dyn PatternCompiler,
        severity: Severity,
    ) -> Result<(), EditorError> {
        if let Some(index) = self.field("pattern")? {
            if let Some(Value::String(pattern)) = schema.get(index) {
                self.spend(value.len())?;
                let regex = compile_extended(compiler, pattern, &mut self.remaining)?;
                if !regex.is_some_and(|regex| regex.is_match(value)) {
                    let message = if let Some(message) = self.pattern_message()? {
                        message
                    } else {
                        let mut message = Vec::new();
                        self.literal(&mut message, "String does not match the pattern of \"")?;
                        self.append(&mut message, pattern)?;
                        self.literal(&mut message, "\".")?;
                        message
                    };
                    self.add(node, message, severity, None)?;
                }
            }
        }
        let Some(index) = self.field("format")? else {
            return Ok(());
        };
        let Some(Value::String(format)) = schema.get(index) else {
            return Ok(());
        };
        self.spend(format.len() + value.len() + 1)?;
        let is_uri = format.iter().copied().eq("uri".encode_utf16());
        let is_reference = format.iter().copied().eq("uri-reference".encode_utf16());
        if is_uri || is_reference {
            let error = if value.is_empty() {
                Some("URI expected.")
            } else if is_uri
                && !value
                    .iter()
                    .position(|unit| matches!(*unit, COLON | SLASH | QUESTION | HASH))
                    .is_some_and(|index| index > 0 && value[index] == COLON)
            {
                Some("URI with a scheme is expected.")
            } else {
                None
            };
            if let Some(error) = error {
                let message = if let Some(message) = self.pattern_message()? {
                    message
                } else {
                    let mut message = Vec::new();
                    self.literal(&mut message, "String is not a URI: ")?;
                    self.literal(&mut message, error)?;
                    message
                };
                self.add(node, message, severity, None)?;
            }
            return Ok(());
        }
        let Some((_, source, message, ignore_case)) = FORMATS
            .iter()
            .find(|(name, _, _, _)| format.iter().copied().eq(name.encode_utf16()))
        else {
            return Ok(());
        };
        let source = source.encode_utf16().collect::<Vec<_>>();
        self.spend(source.len() + 1)?;
        let regex = compiler
            .compile(&source, *ignore_case, false)?
            .ok_or(EditorError::InvalidBoundary)?;
        if value.is_empty() || !regex.is_match(value) {
            let message = if let Some(message) = self.pattern_message()? {
                message
            } else {
                message.encode_utf16().collect()
            };
            self.add(node, message, severity, None)?;
        }
        Ok(())
    }

    fn pattern_message(&mut self) -> Result<Option<Vec<u16>>, EditorError> {
        if let Some(message) = self.custom("patternErrorMessage")? {
            return Ok(Some(message));
        }
        self.custom("errorMessage")
    }
}
