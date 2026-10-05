use std::ops::Range;

use crate::document::EditorError;
use crate::snippet_normalization::own_cost;
use crate::snippet_syntax::{Index, MAX_STACK_DEPTH, Marker, ParseLimits, Transform};

pub struct VariableContext<'a> {
    pub name: &'a str,
    pub preceding_text_line: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceholderSpan {
    pub index: Index,
    pub bytes: Range<usize>,
    pub marker_path: Vec<usize>,
    pub enclosing: Vec<usize>,
}

pub struct Expansion {
    pub markers: Vec<Marker>,
    pub text: String,
    pub placeholders: Vec<PlaceholderSpan>,
}

struct Builder {
    limits: ParseLimits,
    allocated_bytes: usize,
    allocated_markers: usize,
    text: String,
    placeholders: Vec<PlaceholderSpan>,
    preceding_text_line: Option<String>,
    path: Vec<usize>,
    enclosing: Vec<usize>,
}

pub fn expand(
    markers: &[Marker],
    limits: ParseLimits,
    mut resolve: impl FnMut(VariableContext<'_>) -> Result<Option<String>, EditorError>,
    mut evaluate: impl FnMut(&Transform, &str) -> Result<String, EditorError>,
) -> Result<Expansion, EditorError> {
    if limits.max_nesting > MAX_STACK_DEPTH || limits.max_markers == 0 {
        return Err(EditorError::Capacity);
    }
    let mut builder = Builder {
        limits,
        allocated_bytes: 0,
        allocated_markers: 0,
        text: String::new(),
        placeholders: Vec::new(),
        preceding_text_line: None,
        path: Vec::new(),
        enclosing: Vec::new(),
    };
    let markers = builder.children(markers, 0, &mut resolve, &mut evaluate)?;
    Ok(Expansion {
        markers,
        text: builder.text,
        placeholders: builder.placeholders,
    })
}

impl Builder {
    fn charge(&mut self, marker: &Marker) -> Result<(), EditorError> {
        let (bytes, markers) = own_cost(marker)?;
        let bytes = self
            .allocated_bytes
            .checked_add(bytes)
            .ok_or(EditorError::Capacity)?;
        let markers = self
            .allocated_markers
            .checked_add(markers)
            .ok_or(EditorError::Capacity)?;
        if bytes > self.limits.max_bytes || markers > self.limits.max_markers {
            return Err(EditorError::Capacity);
        }
        self.allocated_bytes = bytes;
        self.allocated_markers = markers;
        Ok(())
    }

    fn append(&mut self, text: &str) -> Result<(), EditorError> {
        let length = self
            .text
            .len()
            .checked_add(text.len())
            .ok_or(EditorError::Capacity)?;
        if length > self.limits.max_bytes {
            return Err(EditorError::Capacity);
        }
        self.text.push_str(text);
        Ok(())
    }

    fn children(
        &mut self,
        markers: &[Marker],
        depth: usize,
        resolve: &mut impl FnMut(VariableContext<'_>) -> Result<Option<String>, EditorError>,
        evaluate: &mut impl FnMut(&Transform, &str) -> Result<String, EditorError>,
    ) -> Result<Vec<Marker>, EditorError> {
        if depth > self.limits.max_nesting {
            return Err(EditorError::Capacity);
        }
        let mut children = Vec::new();
        for (index, marker) in markers.iter().enumerate() {
            self.path.push(index);
            let child = self.marker(marker, depth, resolve, evaluate)?;
            self.path.pop();
            children.push(child);
        }
        Ok(children)
    }

    fn marker(
        &mut self,
        marker: &Marker,
        depth: usize,
        resolve: &mut impl FnMut(VariableContext<'_>) -> Result<Option<String>, EditorError>,
        evaluate: &mut impl FnMut(&Transform, &str) -> Result<String, EditorError>,
    ) -> Result<Marker, EditorError> {
        self.charge(marker)?;
        match marker {
            Marker::Text(text) => {
                self.append(text)?;
                self.preceding_text_line = Some(text.rsplit(['\r', '\n']).next().unwrap().into());
                Ok(Marker::Text(text.clone()))
            }
            Marker::Variable {
                name,
                children,
                transform,
            } => {
                let value = resolve(VariableContext {
                    name,
                    preceding_text_line: self.preceding_text_line.as_deref(),
                })?;
                if value
                    .as_ref()
                    .is_some_and(|value| value.len() > self.limits.max_bytes)
                {
                    return Err(EditorError::Capacity);
                }
                let value = match transform {
                    Some(transform) => Some(evaluate(transform, value.as_deref().unwrap_or(""))?),
                    None => value,
                };
                let children = if let Some(value) = value {
                    if value.len() > self.limits.max_bytes {
                        return Err(EditorError::Capacity);
                    }
                    self.children(&[Marker::Text(value)], depth + 1, resolve, evaluate)?
                } else {
                    self.children(children, depth + 1, resolve, evaluate)?
                };
                Ok(Marker::Variable {
                    name: name.clone(),
                    children,
                    transform: transform.clone(),
                })
            }
            Marker::Placeholder {
                index,
                children,
                choices,
                transform,
            } => {
                let placeholder = self.placeholders.len();
                self.placeholders.push(PlaceholderSpan {
                    index: *index,
                    bytes: self.text.len()..self.text.len(),
                    marker_path: self.path.clone(),
                    enclosing: self.enclosing.iter().copied().rev().collect(),
                });
                self.enclosing.push(placeholder);
                let children = if let Some(choices) = choices {
                    if !children.is_empty() {
                        return Err(EditorError::InvalidBoundary);
                    }
                    self.append(choices.first().ok_or(EditorError::InvalidBoundary)?)?;
                    Vec::new()
                } else {
                    self.children(children, depth + 1, resolve, evaluate)?
                };
                self.enclosing.pop();
                self.placeholders[placeholder].bytes.end = self.text.len();
                Ok(Marker::Placeholder {
                    index: *index,
                    children,
                    choices: choices.clone(),
                    transform: transform.clone(),
                })
            }
        }
    }
}
