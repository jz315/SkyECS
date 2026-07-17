use rustc_hash::FxHashMap;
use std::cell::Cell;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::ptr;
use std::sync::RwLock;

use smallvec::SmallVec;

use crate::ecs::{component_type, ComponentType};

/// Maximum number of component columns represented by one archetype.
pub const MAX_COMPONENTS: usize = 32;
const MISSING_COMPONENT_INDEX: u8 = u8::MAX;

lazy_static::lazy_static! {
    static ref ARCHETYPE_CACHE: RwLock<FxHashMap<Vec<usize>, Archetype>> = RwLock::new(FxHashMap::default());
}

thread_local! {
    static LAST_COMPONENT_LOOKUP: Cell<Option<(usize, usize, u8)>> = const { Cell::new(None) };
}

// Interned archetype metadata, including component columns and alignment.
#[derive(Debug)]
pub struct InternalArchetype {
    pub components: SmallVec<[ComponentType; MAX_COMPONENTS]>,
    pub(crate) drop_component_indices: SmallVec<[u8; MAX_COMPONENTS]>,
    pub alignment: usize,
}

impl InternalArchetype {
    // Creates empty archetype metadata before it is normalized and interned.
    fn new() -> Self {
        InternalArchetype {
            components: SmallVec::new(),
            drop_component_indices: SmallVec::new(),
            alignment: 1,
        }
    }

    // Adds one component before the final sort and validation pass.
    fn add_component(mut self, ty: ComponentType) -> Self {
        self.alignment = self.alignment.max(ty.align);
        self.components.push(ty);

        self
    }

    fn build(mut self) -> Self {
        assert!(
            self.components.len() <= MAX_COMPONENTS,
            "archetypes support at most {MAX_COMPONENTS} component types"
        );
        self.components.sort_by_key(|a| a.id());
        for window in self.components.windows(2) {
            if window[0].id() == window[1].id() {
                panic!(
                    "duplicate component type `{}` is not supported in archetypes",
                    window[0].name
                );
            }
        }
        self.drop_component_indices.clear();
        for (index, component) in self.components.iter().enumerate() {
            if component.needs_drop() {
                debug_assert!(index < u8::MAX as usize);
                self.drop_component_indices.push(index as u8);
            }
        }
        self
    }
}

impl InternalArchetype {
    // Returns whether this archetype contains the requested component type.
    #[inline(always)]
    pub fn has_component(&self, ty: &ComponentType) -> bool {
        self.query_component_index(ty).is_some()
    }

    #[inline(always)]
    pub fn query_component_index(&self, ty: &ComponentType) -> Option<usize> {
        let archetype_id = self as *const InternalArchetype as usize;
        let component_id = ty.id();

        if let Some((cached_archetype_id, cached_component_id, cached_index)) =
            LAST_COMPONENT_LOOKUP.with(Cell::get)
        {
            if cached_archetype_id == archetype_id && cached_component_id == component_id {
                return (cached_index != MISSING_COMPONENT_INDEX).then_some(cached_index as usize);
            }
        }

        let index = self
            .components
            .binary_search_by(|component| component.id().cmp(&ty.id()))
            .ok();

        debug_assert!(index.is_none_or(|index| index < MISSING_COMPONENT_INDEX as usize));
        LAST_COMPONENT_LOOKUP.with(|cache| {
            cache.set(Some((
                archetype_id,
                component_id,
                index.map_or(MISSING_COMPONENT_INDEX, |index| index as u8),
            )));
        });
        index
    }
}

impl fmt::Display for InternalArchetype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Archetype[")?;
        for (i, component) in self.components.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", component.name)?;
        }
        write!(f, "]")
    }
}
impl PartialEq for Archetype {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.archetype, other.archetype)
    }
}

impl Eq for Archetype {}

