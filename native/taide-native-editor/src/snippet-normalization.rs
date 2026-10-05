use std::collections::{BTreeMap, BTreeSet};

use crate::document::EditorError;
use crate::snippet_syntax::{FormatPart, Index, Marker, ParseLimits, Transform, parse};

#[derive(Clone, Copy)]
pub struct FinalTabstopOptions {
    pub insert: bool,
    pub enforce: bool,
}

#[derive(Clone)]
pub struct RegexMetadata {
    pub source: String,
    pub ignore_case: bool,
    pub global: bool,
}

struct Node {
    marker: Marker,
    parent: Option<usize>,
    children: Vec<usize>,
}

struct Defaults {
    children: Vec<usize>,
    choices: Option<Vec<String>>,
}

struct Arena<F> {
    nodes: Vec<Node>,
    bytes: usize,
    markers: usize,
    limits: ParseLimits,
    compile: F,
    compiled: BTreeMap<(String, String), RegexMetadata>,
}

pub fn parse_complete(
    input: &str,
    limits: ParseLimits,
    final_tabstop: FinalTabstopOptions,
    mut compile: impl FnMut(&str, &str) -> Option<RegexMetadata>,
) -> Result<Vec<Marker>, EditorError> {
    let mut compiled = BTreeMap::new();
    let mut refused = false;
    let markers = parse(input, limits, |pattern, options| {
        let Some(metadata) = compile(pattern, options) else {
            return false;
        };
        if metadata.source.len() > limits.max_bytes {
            refused = true;
            return false;
        }
        compiled.insert((pattern.into(), options.into()), metadata);
        true
    })?;
    if refused {
        return Err(EditorError::Capacity);
    }
    let mut arena = Arena {
        nodes: vec![Node {
            marker: Marker::Text(String::new()),
            parent: None,
            children: Vec::new(),
        }],
        bytes: 0,
        markers: 0,
        limits,
        compile,
        compiled,
    };
    for marker in markers {
        let id = arena.import(marker, 0, 0)?;
        arena.nodes[0].children.push(id);
    }
    let mut defaults = BTreeMap::new();
    let mut incomplete = Vec::new();
    let mut pending = arena.nodes[0]
        .children
        .iter()
        .copied()
        .rev()
        .collect::<Vec<_>>();
    while let Some(id) = pending.pop() {
        let node = &arena.nodes[id];
        if let Marker::Placeholder { index, choices, .. } = &node.marker {
            if *index == Index::FINAL {
                defaults.insert(*index, None);
            } else if !defaults.contains_key(index)
                && (!node.children.is_empty() || choices.is_some())
            {
                defaults.insert(
                    *index,
                    Some(Defaults {
                        children: node.children.clone(),
                        choices: choices.clone(),
                    }),
                );
            } else {
                incomplete.push(id);
            }
        }
        pending.extend(node.children.iter().copied().rev());
    }
    for id in incomplete {
        arena.fill(id, &defaults, &mut BTreeSet::new(), 0)?;
    }
    let mut has_placeholder = false;
    let mut has_final = false;
    let mut pending = arena.nodes[0].children.clone();
    while let Some(id) = pending.pop() {
        let node = &arena.nodes[id];
        if let Marker::Placeholder { index, .. } = node.marker {
            has_placeholder = true;
            has_final |= index == Index::FINAL;
        }
        pending.extend(node.children.iter().copied());
    }
    if !has_final && (final_tabstop.enforce || final_tabstop.insert && has_placeholder) {
        let id = arena.allocate(
            Marker::Placeholder {
                index: Index::FINAL,
                children: Vec::new(),
                choices: None,
                transform: None,
            },
            0,
        )?;
        arena.nodes[0].children.push(id);
    }
    arena.nodes[0]
        .children
        .iter()
        .map(|id| arena.export(*id, 0))
        .collect()
}

impl<F: FnMut(&str, &str) -> Option<RegexMetadata>> Arena<F> {
    fn check_depth(&self, depth: usize) -> Result<(), EditorError> {
        if depth > self.limits.max_nesting {
            return Err(EditorError::Capacity);
        }
        Ok(())
    }

    fn charge(&mut self, marker: &Marker) -> Result<(), EditorError> {
        let (bytes, markers) = own_cost(marker)?;
        let bytes = self.bytes.checked_add(bytes).ok_or(EditorError::Capacity)?;
        let markers = self
            .markers
            .checked_add(markers)
            .ok_or(EditorError::Capacity)?;
        if bytes > self.limits.max_bytes || markers > self.limits.max_markers {
            return Err(EditorError::Capacity);
        }
        self.bytes = bytes;
        self.markers = markers;
        Ok(())
    }

    fn allocate(&mut self, marker: Marker, parent: usize) -> Result<usize, EditorError> {
        self.charge(&marker)?;
        let id = self.nodes.len();
        self.nodes.push(Node {
            marker,
            parent: Some(parent),
            children: Vec::new(),
        });
        Ok(id)
    }

    fn import(
        &mut self,
        mut marker: Marker,
        parent: usize,
        depth: usize,
    ) -> Result<usize, EditorError> {
        self.check_depth(depth)?;
        let children = match &mut marker {
            Marker::Text(_) => Vec::new(),
            Marker::Placeholder { children, .. } | Marker::Variable { children, .. } => {
                std::mem::take(children)
            }
        };
        let id = self.allocate(marker, parent)?;
        for child in children {
            let child = self.import(child, id, depth + 1)?;
            self.nodes[id].children.push(child);
        }
        Ok(id)
    }

