use regress::Regex;
use taide_native_editor::document::EditorError;
use taide_native_editor::json_schema_associations::{Pattern, PatternCompiler};

pub struct JsonSchemaPatternCompiler;

struct JsonSchemaPattern(Regex);

impl PatternCompiler for JsonSchemaPatternCompiler {
    fn compile(&self, source: &[u16]) -> Result<Box<dyn Pattern>, EditorError> {
        let regex = Regex::from_unicode(source.iter().copied().map(u32::from), "")
            .map_err(|_| EditorError::InvalidBoundary)?;
        Ok(Box::new(JsonSchemaPattern(regex)))
    }
}

impl Pattern for JsonSchemaPattern {
    fn is_match(&self, text: &[u16]) -> bool {
        self.0.find_from_ucs2(text, 0).next().is_some()
    }
}
