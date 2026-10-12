use std::ops::Range;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use taide_native_editor::document::DocumentSnapshot;
use taide_native_editor::editing::line_content_range;
use taide_native_editor::folding::{FoldingModel, MAX_FOLDING_REGIONS};
use taide_native_editor::view::ViewState;

const CONTRIBUTION: &str = "editor.contrib.folding";
const STRING_HASH_SEED: u32 = 149_417;
const STRING_HASH_BASE: u32 = 31;
const CHECKSUM_MODULUS: i32 = 1_000_000;
const LINE_HASH_SEED: u64 = 0xcbf2_9ce4_8422_2325;
const LINE_HASH_BASE: u64 = 0x0100_0000_01b3;
const UTF16_SCALAR_CAPACITY: usize = 2;
const USER_SOURCE: u8 = 1;
const RECOVERED_SOURCE: u8 = 2;
const BODY_HASH_VERSION: &str = "fnv-line-v1";

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Region {
    start_line_number: usize,
    end_line_number: usize,
    is_collapsed: Option<bool>,
    source: Option<u8>,
    checksum: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    taide_body_hash: Option<String>,
}

struct LineHash {
    checksum: u32,
    power: u32,
}

struct Hashes {
    lines: Vec<LineHash>,
    prefix: Vec<u64>,
    powers: Vec<u64>,
}

impl Hashes {
    fn new(document: &DocumentSnapshot) -> Self {
        let mut lines = Vec::with_capacity(document.rope.len_lines());
        let mut prefix = vec![0_u64];
        let mut powers = vec![1_u64];
        for line in 0..document.rope.len_lines() {
            let mut checksum = 0_u32;
            let mut power = 1_u32;
            let mut hash = LINE_HASH_SEED;
            for character in document
                .rope
                .byte_slice(line_content_range(document, line))
                .chars()
            {
                let mut units = [0; UTF16_SCALAR_CAPACITY];
                for unit in character.encode_utf16(&mut units).iter() {
                    checksum = checksum
                        .wrapping_mul(STRING_HASH_BASE)
                        .wrapping_add(u32::from(*unit));
                    power = power.wrapping_mul(STRING_HASH_BASE);
                    hash = (hash ^ u64::from(*unit)).wrapping_mul(LINE_HASH_BASE);
                }
            }
            lines.push(LineHash { checksum, power });
            prefix.push(
                prefix
                    .last()
                    .copied()
                    .unwrap_or(0)
                    .wrapping_mul(LINE_HASH_BASE)
                    .wrapping_add(hash),
            );
            powers.push(
                powers
                    .last()
                    .copied()
                    .unwrap_or(1)
                    .wrapping_mul(LINE_HASH_BASE),
            );
        }
        Self {
            lines,
            prefix,
            powers,
        }
    }

    fn checksum(&self, start: usize, end: usize) -> i32 {
        let first = &self.lines[start + 1];
        let last = &self.lines[end];
        let hash = STRING_HASH_SEED
            .wrapping_mul(first.power)
            .wrapping_add(first.checksum)
            .wrapping_mul(last.power)
            .wrapping_add(last.checksum);
        i32::from_ne_bytes(hash.to_ne_bytes()) % CHECKSUM_MODULUS
    }

    fn body(&self, start: usize, end: usize) -> String {
        let first = start + 1;
        let after = end + 1;
        let hash = self.prefix[after]
            .wrapping_sub(self.prefix[first].wrapping_mul(self.powers[after - first]));
        format!("{BODY_HASH_VERSION}:{hash:016x}")
    }
}

pub(super) fn capture(document: &DocumentSnapshot, view: &ViewState) -> Map<String, Value> {
    let is_enabled = taide_native_ui::presentation::editor_folding(document.metadata.tier);
    let mut ranges = if is_enabled {
        view.folds
            .iter()
            .chain(&view.manual_folds)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    ranges.sort_by_key(|range| (range.start, range.end));
    ranges.dedup();
    let hashes = (!ranges.is_empty()).then(|| Hashes::new(document));
    let regions = ranges
        .into_iter()
        .take(MAX_FOLDING_REGIONS)
        .filter_map(|range| {
            let start = document.rope.byte_to_line(range.start).checked_sub(1)?;
            let end = document.rope.byte_to_line(range.end);
            if start >= end || end >= document.rope.len_lines() {
                return None;
            }
            let hashes = hashes.as_ref()?;
            Some(Region {
                start_line_number: start + 1,
                end_line_number: end + 1,
                is_collapsed: Some(view.folds.contains(&range)),
                source: Some(if view.manual_folds.contains(&range) {
                    USER_SOURCE
                } else {
                    0
                }),
                checksum: Some(hashes.checksum(start, end)),
                taide_body_hash: Some(hashes.body(start, end)),
            })
        })
        .collect::<Vec<_>>();
    let value = serde_json::json!({ "collapsedRegions": regions, "lineCount": document.rope.len_lines(), "provider": "native" });
    Map::from_iter([(CONTRIBUTION.into(), value)])
}

pub(super) fn restore(
    document: &DocumentSnapshot,
    contributions: Option<&Map<String, Value>>,
) -> Option<(Vec<Range<usize>>, Vec<Range<usize>>)> {
    if !taide_native_ui::presentation::editor_folding(document.metadata.tier) {
        return None;
    }
    let saved = contributions?
        .get(CONTRIBUTION)?
        .as_object()?
        .get("collapsedRegions")?
        .as_array()?;
    let regions = saved
        .iter()
        .take(MAX_FOLDING_REGIONS)
        .filter_map(|value| {
            value.as_object()?;
            let region: Region = serde_json::from_value(value.clone()).ok()?;
            if region.start_line_number == 0
                || region.start_line_number >= region.end_line_number
                || region.end_line_number > document.rope.len_lines()
                || region
                    .source
                    .is_some_and(|source| source > RECOVERED_SOURCE)
            {
                return None;
            }
            Some(region)
        })
        .collect::<Vec<_>>();
    let hashes = (!regions.is_empty()).then(|| Hashes::new(document));
    let mut collapsed = Vec::new();
    let mut manual = Vec::new();
    for region in regions {
        let hashes = hashes.as_ref()?;
        let start = region.start_line_number - 1;
        let end = region.end_line_number - 1;
        if region
            .checksum
            .is_some_and(|checksum| checksum != 0 && checksum != hashes.checksum(start, end))
            || region
                .taide_body_hash
                .as_ref()
                .is_some_and(|hash| hash != &hashes.body(start, end))
        {
            continue;
        }
        let range = document.rope.line_to_byte(start + 1)..line_content_range(document, end).end;
        if region.source.is_some_and(|source| source != 0) {
            manual.push(range.clone());
        }
        if region.is_collapsed.unwrap_or(true) {
            collapsed.push(range);
        }
    }
    let manual = FoldingModel::with_manual(&[], document, &manual, &manual).folds(&document.rope);
    let collapsed =
        FoldingModel::with_manual(&[], document, &collapsed, &manual).folds(&document.rope);
    Some((collapsed, manual))
}
