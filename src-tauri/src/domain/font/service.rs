use std::collections::BTreeMap;

use fontdb::Database;

use crate::domain::font::types::FontFamily;

pub fn list_families() -> Vec<FontFamily> {
    let mut database = Database::new();
    database.load_system_fonts();
    collect_families(&database)
}

fn collect_families(database: &Database) -> Vec<FontFamily> {
    let mut families: BTreeMap<String, bool> = BTreeMap::new();

    for face in database.faces() {
        for (name, _) in &face.families {
            let entry = families.entry(name.clone()).or_insert(face.monospaced);
            *entry = *entry || face.monospaced;
        }
    }

    families
        .into_iter()
        .map(|(name, monospaced)| FontFamily { name, monospaced })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 빈_데이터베이스는_빈_목록을_반환한다() {
        assert!(collect_families(&Database::new()).is_empty());
    }

    #[test]
    fn 가족명은_중복_없이_정렬되어_반환된다() {
        let families = list_families();
        let names: Vec<_> = families.iter().map(|family| family.name.clone()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(names, sorted);
    }
}
