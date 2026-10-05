use taide_model::snippet::{SnippetFile, SnippetStringOrList};

use crate::snippet_draft::{is_global_file, lines, object_order, trim};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Candidate {
    pub name: String,
    pub prefix: String,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

pub fn collect(files: &[SnippetFile], language_id: &str) -> Vec<Candidate> {
    let language_file = format!("{language_id}.json");
    let mut candidates = Vec::new();
    for file in files {
        let global = is_global_file(&file.file_name);
        if !global && file.file_name != language_file {
            continue;
        }
        let mut entries = file
            .snippets
            .iter()
            .map(|(name, entry)| (name.clone(), entry))
            .collect::<Vec<_>>();
        object_order(&mut entries);
        for (name, entry) in entries {
            if global {
                let scope = entry
                    .scope
                    .as_deref()
                    .unwrap_or_default()
                    .split(',')
                    .map(trim)
                    .filter(|language| !language.is_empty())
                    .collect::<Vec<_>>();
                if !scope.is_empty() && !scope.contains(&language_id) {
                    continue;
                }
            }
            let prefixes = match &entry.prefix {
                SnippetStringOrList::Single(prefix) => std::slice::from_ref(prefix),
                SnippetStringOrList::Multiple(prefixes) => prefixes.as_slice(),
            };
            candidates.extend(prefixes.iter().map(|prefix| Candidate {
                name: name.clone(),
                prefix: prefix.clone(),
                body: lines(&entry.body),
                description: entry.description.as_ref().map(lines),
            }));
        }
    }
    candidates
}
