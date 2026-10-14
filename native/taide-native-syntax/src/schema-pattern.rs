use regress::Regex;
use taide_native_editor::document::EditorError;
use taide_native_editor::json_schema_associations::{Pattern, PatternCompiler};

pub struct JsonSchemaPatternCompiler;

pub struct JsonSchemaValidationPatternCompiler;

struct JsonSchemaPattern(Regex);

struct JsonSchemaValidationPattern {
    regex: Regex,
    unicode: bool,
}

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

impl taide_native_editor::json_schema_string::PatternCompiler
    for JsonSchemaValidationPatternCompiler
{
    fn compile(
        &self,
        source: &[u16],
        ignore_case: bool,
        unicode: bool,
    ) -> Result<Option<Box<dyn Pattern>>, EditorError> {
        let flags = match (ignore_case, unicode) {
            (true, true) => "iu",
            (true, false) => "i",
            (false, true) => "u",
            (false, false) => "",
        };
        let regex = if unicode {
            Regex::from_unicode(
                char::decode_utf16(source.iter().copied()).map(|character| match character {
                    Ok(character) => u32::from(character),
                    Err(error) => u32::from(error.unpaired_surrogate()),
                }),
                flags,
            )
        } else {
            Regex::from_unicode(source.iter().copied().map(u32::from), flags)
        };
        match regex {
            Ok(regex) => Ok(Some(Box::new(JsonSchemaValidationPattern {
                regex,
                unicode,
            }))),
            Err(_) => Ok(None),
        }
    }
}

impl Pattern for JsonSchemaValidationPattern {
    fn is_match(&self, text: &[u16]) -> bool {
        if self.unicode {
            return self.regex.find_from_utf16(text, 0).next().is_some();
        }
        self.regex.find_from_ucs2(text, 0).next().is_some()
    }
}
