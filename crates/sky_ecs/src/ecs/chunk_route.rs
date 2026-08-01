/// Diagnostic counts for the World-local chunk route table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RouteTableStats {
    /// Number of route slots currently assigned to live chunks.
    pub live_chunk_routes: usize,
    /// Total addressable route slots, including internal vacancies.
    pub route_slots: usize,
    /// Number of vacant slots retained for later chunk-ID reuse.
    pub vacant_route_slots: usize,
}

/// Stable World-local identity for one physical chunk.
///
/// Entity records use this key instead of storing both an archetype-storage
/// index and a local chunk index. The directory translates the key only for
/// structural operations; typed accessors index their component route table
/// directly with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChunkId(pub(crate) u32);

impl ChunkId {
    pub(crate) const UNASSIGNED: Self = Self(u32::MAX);

    #[inline(always)]
    pub(crate) fn is_assigned(self) -> bool {
        self != Self::UNASSIGNED
    }

    #[inline(always)]
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChunkAddress {
    pub(crate) data_index: usize,
    pub(crate) chunk_index: usize,
}

#[derive(Clone, Copy)]
struct ChunkDirectoryEntry {
    data_index: u32,
    chunk_index: u32,
}

impl ChunkDirectoryEntry {
    const VACANT: Self = Self {
        data_index: u32::MAX,
        chunk_index: 0,
    };

    #[inline(always)]
    fn is_vacant(self) -> bool {
        self.data_index == u32::MAX
    }
}

/// Owns the lifecycle of stable chunk route keys for one World.
#[derive(Default)]
pub(crate) struct ChunkDirectory {
    entries: Vec<ChunkDirectoryEntry>,
    free: Vec<u32>,
    epoch: u64,
    last_resolved: Option<(ChunkId, ChunkAddress)>,
}

impl ChunkDirectory {
    #[inline]
    fn bump_epoch(&mut self) {
        self.epoch = self
            .epoch
            .checked_add(1)
            .expect("world route-table epoch exhausted");
    }

    #[inline(always)]
    pub(crate) fn epoch(&self) -> u64 {
        self.epoch
    }

    pub(crate) fn stats(&self) -> RouteTableStats {
        RouteTableStats {
            live_chunk_routes: self.entries.len() - self.free.len(),
            route_slots: self.entries.len(),
            vacant_route_slots: self.free.len(),
        }
    }

    pub(crate) fn shrink_tail(&mut self) -> bool {
        let old_len = self.entries.len();
        while self.entries.last().is_some_and(|entry| entry.is_vacant()) {
            self.entries.pop();
        }
        if self.entries.len() != old_len {
            let new_len = self.entries.len();
            self.free.retain(|&id| (id as usize) < new_len);
            self.bump_epoch();
        }
        self.entries.shrink_to_fit();
        self.free.shrink_to_fit();
        self.entries.len() != old_len
    }

    pub(crate) fn ensure(
        &mut self,
        id: &mut ChunkId,
        data_index: usize,
        chunk_index: usize,
    ) -> ChunkId {
        if id.is_assigned() {
            debug_assert_eq!(
                self.resolve(*id),
                Some(ChunkAddress {
                    data_index,
                    chunk_index,
                })
            );
            return *id;
        }

        let entry = ChunkDirectoryEntry {
            data_index: u32::try_from(data_index)
                .ok()
                .filter(|&index| index != u32::MAX)
                .expect("World storage index limit exhausted"),
            chunk_index: u32::try_from(chunk_index).expect("chunk index limit exhausted"),
        };
        let raw_id = if let Some(raw_id) = self.free.pop() {
            let slot = &mut self.entries[raw_id as usize];
            debug_assert!(slot.is_vacant());
            *slot = entry;
            raw_id
        } else {
            assert!(
                self.entries.len() < u32::MAX as usize,
                "chunk route limit exhausted"
            );
            let raw_id = self.entries.len() as u32;
            self.entries.push(entry);
            self.bump_epoch();
            raw_id
        };

        *id = ChunkId(raw_id);
        *id
    }

    #[inline(always)]
    pub(crate) fn resolve(&self, id: ChunkId) -> Option<ChunkAddress> {
        let entry = *self.entries.get(id.index())?;
        (!entry.is_vacant()).then_some(ChunkAddress {
            data_index: entry.data_index as usize,
            chunk_index: entry.chunk_index as usize,
        })
    }

    /// Resolves a live route for a mutable World operation, reusing the most
    /// recent address when consecutive entities occupy the same chunk.
    #[inline(always)]
    pub(crate) fn resolve_live_cached(&mut self, id: ChunkId) -> ChunkAddress {
        if let Some((cached_id, address)) = self.last_resolved {
            if cached_id == id {
                return address;
            }
        }

        let address = self
            .resolve(id)
            .expect("live entity must reference a registered chunk");
        self.last_resolved = Some((id, address));
        address
    }

    pub(crate) fn release(&mut self, id: ChunkId) {
        if !id.is_assigned() {
            return;
        }
        let entry = self
            .entries
            .get_mut(id.index())
            .expect("registered chunk route must stay in bounds");
        assert!(!entry.is_vacant(), "chunk route released twice");
        *entry = ChunkDirectoryEntry::VACANT;
        self.free.push(id.0);
        if self
            .last_resolved
            .is_some_and(|(cached_id, _)| cached_id == id)
        {
            self.last_resolved = None;
        }
    }

    #[inline(always)]
    pub(crate) fn slot_count(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn clear(&mut self) {
        self.bump_epoch();
        self.entries.clear();
        self.free.clear();
        self.last_resolved = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn released_ids_are_reused_with_a_new_address() {
        let mut directory = ChunkDirectory::default();
        let mut first = ChunkId::UNASSIGNED;
        directory.ensure(&mut first, 2, 3);
        assert_eq!(
            directory.resolve(first),
            Some(ChunkAddress {
                data_index: 2,
                chunk_index: 3
            })
        );

        directory.release(first);
        assert_eq!(directory.resolve(first), None);

        let mut reused = ChunkId::UNASSIGNED;
        directory.ensure(&mut reused, 5, 7);
        assert_eq!(reused, first);
        assert_eq!(
            directory.resolve(reused),
            Some(ChunkAddress {
                data_index: 5,
                chunk_index: 7
            })
        );
    }

    #[test]
    fn cached_live_resolution_is_invalidated_before_id_reuse() {
        let mut directory = ChunkDirectory::default();
        let mut first = ChunkId::UNASSIGNED;
        directory.ensure(&mut first, 2, 3);
        assert_eq!(
            directory.resolve_live_cached(first),
            ChunkAddress {
                data_index: 2,
                chunk_index: 3,
            }
        );

        directory.release(first);
        let mut reused = ChunkId::UNASSIGNED;
        directory.ensure(&mut reused, 5, 7);
        assert_eq!(reused, first);
        assert_eq!(
            directory.resolve_live_cached(reused),
            ChunkAddress {
                data_index: 5,
                chunk_index: 7,
            }
        );
    }
}
