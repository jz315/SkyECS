use super::routes::{ComponentRoutes, ResolveError};
use crate::ecs::{EntityId, World};
use core::fmt;
use core::marker::PhantomData;
use core::ptr::NonNull;
use rustc_hash::FxHashMap;

/// Error returned while preparing a fixed entity access sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrepareAccessError {
    /// The input contains a dead, stale, or otherwise invalid entity ID.
    InvalidEntity {
        /// Position of the invalid entity in the input sequence.
        index: usize,
        /// Entity ID that failed validation.
        entity: EntityId,
    },
    /// A live input entity does not contain the requested component.
    MissingComponent {
        /// Position of the entity in the input sequence.
        index: usize,
        /// Entity ID that does not contain the requested component.
        entity: EntityId,
    },
    /// A mutable access sequence contains the same live entity more than once.
    DuplicateEntity {
        /// Position of the first occurrence.
        first_index: usize,
        /// Position of the repeated occurrence.
        duplicate_index: usize,
        /// Repeated entity ID.
        entity: EntityId,
    },
}

impl fmt::Display for PrepareAccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidEntity { index, entity } => write!(
                formatter,
                "entity {:?} at access index {index} is not live in this world",
                entity
            ),
            Self::MissingComponent { index, entity } => write!(
                formatter,
                "entity {:?} at access index {index} does not contain the requested component",
                entity
            ),
            Self::DuplicateEntity {
                first_index,
                duplicate_index,
                entity,
            } => write!(
                formatter,
                "entity {:?} is duplicated at access indices {first_index} and {duplicate_index}",
                entity
            ),
        }
    }
}

impl std::error::Error for PrepareAccessError {}

fn resolve_error(index: usize, entity: EntityId, error: ResolveError) -> PrepareAccessError {
    match error {
        ResolveError::InvalidEntity => PrepareAccessError::InvalidEntity { index, entity },
        ResolveError::MissingComponent => PrepareAccessError::MissingComponent { index, entity },
    }
}

fn prepare_pointers<T: 'static>(
    world: &World,
    entities: &[EntityId],
) -> Result<Box<[NonNull<T>]>, PrepareAccessError> {
    let routes = ComponentRoutes::<T>::new(world);
    let mut pointers = Vec::with_capacity(entities.len());
    for (index, &entity) in entities.iter().enumerate() {
        pointers.push(
            routes
                .resolve(world, entity)
                .map_err(|error| resolve_error(index, entity, error))?,
        );
    }
    Ok(pointers.into_boxed_slice())
}

fn prepare_unique_pointers<T: 'static>(
    world: &World,
    entities: &[EntityId],
) -> Result<Box<[NonNull<T>]>, PrepareAccessError> {
    let routes = ComponentRoutes::<T>::new(world);
    let mut pointers = Vec::with_capacity(entities.len());
    let mut first_indices = FxHashMap::default();
    first_indices.reserve(entities.len());

    for (index, &entity) in entities.iter().enumerate() {
        let pointer = routes
            .resolve(world, entity)
            .map_err(|error| resolve_error(index, entity, error))?;
        if let Some(&first_index) = first_indices.get(&entity) {
            return Err(PrepareAccessError::DuplicateEntity {
                first_index,
                duplicate_index: index,
                entity,
            });
        }
        first_indices.insert(entity, index);
        pointers.push(pointer);
    }

    Ok(pointers.into_boxed_slice())
}

/// Direct read-only component access prepared for one fixed entity sequence.
///
/// Construction validates every entity and resolves its component address.
/// Iteration therefore preserves the input order without repeating entity,
/// generation, chunk-route, or component-presence checks. The plan holds a
/// shared borrow of its world, so structural changes cannot invalidate those
/// addresses while it is in use.
#[must_use = "prepared entity access does nothing until it is read"]
pub struct PreparedEntityAccess<'w, T> {
    _world: &'w World,
    pointers: Box<[NonNull<T>]>,
}

impl<'w, T: 'static> PreparedEntityAccess<'w, T> {
    pub(crate) fn new(world: &'w World, entities: &[EntityId]) -> Result<Self, PrepareAccessError> {
        Ok(Self {
            _world: world,
            pointers: prepare_pointers(world, entities)?,
        })
    }

    /// Number of component addresses in the prepared sequence.
    #[inline]
    pub fn len(&self) -> usize {
        self.pointers.len()
    }

    /// Returns `true` when the prepared sequence is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pointers.is_empty()
    }

    /// Returns the component at `index` in the prepared sequence.
    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&T> {
        let pointer = self.pointers.get(index)?;
        Some(unsafe {
            // Every stored pointer was validated against `world` during
            // construction, and the retained World borrow prevents relocation.
            &*pointer.as_ptr()
        })
    }

    /// Iterates over components in the exact order supplied during preparation.
    #[inline]
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator + '_ {
        self.pointers.iter().map(|pointer| unsafe {
            // Every pointer was validated during construction. `self.world`
            // keeps the originating World immutably borrowed and alive.
            &*pointer.as_ptr()
        })
    }
}

/// Direct exclusive component access prepared for one fixed unique entity sequence.
///
/// Construction rejects duplicate entities before exposing the plan. Together
/// with the exclusive World lifetime and references tied to mutable plan
/// borrows, this guarantees that [`iter_mut`](Self::iter_mut) cannot yield
/// overlapping mutable component references.
#[must_use = "prepared mutable entity access does nothing until it is read or written"]
pub struct PreparedEntityAccessMut<'w, T> {
    pointers: Box<[NonNull<T>]>,
    marker: PhantomData<&'w mut World>,
}

impl<'w, T: 'static> PreparedEntityAccessMut<'w, T> {
    pub(crate) fn new(
        world: &'w mut World,
        entities: &[EntityId],
    ) -> Result<Self, PrepareAccessError> {
        let pointers = prepare_unique_pointers(world, entities)?;
        Ok(Self {
            pointers,
            marker: PhantomData,
        })
    }

    /// Number of component addresses in the prepared sequence.
    #[inline]
    pub fn len(&self) -> usize {
        self.pointers.len()
    }

    /// Returns `true` when the prepared sequence is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pointers.is_empty()
    }

    /// Returns the component at `index` in the prepared sequence.
    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&T> {
        let pointer = self.pointers.get(index)?;
        Some(unsafe {
            // The exclusive World lifetime keeps every validated pointer live;
            // this shared plan borrow permits only shared component references.
            &*pointer.as_ptr()
        })
    }

    /// Returns the component at `index` with exclusive access.
    #[inline(always)]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let pointer = self.pointers.get_mut(index)?;
        Some(unsafe {
            // Duplicate entities were rejected during construction, and the
            // returned reference is tied to this exclusive plan borrow.
            &mut *pointer.as_ptr()
        })
    }

    /// Iterates immutably over components in prepared order.
    #[inline]
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator + '_ {
        self.pointers.iter().map(|pointer| unsafe {
            // Every pointer remains valid under the retained exclusive World
            // lifetime; this method only creates shared references.
            &*pointer.as_ptr()
        })
    }

    /// Iterates mutably over components in prepared order.
    #[inline]
    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut T> + ExactSizeIterator + '_ {
        self.pointers.iter_mut().map(|pointer| unsafe {
            // Duplicate live entities were rejected, so non-ZST component
            // ranges are disjoint. Distinct ZST rows occupy no bytes. The
            // iterator and yielded references are tied to this mutable borrow.
            &mut *pointer.as_ptr()
        })
    }
}
