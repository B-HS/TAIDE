use regress::Regex;
use taide_native_editor::find::{
    FindCaptures, FindPattern, FindPatternCompiler, FindPatternError, FindPatternOptions,
    escape_find_literal,
};

pub struct MonacoFindPatternCompiler;

struct MonacoFindPattern(Regex);

impl FindPatternCompiler for MonacoFindPatternCompiler {
    fn compile(
        &self,
        source: &str,
        options: FindPatternOptions,
    ) -> Result<Box<dyn FindPattern>, FindPatternError> {
        let source = if options.is_regex {
            source.to_string()
        } else {
            escape_find_literal(source)
        };
        let mut flags = String::from("u");
        if !options.match_case {
            flags.push('i');
        }
        if options.multiline {
            flags.push('m');
        }
        let expression = Regex::with_flags(&source, flags.as_str())
            .map_err(|error| FindPatternError(error.to_string()))?;
        Ok(Box::new(MonacoFindPattern(expression)))
    }
}

impl FindPattern for MonacoFindPattern {
    fn captures_at(
        &self,
        text: &str,
        start: usize,
    ) -> Result<Option<FindCaptures>, FindPatternError> {
        if !text.is_char_boundary(start) {
            return Err(FindPatternError("Invalid search boundary".to_string()));
        }
        Ok(self
            .0
            .find_from(text, start)
            .next()
            .map(|found| FindCaptures {
                groups: found.groups().collect(),
            }))
    }
}
