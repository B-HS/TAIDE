#![doc = include_str!("../README.md")]

use std::{
    alloc::Layout,
    borrow::Cow,
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    hash::BuildHasher,
    marker::PhantomData,
    sync::{Arc, Mutex, RwLock, atomic::AtomicUsize},
};

pub use taide_native_retained_derive::RetainedBytes;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub bytes: usize,
    pub visits: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Report {
    pub bytes: usize,
    pub visits: usize,
    pub shared_allocations: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    ByteBudget,
    VisitBudget,
    Allocation,
    Borrowed,
    Locked,
    Opaque,
}

impl fmt::Display for Error {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ByteBudget => "retained payload exceeds the byte budget",
            Self::VisitBudget => "retained traversal exceeds the visit budget",
            Self::Allocation => "retained traversal allocation failed",
            Self::Borrowed => "retained payload is mutably borrowed",
            Self::Locked => "retained payload is locked or poisoned",
            Self::Opaque => "retained payload contains an unaccounted opaque owner",
        };
        output.write_str(message)
    }
}

impl std::error::Error for Error {}

pub trait RetainedBytes {
    fn has_owned_children() -> bool
    where
        Self: Sized,
    {
        true
    }

    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error>;
}

struct State {
    limits: Limits,
    bytes: usize,
    scheduled: usize,
    visited: usize,
    shared: HashSet<usize>,
}

pub struct Visitor<'value, 'state> {
    state: &'state mut State,
    pending: Vec<&'value dyn RetainedBytes>,
}

impl<'value> Visitor<'value, '_> {
    pub fn charge(&mut self, bytes: usize) -> Result<(), Error> {
        self.state.bytes = self
            .state
            .bytes
            .checked_add(bytes)
            .filter(|bytes| *bytes <= self.state.limits.bytes)
            .ok_or(Error::ByteBudget)?;
        Ok(())
    }

    pub fn capacity<T>(&mut self, capacity: usize) -> Result<(), Error> {
        self.charge(
            capacity
                .checked_mul(size_of::<T>())
                .ok_or(Error::ByteBudget)?,
        )
    }

    pub fn push<T: RetainedBytes>(&mut self, value: &'value T) -> Result<(), Error> {
        if !T::has_owned_children() {
            return Ok(());
        }
        self.push_dyn(value)
    }

    pub fn push_dyn(&mut self, value: &'value dyn RetainedBytes) -> Result<(), Error> {
        if self.state.scheduled >= self.state.limits.visits {
            return Err(Error::VisitBudget);
        }
        self.pending.try_reserve(1).map_err(|_| Error::Allocation)?;
        self.state.scheduled += 1;
        self.pending.push(value);
        Ok(())
    }

    pub fn mark_shared(&mut self, pointer: usize) -> Result<bool, Error> {
        if self.state.shared.contains(&pointer) {
            return Ok(false);
        }
        self.state
            .shared
            .try_reserve(1)
            .map_err(|_| Error::Allocation)?;
        self.state.shared.insert(pointer);
        Ok(true)
    }

    pub fn shared_arc<T: ?Sized>(&mut self, value: &Arc<T>) -> Result<bool, Error> {
        if !self.mark_shared(Arc::as_ptr(value).cast::<()>() as usize)? {
            return Ok(false);
        }
        self.charge(arc_layout(value.as_ref())?)?;
        Ok(true)
    }

    pub fn scoped<T: RetainedBytes>(&mut self, value: &T) -> Result<(), Error> {
        let mut child = Visitor {
            state: self.state,
            pending: Vec::new(),
        };
        child.push(value)?;
        child.run()
    }

    fn run(&mut self) -> Result<(), Error> {
        while let Some(value) = self.pending.pop() {
            self.state.visited += 1;
            value.visit(self)?;
        }
        Ok(())
    }
}

