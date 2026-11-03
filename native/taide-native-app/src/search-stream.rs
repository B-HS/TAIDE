use std::collections::HashMap;
use std::sync::Mutex;

use taide_model::search::SearchFileMatches;

/// A search surface's backend session: the window label that owns it and the surface's id.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Session {
    pub(crate) owner: String,
    pub(crate) id: String,
}

struct Stream {
    generation: u64,
    batches: Vec<SearchFileMatches>,
}

/// Per-file batches a running search delivered from the host's blocking walk, buffered until
/// the surface's window next draws; only the surface's current run generation is kept.
#[derive(Default)]
pub struct Streams {
    entries: Mutex<HashMap<Session, Stream>>,
}

impl Streams {
    pub(crate) fn begin(&self, session: &Session, generation: u64) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.insert(
                session.clone(),
                Stream {
                    generation,
                    batches: Vec::new(),
                },
            );
        }
    }

    /// Buffers `batch` for the session's current run; true when the buffer was empty before.
    pub(crate) fn push(
        &self,
        session: &Session,
        generation: u64,
        batch: SearchFileMatches,
    ) -> bool {
        self.entries
            .lock()
            .ok()
            .and_then(|mut entries| {
                let stream = entries
                    .get_mut(session)
                    .filter(|stream| stream.generation == generation)?;
                stream.batches.push(batch);
                Some(stream.batches.len() == 1)
            })
            .unwrap_or(false)
    }

    pub(crate) fn has(&self, session: &Session, generation: u64) -> bool {
        self.entries.lock().is_ok_and(|entries| {
            entries
                .get(session)
                .is_some_and(|stream| stream.generation == generation && !stream.batches.is_empty())
        })
    }

    pub(crate) fn take(&self, session: &Session, generation: u64) -> Vec<SearchFileMatches> {
        self.entries
            .lock()
            .ok()
            .and_then(|mut entries| {
                entries
                    .get_mut(session)
                    .filter(|stream| stream.generation == generation)
                    .map(|stream| std::mem::take(&mut stream.batches))
            })
            .unwrap_or_default()
    }

    pub(crate) fn forget(&self, session: &Session) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(session);
        }
    }
}
