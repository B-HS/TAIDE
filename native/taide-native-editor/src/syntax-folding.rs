use std::sync::Arc;

use crate::document::{DocumentId, DocumentKey, DocumentSnapshot};
use crate::folding::{FoldRegion, MAX_FOLDABLE_LINE, MAX_FOLDING_REGIONS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxFoldRange {
    pub region: FoldRegion,
    pub kind: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SyntaxFolds {
    document: DocumentId,
    key: DocumentKey,
    revision: u64,
    language_id: String,
    regions: Arc<[FoldRegion]>,
    kinds: Vec<Option<String>>,
}

impl SyntaxFolds {
    pub fn new(document: &DocumentSnapshot, providers: Vec<Vec<SyntaxFoldRange>>) -> Self {
        let mut ranges = providers
            .into_iter()
            .enumerate()
            .flat_map(|(rank, ranges)| {
                ranges
                    .into_iter()
                    .take(MAX_FOLDING_REGIONS)
                    .map(move |range| (rank, range))
            })
            .filter(|(_, range)| {
                range.region.start_line < range.region.end_line
                    && range.region.end_line < document.rope.len_lines()
                    && range.region.end_line < MAX_FOLDABLE_LINE
            })
            .collect::<Vec<_>>();
        ranges.sort_by_key(|(rank, range)| (range.region.start_line, *rank));
        let mut nested = Vec::<FoldRegion>::new();
        let mut accepted = Vec::new();
        for (_, range) in ranges {
            while nested
                .last()
                .is_some_and(|outer| outer.end_line < range.region.start_line)
            {
                nested.pop();
            }
            if accepted
                .last()
                .is_some_and(|(last, _): &(SyntaxFoldRange, usize)| {
                    last.region.start_line == range.region.start_line
                })
                || nested
                    .last()
                    .is_some_and(|outer| outer.end_line < range.region.end_line)
            {
                continue;
            }
            let region = range.region;
            accepted.push((range, nested.len()));
            nested.push(region);
        }
        if accepted.len() > MAX_FOLDING_REGIONS {
            let mut depths = accepted.iter().map(|(_, depth)| *depth).collect::<Vec<_>>();
            depths.sort_unstable();
            let threshold = depths[MAX_FOLDING_REGIONS - 1];
            let mut remaining =
                MAX_FOLDING_REGIONS - depths.partition_point(|depth| *depth < threshold);
            accepted.retain(|(_, depth)| {
                if *depth < threshold {
                    return true;
                }
                if *depth != threshold || remaining == 0 {
                    return false;
                }
                remaining -= 1;
                true
            });
        }
        Self {
            document: document.id,
            key: document.key.clone(),
            revision: document.revision,
            language_id: document.metadata.language_id.clone(),
            regions: accepted.iter().map(|(range, _)| range.region).collect(),
            kinds: accepted.into_iter().map(|(range, _)| range.kind).collect(),
        }
    }

    pub fn describes(&self, document: &DocumentSnapshot) -> bool {
        self.document == document.id
            && self.key == document.key
            && self.revision == document.revision
            && self.language_id == document.metadata.language_id
    }

    pub fn regions(&self) -> &Arc<[FoldRegion]> {
        &self.regions
    }

    pub fn kind(&self, line: usize) -> Option<&str> {
        let index = self
            .regions
            .binary_search_by_key(&line, |region| region.start_line)
            .ok()?;
        self.kinds[index].as_deref()
    }

    pub fn has_kinds(&self) -> bool {
        self.kinds.iter().flatten().any(|kind| !kind.is_empty())
    }
}