pub fn measure<T: RetainedBytes>(value: &T, limits: Limits) -> Result<Report, Error> {
    let mut state = State {
        limits,
        bytes: 0,
        scheduled: 0,
        visited: 0,
        shared: HashSet::new(),
    };
    let mut visitor = Visitor {
        state: &mut state,
        pending: Vec::new(),
    };
    visitor.charge(size_of_val(value))?;
    visitor.push(value)?;
    visitor.run()?;
    Ok(Report {
        bytes: state.bytes,
        visits: state.visited,
        shared_allocations: state.shared.len(),
    })
}

macro_rules! leaf {
    ($($kind:ty),* $(,)?) => { $(
        impl RetainedBytes for $kind {
            fn has_owned_children() -> bool { false }
            fn visit<'value>(&'value self, _: &mut Visitor<'value, '_>) -> Result<(), Error> { Ok(()) }
        }
    )* };
}

leaf!(
    (),
    bool,
    char,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
    std::time::Instant,
    AtomicUsize,
    std::collections::hash_map::RandomState
);

impl<T: ?Sized> RetainedBytes for &T {
    fn has_owned_children() -> bool {
        false
    }
    fn visit<'value>(&'value self, _: &mut Visitor<'value, '_>) -> Result<(), Error> {
        Ok(())
    }
}

impl<T: ?Sized> RetainedBytes for PhantomData<T> {
    fn has_owned_children() -> bool {
        false
    }
    fn visit<'value>(&'value self, _: &mut Visitor<'value, '_>) -> Result<(), Error> {
        Ok(())
    }
}

impl RetainedBytes for String {
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        visitor.charge(self.capacity())
    }
}

impl RetainedBytes for std::path::PathBuf {
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        visitor.charge(self.capacity())
    }
}

impl<T: RetainedBytes> RetainedBytes for std::io::Cursor<T> {
    fn has_owned_children() -> bool {
        T::has_owned_children()
    }

    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        visitor.push(self.get_ref())
    }
}

impl<T: RetainedBytes> RetainedBytes for Vec<T> {
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        visitor.capacity::<T>(self.capacity())?;
        if T::has_owned_children() {
            for value in self {
                visitor.push(value)?;
            }
        }
        Ok(())
    }
}

impl<T: RetainedBytes> RetainedBytes for VecDeque<T> {
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        visitor.capacity::<T>(self.capacity())?;
        if T::has_owned_children() {
            for value in self {
                visitor.push(value)?;
            }
        }
        Ok(())
    }
}

impl<T: RetainedBytes, const N: usize> RetainedBytes for [T; N] {
    fn has_owned_children() -> bool {
        T::has_owned_children()
    }
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        for value in self {
            visitor.push(value)?;
        }
        Ok(())
    }
}

impl<T: RetainedBytes> RetainedBytes for Option<T> {
    fn has_owned_children() -> bool {
        T::has_owned_children()
    }
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        if let Some(value) = self {
            visitor.push(value)?;
        }
        Ok(())
    }
}

impl<T: RetainedBytes> RetainedBytes for std::ops::Range<T> {
    fn has_owned_children() -> bool {
        T::has_owned_children()
    }

    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        visitor.push(&self.start)?;
        visitor.push(&self.end)
    }
}

impl<T: RetainedBytes> RetainedBytes for Box<T> {
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        visitor.charge(size_of::<T>())?;
        visitor.push(self.as_ref())
    }
}

impl<T: RetainedBytes> RetainedBytes for Box<[T]> {
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        visitor.charge(size_of_val(self.as_ref()))?;
        if T::has_owned_children() {
            for value in self {
                visitor.push(value)?;
            }
        }
        Ok(())
    }
}

impl RetainedBytes for Box<str> {
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        visitor.charge(self.len())
    }
}

fn arc_layout<T: ?Sized>(value: &T) -> Result<usize, Error> {
    Layout::new::<(AtomicUsize, AtomicUsize)>()
        .extend(Layout::for_value(value))
        .map(|(layout, _)| layout.pad_to_align().size())
        .map_err(|_| Error::ByteBudget)
}

