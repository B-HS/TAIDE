use std::collections::HashSet;

use taide_model::tree::TreeRow;
use taide_native_editor::document_symbols::Symbol;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Row {
    pub symbol: usize,
    pub id: String,
    pub depth: usize,
    pub has_children: bool,
    pub collapsed: bool,
}

struct Node {
    id: String,
    parent: Option<usize>,
    children: Vec<usize>,
    depth: usize,
}

pub(crate) struct Tree {
    nodes: Vec<Node>,
    roots: Vec<usize>,
}

impl Tree {
    pub(crate) fn new(symbols: &[Symbol]) -> Self {
        let mut nodes: Vec<Node> = Vec::with_capacity(symbols.len());
        let mut roots = Vec::new();
        for (index, symbol) in symbols.iter().enumerate() {
            let parent = symbol.parent.filter(|parent| *parent < index);
            let (id, depth) = match parent {
                Some(parent) => {
                    let node = &mut nodes[parent];
                    let id = format!("{}/{}", node.id, node.children.len());
                    node.children.push(index);
                    (id, node.depth + 1)
                }
                None => {
                    let id = format!("/{}", roots.len());
                    roots.push(index);
                    (id, 0)
                }
            };
            nodes.push(Node {
                id,
                parent,
                children: Vec::new(),
                depth,
            });
        }
        Self { nodes, roots }
    }

    pub(crate) fn rows(&self, collapsed: &HashSet<String>) -> Vec<Row> {
        let mut visible = vec![false; self.nodes.len()];
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                visible[index] = node.parent.is_none_or(|parent| {
                    visible[parent] && !collapsed.contains(&self.nodes[parent].id)
                });
                visible[index].then(|| Row {
                    symbol: index,
                    id: node.id.clone(),
                    depth: node.depth,
                    has_children: !node.children.is_empty(),
                    collapsed: collapsed.contains(&node.id),
                })
            })
            .collect()
    }

    pub(crate) fn parent(&self, index: usize) -> Option<usize> {
        self.nodes.get(index)?.parent
    }

    pub(crate) fn id(&self, index: usize) -> Option<&str> {
        self.nodes.get(index).map(|node| node.id.as_str())
    }

    pub(crate) fn children(&self, parent: Option<usize>) -> &[usize] {
        match parent {
            Some(parent) => self
                .nodes
                .get(parent)
                .map_or(&[], |node| node.children.as_slice()),
            None => &self.roots,
        }
    }

    pub(crate) fn enclosing(&self, symbols: &[Symbol], byte: usize) -> Vec<usize> {
        let mut siblings = self.roots.as_slice();
        let mut chain = Vec::new();
        while let Some(index) = siblings.iter().copied().find(|index| {
            symbols
                .get(*index)
                .is_some_and(|symbol| symbol.bytes.start <= byte && byte <= symbol.bytes.end)
        }) {
            chain.push(index);
            siblings = self.nodes[index].children.as_slice();
        }
        chain
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathSegment {
    pub label: String,
    pub path: String,
    pub parent: String,
}

pub(crate) fn path_segments(root: &str, path: &str) -> Vec<PathSegment> {
    let root = root.trim_end_matches('/');
    let prefix = format!("{root}/");
    let (mut current, remainder) = match path.strip_prefix(&prefix) {
        Some(relative) => (root.to_owned(), relative),
        None => (String::new(), path),
    };
    let mut segments = Vec::new();
    for label in remainder.split('/').filter(|part| !part.is_empty()) {
        let parent = if current.is_empty() {
            "/".into()
        } else {
            current.clone()
        };
        current = if current.is_empty() && !path.starts_with('/') {
            label.into()
        } else {
            format!("{current}/{label}")
        };
        segments.push(PathSegment {
            label: label.into(),
            path: current.clone(),
            parent,
        });
    }
    segments
}

pub(crate) fn direct_children<'a>(rows: &'a [TreeRow], parent: &str) -> Vec<&'a TreeRow> {
    rows.iter()
        .filter(|row| {
            let directory = row.path.rsplit_once('/').map_or("/", |(directory, _)| {
                if directory.is_empty() { "/" } else { directory }
            });
            directory == parent
        })
        .collect()
}

#[cfg(test)]
#[path = "symbol-navigation-tests.rs"]
mod tests;