impl Hash for Archetype {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.id());
    }
}
#[derive(Debug, Clone, Copy)]
/// A process-interned component signature.
///
/// Archetype metadata intentionally lives for the process lifetime so this
/// handle can remain `Copy` and pointer comparisons stay constant-time. Tools
/// that generate schemas dynamically should reuse component combinations
/// instead of creating an unbounded stream of unique signatures.
pub struct Archetype {
    archetype: &'static InternalArchetype,
}

impl Archetype {
    fn new(archetype: &'static InternalArchetype) -> Self {
        Archetype { archetype }
    }

    #[inline(always)]
    pub fn id(&self) -> usize {
        self.archetype as *const InternalArchetype as usize
    }
}

impl Deref for Archetype {
    type Target = InternalArchetype;

    fn deref(&self) -> &Self::Target {
        self.archetype
    }
}

// Incremental builder used by typed, dynamic, and expert construction paths.
pub struct ArchetypeBuilder {
    internal_archetype: InternalArchetype,
}

impl ArchetypeBuilder {
    fn new() -> Self {
        ArchetypeBuilder {
            internal_archetype: InternalArchetype::new(),
        }
    }

    pub fn add_component(mut self, ty: ComponentType) -> Self {
        self.internal_archetype = self.internal_archetype.add_component(ty);
        self
    }

    pub fn add_rust_component<T: 'static>(self) -> Self {
        self.add_component(component_type::<T>())
    }

    pub fn build(self) -> Archetype {
        let built = self.internal_archetype.build();
        let key = built
            .components
            .iter()
            .map(|component| component.id())
            .collect::<SmallVec<[usize; 16]>>();

        if let Some(archetype) = ARCHETYPE_CACHE.read().unwrap().get(key.as_slice()).copied() {
            return archetype;
        }

        let mut cache = ARCHETYPE_CACHE.write().unwrap();
        if let Some(archetype) = cache.get(key.as_slice()).copied() {
            return archetype;
        }

        let archetype = Archetype::new(Box::leak(Box::new(built)));
        cache.insert(key.into_vec(), archetype);
        archetype
    }
}

pub(crate) fn interned_archetype_count() -> usize {
    ARCHETYPE_CACHE.read().unwrap().len()
}

pub fn create_archetype() -> ArchetypeBuilder {
    ArchetypeBuilder::new()
}

#[cfg(test)]
mod tests {
    use super::{component_type, create_archetype, InternalArchetype, MAX_COMPONENTS};
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[derive(Clone, Copy)]
    #[allow(dead_code)]
    struct Position(f32);

    #[derive(Clone, Copy)]
    #[allow(dead_code)]
    struct Velocity(f32);

    #[test]
    #[should_panic(expected = "duplicate component type")]
    fn archetype_builder_rejects_duplicate_component_types() {
        let _ = create_archetype()
            .add_rust_component::<Position>()
            .add_rust_component::<Position>()
            .build();
    }

    #[test]
    #[should_panic(expected = "archetypes support at most 32 component types")]
    fn archetype_rejects_more_columns_than_storage_can_represent() {
        let mut archetype = InternalArchetype::new();
        for _ in 0..=MAX_COMPONENTS {
            archetype.components.push(component_type::<Position>());
        }
        let _ = archetype.build();
    }

    #[test]
    fn concurrent_archetype_builds_reuse_cached_instance() {
        let thread_count = 16usize;
        let barrier = Arc::new(Barrier::new(thread_count));
        let mut handles = Vec::with_capacity(thread_count);

        for _ in 0..thread_count {
            let barrier = barrier.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                create_archetype()
                    .add_rust_component::<Position>()
                    .add_rust_component::<Velocity>()
                    .build()
                    .id()
            }));
        }

        let ids = handles
            .into_iter()
            .map(|handle| handle.join().expect("archetype build thread panicked"))
            .collect::<Vec<_>>();

        assert!(ids.windows(2).all(|window| window[0] == window[1]));
    }
}