    fn copy(&mut self, source: usize, parent: usize, depth: usize) -> Result<usize, EditorError> {
        self.check_depth(depth)?;
        let cost = own_cost(&self.nodes[source].marker)?;
        let bytes = self
            .bytes
            .checked_add(cost.0)
            .ok_or(EditorError::Capacity)?;
        let markers = self
            .markers
            .checked_add(cost.1)
            .ok_or(EditorError::Capacity)?;
        if bytes > self.limits.max_bytes || markers > self.limits.max_markers {
            return Err(EditorError::Capacity);
        }
        let mut marker = self.nodes[source].marker.clone();
        self.clone_transform(&mut marker)?;
        let children = self.nodes[source].children.clone();
        let id = self.allocate(marker, parent)?;
        for child in children {
            let child = self.copy(child, id, depth + 1)?;
            self.nodes[id].children.push(child);
        }
        Ok(id)
    }

    fn clone_transform(&mut self, marker: &mut Marker) -> Result<(), EditorError> {
        let transform = match marker {
            Marker::Text(_) => return Ok(()),
            Marker::Placeholder { transform, .. } | Marker::Variable { transform, .. } => transform,
        };
        let Some(transform) = transform else {
            return Ok(());
        };
        let metadata = self
            .compiled
            .get(&(transform.pattern.clone(), transform.options.clone()))
            .ok_or(EditorError::InvalidBoundary)?;
        let pattern = metadata.source.clone();
        let mut options = String::new();
        if metadata.ignore_case {
            options.push('i');
        }
        if metadata.global {
            options.push('g');
        }
        let metadata = (self.compile)(&pattern, &options).ok_or(EditorError::InvalidBoundary)?;
        if metadata.source.len() > self.limits.max_bytes {
            return Err(EditorError::Capacity);
        }
        self.compiled
            .insert((pattern.clone(), options.clone()), metadata);
        transform.pattern = pattern;
        transform.options = options;
        Ok(())
    }

    fn fill(
        &mut self,
        old: usize,
        defaults: &BTreeMap<Index, Option<Defaults>>,
        stack: &mut BTreeSet<Index>,
        depth: usize,
    ) -> Result<(), EditorError> {
        self.check_depth(depth)?;
        let Marker::Placeholder {
            index, transform, ..
        } = &self.nodes[old].marker
        else {
            return Err(EditorError::InvalidBoundary);
        };
        let Some(Some(value)) = defaults.get(index) else {
            return Ok(());
        };
        let parent = self.nodes[old].parent.ok_or(EditorError::InvalidBoundary)?;
        let marker = Marker::Placeholder {
            index: *index,
            children: Vec::new(),
            choices: value.choices.clone(),
            transform: transform.clone(),
        };
        let replacement = self.allocate(marker, parent)?;
        for child in &value.children {
            let child = self.copy(*child, replacement, depth + 1)?;
            self.nodes[replacement].children.push(child);
            if let Marker::Placeholder { index, .. } = self.nodes[child].marker
                && defaults.contains_key(&index)
                && stack.insert(index)
            {
                self.fill(child, defaults, stack, depth + 1)?;
                stack.remove(&index);
            }
        }
        let position = self.nodes[parent]
            .children
            .iter()
            .position(|child| *child == old)
            .ok_or(EditorError::InvalidBoundary)?;
        self.nodes[parent].children[position] = replacement;
        Ok(())
    }

    fn export(&self, id: usize, depth: usize) -> Result<Marker, EditorError> {
        self.check_depth(depth)?;
        let node = &self.nodes[id];
        let mut marker = node.marker.clone();
        match &mut marker {
            Marker::Text(_) => {}
            Marker::Placeholder { children, .. } | Marker::Variable { children, .. } => {
                *children = node
                    .children
                    .iter()
                    .map(|child| self.export(*child, depth + 1))
                    .collect::<Result<_, _>>()?;
            }
        }
        Ok(marker)
    }
}

pub(crate) fn own_cost(marker: &Marker) -> Result<(usize, usize), EditorError> {
    let (base, choices, transform) = match marker {
        Marker::Text(text) => return Ok((text.len(), 1)),
        Marker::Placeholder {
            choices, transform, ..
        } => (0, choices.as_deref().unwrap_or(&[]), transform),
        Marker::Variable {
            name, transform, ..
        } => (name.len(), &[][..], transform),
    };
    let bytes = choices.iter().try_fold(base, |bytes, choice| {
        bytes.checked_add(choice.len()).ok_or(EditorError::Capacity)
    })?;
    let markers = choices.len().checked_add(1).ok_or(EditorError::Capacity)?;
    let Some(transform) = transform else {
        return Ok((bytes, markers));
    };
    let bytes = bytes
        .checked_add(transform_cost(transform)?)
        .ok_or(EditorError::Capacity)?;
    let markers = markers
        .checked_add(transform.format.len())
        .ok_or(EditorError::Capacity)?;
    Ok((bytes, markers))
}

fn transform_cost(transform: &Transform) -> Result<usize, EditorError> {
    let bytes = transform
        .pattern
        .len()
        .checked_add(transform.options.len())
        .ok_or(EditorError::Capacity)?;
    transform.format.iter().try_fold(bytes, |bytes, part| {
        let values = match part {
            FormatPart::Text(text) => [Some(text), None, None],
            FormatPart::Capture {
                shorthand,
                if_value,
                else_value,
                ..
            } => [shorthand.as_ref(), if_value.as_ref(), else_value.as_ref()],
        };
        values
            .into_iter()
            .flatten()
            .try_fold(bytes, |bytes, value| {
                bytes.checked_add(value.len()).ok_or(EditorError::Capacity)
            })
    })
}
