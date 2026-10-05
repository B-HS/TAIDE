use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use bytes::Bytes;
use taide_model::error::AppResult;

use crate::{preview::invalid, preview_web_document::MAX_OUTPUT_BYTES};

pub(crate) const MAX_DOCUMENT_BYTES: usize = 128 * 1024 * 1024;

pub(crate) struct Budget {
    limit: usize,
    used: AtomicUsize,
}

impl Budget {
    pub(crate) fn new(limit: usize) -> Arc<Self> {
        Arc::new(Self {
            limit,
            used: AtomicUsize::new(0),
        })
    }

    fn reserve(self: &Arc<Self>, bytes: usize) -> AppResult<Lease> {
        self.used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(bytes).filter(|total| *total <= self.limit)
            })
            .map_err(|_| invalid("preview published document budget exceeded"))?;
        Ok(Lease {
            budget: self.clone(),
            bytes,
        })
    }
}

struct Lease {
    budget: Arc<Budget>,
    bytes: usize,
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.budget.used.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}

struct OwnedBytes {
    html: Arc<String>,
    _lease: Lease,
}

impl AsRef<[u8]> for OwnedBytes {
    fn as_ref(&self) -> &[u8] {
        self.html.as_bytes()
    }
}

pub(crate) struct Document {
    pub(crate) path: String,
    pub(crate) bytes: Bytes,
}

impl Document {
    pub(crate) fn new(path: String, html: Arc<String>, budget: &Arc<Budget>) -> AppResult<Self> {
        if html.len() > MAX_OUTPUT_BYTES || html.capacity() > MAX_OUTPUT_BYTES {
            return Err(invalid(
                "preview published document exceeds its source budget",
            ));
        }
        let retained = html
            .capacity()
            .checked_add(path.capacity())
            .and_then(|bytes| {
                bytes.checked_add(size_of::<Self>() + size_of::<OwnedBytes>() + size_of::<String>())
            })
            .ok_or_else(|| invalid("preview published document size overflow"))?;
        let lease = budget.reserve(retained)?;
        Ok(Self {
            path,
            bytes: Bytes::from_owner(OwnedBytes {
                html,
                _lease: lease,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_served_document_budgetは_zero_copy_sliceの_最後の所有者まで保つ() {
        let html = Arc::new("synthetic document".to_owned());
        let pointer = html.as_ptr();
        let budget = Budget::new(MAX_DOCUMENT_BYTES);
        let document = Document::new("/synthetic.html".into(), html, &budget).unwrap();
        assert_eq!(document.bytes.as_ptr(), pointer);
        let used = budget.used.load(Ordering::Acquire);
        assert!(used > document.bytes.len());
        let slice = document.bytes.slice(1..);
        drop(document);
        assert_eq!(budget.used.load(Ordering::Acquire), used);
        drop(slice);
        assert_eq!(budget.used.load(Ordering::Acquire), 0);
        let budget = Budget::new(0);
        assert!(Document::new("/synthetic.html".into(), Arc::new("x".into()), &budget).is_err());
        assert_eq!(budget.used.load(Ordering::Acquire), 0);
    }
}
