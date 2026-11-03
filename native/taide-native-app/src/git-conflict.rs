use std::time::{Duration, Instant};

use taide_model::ids::TabId;

/// Typing in a conflicted file re-reads its markers this long after the last change
/// (`CONFLICT_REPARSE_DEBOUNCE_MS`).
pub(crate) const REPARSE_DELAY: Duration = Duration::from_millis(150);
const START_MARKER: &str = "<<<<<<<";
const BASE_MARKER: &str = "|||||||";
const SEPARATOR_MARKER: &str = "=======";
const END_MARKER: &str = ">>>>>>>";

/// One complete conflict region in 1-based lines (`conflict-marker.ts` `ConflictRegion`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Region {
    pub(crate) start: u32,
    pub(crate) base: Option<u32>,
    pub(crate) separator: u32,
    pub(crate) end: u32,
}

/// Which side a resolution keeps (`acceptCurrentChange`, `acceptIncomingChange`,
/// `acceptBothChanges`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Resolution {
    Current,
    Incoming,
    Both,
}

enum Parse {
    Start,
    Separator {
        start: u32,
        base: Option<u32>,
    },
    End {
        start: u32,
        base: Option<u32>,
        separator: u32,
    },
}

/// `parseConflictMarkers`: start, optional base, an exact separator line and end, line endings
/// without their carriage return; an unfinished region is dropped.
pub(crate) fn parse(content: &str) -> Vec<Region> {
    let mut regions = Vec::new();
    let mut state = Parse::Start;
    for (index, raw) in content.split('\n').enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let number = index as u32 + 1;
        if line.starts_with(START_MARKER) {
            state = Parse::Separator {
                start: number,
                base: None,
            };
            continue;
        }
        state = match state {
            Parse::Separator { start, .. } if line.starts_with(BASE_MARKER) => Parse::Separator {
                start,
                base: Some(number),
            },
            Parse::Separator { start, base } if line == SEPARATOR_MARKER => Parse::End {
                start,
                base,
                separator: number,
            },
            Parse::End {
                start,
                base,
                separator,
            } if line.starts_with(END_MARKER) => {
                regions.push(Region {
                    start,
                    base,
                    separator,
                    end: number,
                });
                Parse::Start
            }
            other => other,
        };
    }
    regions
}

/// Replaces `region`, markers included, with the side or sides `resolution` keeps; a region that
/// no longer fits `content` leaves it unchanged.
pub(crate) fn resolve(content: &str, region: Region, resolution: Resolution) -> String {
    let lines = content.split('\n').collect::<Vec<_>>();
    let slice = |from: u32, to: u32| {
        if from > to {
            Some(&lines[0..0])
        } else {
            lines.get(from as usize - 1..to as usize)
        }
    };
    let (Some(ours), Some(theirs), Some(before), Some(after)) = (
        slice(
            region.start + 1,
            region.base.unwrap_or(region.separator) - 1,
        ),
        slice(region.separator + 1, region.end - 1),
        lines.get(..region.start as usize - 1),
        lines.get(region.end as usize..),
    ) else {
        return content.to_owned();
    };
    let kept = match resolution {
        Resolution::Current => ours.to_vec(),
        Resolution::Incoming => theirs.to_vec(),
        Resolution::Both => [ours, theirs].concat(),
    };
    before
        .iter()
        .chain(&kept)
        .chain(after)
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
}

/// The compare dialog's request: its virtual documents' owner and the read answer seen when it
/// was pressed, so only a newer failure toasts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Compare {
    pub(crate) owner: TabId,
    pub(crate) seen: Option<Instant>,
}

/// One file editor's conflict state (`use-editor-git-gutter-and-conflicts.ts`): the parsed regions
/// with their document revision, the pending re-parse, the region whose dialog is open and the
/// compare request.
#[derive(Default)]
pub(crate) struct Conflicts {
    regions: Vec<Region>,
    parsed: Option<u64>,
    changed: Option<(u64, Instant)>,
    pub(crate) pending: Option<Region>,
    pub(crate) compare: Option<Compare>,
}

impl Conflicts {
    pub(crate) fn regions(&self) -> &[Region] {
        &self.regions
    }

    /// Follows the document while the file is conflicted: the first sight parses at once, later
    /// revisions [`REPARSE_DELAY`] after the last change. Answers whether `text` should be parsed
    /// now, or how long until it should.
    pub(crate) fn observe(
        &mut self,
        is_conflicted: bool,
        revision: u64,
        now: Instant,
    ) -> Result<bool, Duration> {
        if !is_conflicted {
            self.parsed = None;
            self.changed = None;
            return Ok(false);
        }
        let Some(parsed) = self.parsed else {
            return Ok(true);
        };
        if parsed == revision {
            self.changed = None;
            return Ok(false);
        }
        if self.changed.is_none_or(|(seen, _)| seen != revision) {
            self.changed = Some((revision, now));
        }
        let waited = self
            .changed
            .map_or(Duration::ZERO, |(_, at)| now.saturating_duration_since(at));
        if waited >= REPARSE_DELAY {
            return Ok(true);
        }
        Err(REPARSE_DELAY - waited)
    }

    pub(crate) fn parsed(&mut self, revision: u64, text: &str) {
        self.regions = parse(text);
        self.parsed = Some(revision);
        self.changed = None;
    }

    pub(crate) fn region_at(&self, line: u32) -> Option<Region> {
        self.regions
            .iter()
            .find(|region| region.start == line)
            .copied()
    }
}

#[cfg(test)]
#[path = "git-conflict-tests.rs"]
mod tests;
