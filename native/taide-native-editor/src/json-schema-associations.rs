use crate::document::EditorError;
use crate::json_schema_uri;

const SLASH: u16 = 0x2f;
const BANG: u16 = 0x21;
const STAR: u16 = 0x2a;
const BACKSLASH: u16 = 0x5c;
const GLOBSTAR_PREFIX: &[u16] = &[STAR, STAR, SLASH];

pub trait Pattern {
    fn is_match(&self, text: &[u16]) -> bool;
}

pub trait PatternCompiler {
    fn compile(&self, source: &[u16]) -> Result<Box<dyn Pattern>, EditorError>;
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub uri: json_schema_uri::Limits,
    pub max_patterns: usize,
    pub max_pattern_units: usize,
    pub max_regex_units: usize,
    pub max_uris: usize,
    pub max_uri_units: usize,
    pub max_work: usize,
}

pub struct Association {
    folder: Option<Vec<u16>>,
    patterns: Vec<(Box<dyn Pattern>, bool)>,
    uris: Vec<Vec<u16>>,
    limits: Limits,
}

fn append(target: &mut Vec<u16>, source: &[u16], max_units: usize) -> Result<(), EditorError> {
    if target
        .len()
        .checked_add(source.len())
        .ok_or(EditorError::Capacity)?
        > max_units
    {
        return Err(EditorError::Capacity);
    }
    target.extend_from_slice(source);
    Ok(())
}

pub fn glob_source(input: &[u16], max_units: usize) -> Result<Vec<u16>, EditorError> {
    let mut output = Vec::new();
    append(&mut output, &[0x5e], max_units)?;
    let mut cursor = 0;
    let mut is_group = false;
    while let Some(&unit) = input.get(cursor) {
        cursor += 1;
        match unit {
            SLASH | 0x24 | 0x5e | 0x2b | 0x2e | 0x28 | 0x29 | 0x3d | BANG | 0x7c => {
                append(&mut output, &[BACKSLASH, unit], max_units)?;
            }
            0x3f => append(&mut output, &[0x2e], max_units)?,
            0x5b | 0x5d => append(&mut output, &[unit], max_units)?,
            0x7b => {
                is_group = true;
                append(&mut output, &[0x28], max_units)?;
            }
            0x7d => {
                is_group = false;
                append(&mut output, &[0x29], max_units)?;
            }
            0x2c if is_group => append(&mut output, &[0x7c], max_units)?,
            0x2c => append(&mut output, &[BACKSLASH, unit], max_units)?,
            STAR => {
                let start = cursor - 1;
                while input.get(cursor) == Some(&STAR) {
                    cursor += 1;
                }
                let previous = start
                    .checked_sub(1)
                    .and_then(|index| input.get(index))
                    .copied();
                let next = input.get(cursor).copied();
                let is_globstar = cursor - start > 1
                    && matches!(previous, None | Some(SLASH | 0x7b | 0x2c))
                    && matches!(next, None | Some(SLASH | 0x7d | 0x2c));
                if is_globstar {
                    if next == Some(SLASH) {
                        cursor += 1;
                    } else if previous == Some(SLASH) && output.ends_with(&[BACKSLASH, SLASH]) {
                        output.truncate(output.len() - 2);
                    }
                    append(
                        &mut output,
                        &"((?:[^/]*(?:/|$))*)".encode_utf16().collect::<Vec<_>>(),
                        max_units,
                    )?;
                } else {
                    append(
                        &mut output,
                        &"([^/]*)".encode_utf16().collect::<Vec<_>>(),
                        max_units,
                    )?;
                }
            }
            _ => append(&mut output, &[unit], max_units)?,
        }
    }
    append(&mut output, &[0x24], max_units)?;
    Ok(output)
}

impl Association {
    pub fn new(
        patterns: &[Vec<u16>],
        folder: Option<&[u16]>,
        uris: &[Vec<u16>],
        compiler: &dyn PatternCompiler,
        limits: Limits,
    ) -> Result<Self, EditorError> {
        if patterns.len() > limits.max_patterns || uris.len() > limits.max_uris {
            return Err(EditorError::Capacity);
        }
        let pattern_units = patterns.iter().try_fold(0usize, |count, pattern| {
            count
                .checked_add(pattern.len())
                .ok_or(EditorError::Capacity)
        })?;
        if pattern_units > limits.max_pattern_units || pattern_units > limits.max_work {
            return Err(EditorError::Capacity);
        }
        let mut result = Self {
            folder: None,
            patterns: Vec::new(),
            uris: Vec::new(),
            limits,
        };
        let mut work = pattern_units;
        let mut regex_units = 0usize;
        for pattern in patterns {
            let is_include = pattern.first() != Some(&BANG);
            let pattern = if is_include {
                pattern.as_slice()
            } else {
                &pattern[1..]
            };
            if pattern.is_empty() {
                continue;
            }
            let pattern = pattern.strip_prefix(&[SLASH]).unwrap_or(pattern);
            let mut input = Vec::new();
            append(&mut input, GLOBSTAR_PREFIX, limits.max_regex_units)?;
            append(&mut input, pattern, limits.max_regex_units)?;
            let regex = glob_source(&input, limits.max_regex_units.saturating_sub(regex_units))?;
            regex_units = regex_units
                .checked_add(regex.len())
                .ok_or(EditorError::Capacity)?;
            work = work.checked_add(regex.len()).ok_or(EditorError::Capacity)?;
            if work > limits.max_work {
                return Err(EditorError::Capacity);
            }
            match compiler.compile(&regex) {
                Ok(pattern) => result.patterns.push((pattern, is_include)),
                Err(EditorError::InvalidBoundary) => {
                    result.patterns.clear();
                    return Ok(result);
                }
                Err(error) => return Err(error),
            }
        }
        if let Some(folder) = folder.filter(|folder| !folder.is_empty()) {
            let mut normalized = json_schema_uri::normalize_resource(folder, limits.uri)?;
            if normalized.last() != Some(&SLASH) {
                append(&mut normalized, &[SLASH], limits.max_uri_units)?;
            }
            result.folder = Some(normalized);
        }
        let mut uri_units = result.folder.as_ref().map_or(0, Vec::len);
        for uri in uris {
            let normalized = json_schema_uri::normalize_id(uri, limits.uri)?;
            uri_units = uri_units
                .checked_add(normalized.len())
                .ok_or(EditorError::Capacity)?;
            if uri_units > limits.max_uri_units {
                return Err(EditorError::Capacity);
            }
            result.uris.push(normalized);
        }
        if uri_units > limits.max_uri_units {
            return Err(EditorError::Capacity);
        }
        Ok(result)
    }

    pub fn uris(&self) -> &[Vec<u16>] {
        &self.uris
    }

    pub fn matches_resource(&self, resource: &[u16]) -> Result<bool, EditorError> {
        let normalized = json_schema_uri::normalize_resource(resource, self.limits.uri)?;
        let work = normalized
            .len()
            .checked_mul(
                self.patterns
                    .len()
                    .checked_add(1)
                    .ok_or(EditorError::Capacity)?,
            )
            .ok_or(EditorError::Capacity)?;
        if work > self.limits.max_work {
            return Err(EditorError::Capacity);
        }
        if self
            .folder
            .as_ref()
            .is_some_and(|folder| !normalized.starts_with(folder))
        {
            return Ok(false);
        }
        let mut is_match = false;
        for (pattern, is_include) in &self.patterns {
            if pattern.is_match(&normalized) {
                is_match = *is_include;
            }
        }
        Ok(is_match)
    }
}
