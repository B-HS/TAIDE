use std::collections::BTreeMap;
use std::sync::OnceLock;

use fontdb::Database;

use crate::domain::font::types::FontFamily;

/// Process-lifetime cache of the system font scan (audit R8#11): every `font_list` call used to
/// walk the entire system font set from scratch. One scan on first use is the whole contract —
/// fonts installed or removed while the app is running are picked up after an app restart, which
/// is the current UX requirement (no live font watching exists anywhere in the app).
static FONT_FAMILIES: OnceLock<Vec<FontFamily>> = OnceLock::new();

/// Test-only probe counting how many times the system font scan actually ran, so the cache test
/// can pin "the public entry point never rescans" — `OnceLock` pointer identity alone is a
/// tautology and would keep passing even if [`list_families`] stopped using the cache.
#[cfg(test)]
static SYSTEM_FONT_SCAN_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn cached_families() -> &'static Vec<FontFamily> {
    FONT_FAMILIES.get_or_init(|| {
        #[cfg(test)]
        SYSTEM_FONT_SCAN_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut database = Database::new();
        database.load_system_fonts();
        collect_families(&database)
    })
}

pub fn list_families() -> Vec<FontFamily> {
    cached_families().clone()
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
    fn 폰트_목록은_프로세스_수명_캐시_한_벌을_공유한다() {
        let first = list_families();
        let second = list_families();

        assert_eq!(first, second, "공개 진입점은 항상 같은 스캔 결과를 반환해야 한다");
        assert_eq!(
            SYSTEM_FONT_SCAN_COUNT.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "시스템 폰트 전수 스캔은 프로세스 수명 동안 1회만 실행되어야 한다 (R8#11)"
        );
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