impl<T: RetainedBytes> RetainedBytes for Arc<T> {
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        if visitor.mark_shared(Arc::as_ptr(self).cast::<()>() as usize)? {
            visitor.charge(arc_layout(self.as_ref())?)?;
            visitor.push(self.as_ref())?;
        }
        Ok(())
    }
}

impl<T: RetainedBytes> RetainedBytes for Arc<[T]> {
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        if visitor.mark_shared(Arc::as_ptr(self).cast::<()>() as usize)? {
            visitor.charge(arc_layout(self.as_ref())?)?;
            if T::has_owned_children() {
                for value in self.iter() {
                    visitor.push(value)?;
                }
            }
        }
        Ok(())
    }
}

impl RetainedBytes for Arc<str> {
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        if visitor.mark_shared(Arc::as_ptr(self).cast::<()>() as usize)? {
            visitor.charge(arc_layout(self.as_ref())?)?;
        }
        Ok(())
    }
}

impl<T: RetainedBytes> RetainedBytes for RefCell<T> {
    fn has_owned_children() -> bool {
        T::has_owned_children()
    }
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        let value = self.try_borrow().map_err(|_| Error::Borrowed)?;
        visitor.scoped(&*value)
    }
}

impl<T: Copy + RetainedBytes> RetainedBytes for Cell<T> {
    fn has_owned_children() -> bool {
        T::has_owned_children()
    }
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        visitor.scoped(&self.get())
    }
}

impl<T: RetainedBytes> RetainedBytes for Mutex<T> {
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        let value = self.try_lock().map_err(|_| Error::Locked)?;
        visitor.scoped(&*value)
    }
}

impl<T: RetainedBytes> RetainedBytes for RwLock<T> {
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        let value = self.try_read().map_err(|_| Error::Locked)?;
        visitor.scoped(&*value)
    }
}

impl<T: RetainedBytes> RetainedBytes for std::sync::OnceLock<T> {
    fn has_owned_children() -> bool {
        T::has_owned_children()
    }

    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        if let Some(value) = self.get() {
            visitor.push(value)?;
        }
        Ok(())
    }
}

impl<K: RetainedBytes, V: RetainedBytes, S: BuildHasher + RetainedBytes> RetainedBytes
    for HashMap<K, V, S>
{
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        visitor.capacity::<(K, V)>(self.capacity())?;
        visitor.push(self.hasher())?;
        for (key, value) in self {
            visitor.push(key)?;
            visitor.push(value)?;
        }
        Ok(())
    }
}

impl<T: RetainedBytes, S: BuildHasher + RetainedBytes> RetainedBytes for HashSet<T, S> {
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        visitor.capacity::<T>(self.capacity())?;
        visitor.push(self.hasher())?;
        if T::has_owned_children() {
            for value in self {
                visitor.push(value)?;
            }
        }
        Ok(())
    }
}

impl<B: ToOwned + ?Sized> RetainedBytes for Cow<'_, B>
where
    B::Owned: RetainedBytes,
{
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        if let Cow::Owned(value) = self {
            visitor.push(value)?;
        }
        Ok(())
    }
}

macro_rules! tuple {
    ($($kind:ident:$position:tt),+ $(,)?) => {
        impl<$($kind: RetainedBytes),+> RetainedBytes for ($($kind,)+) {
            fn has_owned_children() -> bool { false $(|| $kind::has_owned_children())+ }
            fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
                $(visitor.push(&self.$position)?;)+
                Ok(())
            }
        }
    };
}

tuple!(A:0);
tuple!(A:0, B:1);
tuple!(A:0, B:1, C:2);
tuple!(A:0, B:1, C:2, D:3);
tuple!(A:0, B:1, C:2, D:3, E:4);
tuple!(A:0, B:1, C:2, D:3, E:4, F:5);
tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6);
tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7);
tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8);
tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9);
