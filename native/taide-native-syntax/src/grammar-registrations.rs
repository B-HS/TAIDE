use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy)]
pub(crate) struct LanguageRegistration<'a> {
    pub name: &'a str,
    pub scope_name: &'a str,
    pub aliases: &'a [String],
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LoadedGrammars {
    pub root_by_scope_name: BTreeMap<String, usize>,
    pub included_by_scope_name: BTreeMap<String, usize>,
    pub scope_name_by_language_id: BTreeMap<String, String>,
}

fn resolved_alias<'a>(name: &'a str, target_by_alias: &BTreeMap<&'a str, &'a str>) -> &'a str {
    let mut resolved = name;
    let mut followed = BTreeSet::from([name]);
    while let Some(target) = target_by_alias.get(resolved) {
        if !followed.insert(target) {
            break;
        }
        resolved = target;
    }
    resolved
}

pub(crate) fn loaded_grammars(registrations: &[LanguageRegistration<'_>]) -> LoadedGrammars {
    let mut included: BTreeMap<&str, usize> = registrations
        .iter()
        .enumerate()
        .map(|(index, registration)| (registration.scope_name, index))
        .collect();
    let mut load_order: Vec<(&str, usize)> = Vec::new();
    for (index, registration) in registrations.iter().enumerate() {
        match load_order
            .iter_mut()
            .find(|(name, _)| *name == registration.name)
        {
            Some(listed) => listed.1 = index,
            None => load_order.push((registration.name, index)),
        }
    }
    for (_, index) in &load_order {
        included.insert(registrations[*index].scope_name, *index);
    }
    let mut roots: BTreeMap<&str, usize> = BTreeMap::new();
    let mut scope_by_name: BTreeMap<&str, &str> = BTreeMap::new();
    let mut target_by_alias: BTreeMap<&str, &str> = BTreeMap::new();
    for (name, index) in load_order {
        if scope_by_name.contains_key(resolved_alias(name, &target_by_alias)) {
            continue;
        }
        let registration = &registrations[index];
        included.insert(registration.scope_name, index);
        roots.entry(registration.scope_name).or_insert(index);
        scope_by_name.insert(name, registration.scope_name);
        for alias in registration.aliases {
            target_by_alias.insert(alias, name);
        }
    }
    let scope_name_by_language_id = scope_by_name
        .keys()
        .chain(target_by_alias.keys())
        .filter_map(|language_id| {
            let scope_name = scope_by_name.get(resolved_alias(language_id, &target_by_alias))?;
            Some(((*language_id).to_owned(), (*scope_name).to_owned()))
        })
        .collect();
    LoadedGrammars {
        root_by_scope_name: roots
            .into_iter()
            .map(|(scope_name, index)| (scope_name.to_owned(), index))
            .collect(),
        included_by_scope_name: included
            .into_iter()
            .map(|(scope_name, index)| (scope_name.to_owned(), index))
            .collect(),
        scope_name_by_language_id,
    }
}

#[cfg(test)]
#[path = "grammar-registrations-tests.rs"]
mod tests;
