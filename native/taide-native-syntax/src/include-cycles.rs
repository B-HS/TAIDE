use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde_json::Value;

const PATTERNS_KEY: &str = "patterns";
const REPOSITORY_KEY: &str = "repository";
const INJECTIONS_KEY: &str = "injections";
const INCLUDE_KEY: &str = "include";
const MATCH_KEY: &str = "match";
const BEGIN_KEY: &str = "begin";
const CAPTURE_KEYS: [&str; 4] = ["captures", "beginCaptures", "endCaptures", "whileCaptures"];
const BASE_INCLUDE: &str = "$base";
const SELF_INCLUDE: &str = "$self";
const REPOSITORY_SEPARATOR: char = '#';

#[derive(Debug, Clone, Copy)]
pub(crate) struct IncludeSource<'a> {
    pub scope_name: &'a str,
    pub grammar: &'a Value,
    pub plugin: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum RuleKey {
    Root(usize),
    Nested(*const Value),
}

#[derive(Clone, Copy)]
struct Rule<'a> {
    source: usize,
    nested: Option<&'a Value>,
}

impl Rule<'_> {
    fn key(&self) -> RuleKey {
        match self.nested {
            Some(rule) => RuleKey::Nested(std::ptr::from_ref(rule)),
            None => RuleKey::Root(self.source),
        }
    }
}

#[derive(Default)]
struct SourceRules<'a> {
    nested: Vec<&'a Value>,
    named: BTreeMap<&'a str, Vec<&'a Value>>,
}

fn entries(container: Option<&Value>) -> Vec<&Value> {
    match container {
        Some(Value::Array(rules)) => rules.iter().collect(),
        Some(Value::Object(rules)) => rules.values().collect(),
        _ => Vec::new(),
    }
}

fn source_rules(grammar: &Value) -> SourceRules<'_> {
    let mut rules = SourceRules::default();
    let mut pending = entries(grammar.get(PATTERNS_KEY));
    pending.extend(entries(grammar.get(INJECTIONS_KEY)));
    let mut repositories = vec![grammar.get(REPOSITORY_KEY)];
    loop {
        for repository in repositories.drain(..) {
            let Some(Value::Object(repository)) = repository else {
                continue;
            };
            for (name, rule) in repository {
                rules.named.entry(name.as_str()).or_default().push(rule);
                pending.push(rule);
            }
        }
        let Some(rule) = pending.pop() else {
            return rules;
        };
        rules.nested.push(rule);
        if let Value::Array(patterns) = rule {
            pending.extend(patterns);
            continue;
        }
        pending.extend(entries(rule.get(PATTERNS_KEY)));
        for key in CAPTURE_KEYS {
            pending.extend(entries(rule.get(key)));
        }
        repositories.push(rule.get(REPOSITORY_KEY));
    }
}

fn is_include_only(rule: &Value) -> bool {
    match rule {
        Value::Array(_) => true,
        Value::Object(rule) => [MATCH_KEY, BEGIN_KEY]
            .into_iter()
            .all(|key| !rule.get(key).is_some_and(Value::is_string)),
        _ => false,
    }
}

struct IncludeGraph<'a> {
    sources: &'a [IncludeSource<'a>],
    rules: Vec<SourceRules<'a>>,
    source_indexes_by_scope_name: BTreeMap<&'a str, Vec<usize>>,
}

impl<'a> IncludeGraph<'a> {
    fn new(sources: &'a [IncludeSource<'a>]) -> Self {
        let mut source_indexes_by_scope_name: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (index, source) in sources.iter().enumerate() {
            source_indexes_by_scope_name
                .entry(source.scope_name)
                .or_default()
                .push(index);
        }
        Self {
            sources,
            rules: sources
                .iter()
                .map(|source| source_rules(source.grammar))
                .collect(),
            source_indexes_by_scope_name,
        }
    }

    fn sources_of(&self, scope_name: &str) -> &[usize] {
        self.source_indexes_by_scope_name
            .get(scope_name)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    fn named(&self, source: usize, name: &str, included: &mut Vec<Rule<'a>>) {
        let rules = self.rules[source].named.get(name);
        included.extend(rules.into_iter().flatten().map(|rule| Rule {
            source,
            nested: Some(*rule),
        }));
    }

    fn included(&self, source: usize, include: &str, included: &mut Vec<Rule<'a>>) {
        let root = |source| Rule {
            source,
            nested: None,
        };
        if include == BASE_INCLUDE {
            included.extend((0..self.sources.len()).map(root));
        } else if include == SELF_INCLUDE {
            included.push(root(source));
        } else if let Some((scope_name, rule_name)) = include.split_once(REPOSITORY_SEPARATOR) {
            if scope_name.is_empty() {
                self.named(source, rule_name, included);
            } else {
                for external in self.sources_of(scope_name) {
                    self.named(*external, rule_name, included);
                }
            }
        } else {
            included.extend(self.sources_of(include).iter().copied().map(root));
        }
    }

    fn collected(&self, rule: Rule<'a>) -> Vec<Rule<'a>> {
        let mut collected = Vec::new();
        let patterns = match rule.nested {
            None => self.sources[rule.source].grammar.get(PATTERNS_KEY),
            Some(patterns @ Value::Array(_)) => Some(patterns),
            Some(nested) => match nested.get(PATTERNS_KEY) {
                Some(patterns @ Value::Array(_)) => Some(patterns),
                _ => {
                    if let Some(include) = nested.get(INCLUDE_KEY).and_then(Value::as_str) {
                        self.included(rule.source, include, &mut collected);
                    }
                    None
                }
            },
        };
        for pattern in entries(patterns.filter(|patterns| patterns.is_array())) {
            match pattern.get(INCLUDE_KEY).and_then(Value::as_str) {
                Some(include) => self.included(rule.source, include, &mut collected),
                None => collected.push(Rule {
                    source: rule.source,
                    nested: Some(pattern),
                }),
            }
        }
        collected.retain(|collected| collected.nested.is_none_or(is_include_only));
        collected
    }
}

#[derive(Default)]
struct Collection<'a> {
    rules: Vec<Rule<'a>>,
    index_by_key: HashMap<RuleKey, usize>,
    collected: Vec<Vec<usize>>,
}

impl<'a> Collection<'a> {
    fn index(&mut self, rule: Rule<'a>) -> usize {
        let next = self.rules.len();
        let index = *self.index_by_key.entry(rule.key()).or_insert(next);
        if index == next {
            self.rules.push(rule);
        }
        index
    }

    fn reachable_from_plugins(graph: &IncludeGraph<'a>) -> Self {
        let mut collection = Self::default();
        for (source, listed) in graph.sources.iter().enumerate() {
            if listed.plugin.is_none() {
                continue;
            }
            collection.index(Rule {
                source,
                nested: None,
            });
            for nested in graph.rules[source].nested.iter().copied() {
                if is_include_only(nested) {
                    collection.index(Rule {
                        source,
                        nested: Some(nested),
                    });
                }
            }
        }
        while collection.collected.len() < collection.rules.len() {
            let rule = collection.rules[collection.collected.len()];
            let collected = graph
                .collected(rule)
                .into_iter()
                .map(|collected| collection.index(collected))
                .collect();
            collection.collected.push(collected);
        }
        collection
    }

    fn cyclic_groups(&self) -> Vec<Vec<usize>> {
        struct Visit {
            rule: usize,
            next: usize,
        }
        let count = self.rules.len();
        let mut order: Vec<Option<usize>> = vec![None; count];
        let mut lowest: Vec<usize> = vec![0; count];
        let mut is_open: Vec<bool> = vec![false; count];
        let mut open: Vec<usize> = Vec::new();
        let mut visits: Vec<Visit> = Vec::new();
        let mut visited = 0;
        let mut groups = Vec::new();
        for start in 0..count {
            if order[start].is_some() {
                continue;
            }
            visits.push(Visit {
                rule: start,
                next: 0,
            });
            while let Some(visit) = visits.last_mut() {
                let rule = visit.rule;
                if visit.next == 0 {
                    order[rule] = Some(visited);
                    lowest[rule] = visited;
                    visited += 1;
                    open.push(rule);
                    is_open[rule] = true;
                }
                if let Some(collected) = self.collected[rule].get(visit.next).copied() {
                    visit.next += 1;
                    match order[collected] {
                        None => visits.push(Visit {
                            rule: collected,
                            next: 0,
                        }),
                        Some(position) if is_open[collected] => {
                            lowest[rule] = lowest[rule].min(position);
                        }
                        Some(_) => {}
                    }
                    continue;
                }
                visits.pop();
                if let Some(parent) = visits.last() {
                    lowest[parent.rule] = lowest[parent.rule].min(lowest[rule]);
                }
                if order[rule] != Some(lowest[rule]) {
                    continue;
                }
                let mut group = Vec::new();
                while let Some(member) = open.pop() {
                    is_open[member] = false;
                    group.push(member);
                    if member == rule {
                        break;
                    }
                }
                if group.len() > 1 || self.collected[rule].contains(&rule) {
                    groups.push(group);
                }
            }
        }
        groups
    }
}

pub(crate) fn plugins_on_include_only_cycles(sources: &[IncludeSource<'_>]) -> BTreeSet<usize> {
    let graph = IncludeGraph::new(sources);
    let collection = Collection::reachable_from_plugins(&graph);
    collection
        .cyclic_groups()
        .into_iter()
        .flatten()
        .filter_map(|rule| sources[collection.rules[rule].source].plugin)
        .collect()
}

#[cfg(test)]
#[path = "include-cycles-tests.rs"]
mod tests;
