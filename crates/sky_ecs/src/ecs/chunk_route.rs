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
}

impl ChunkDirectory {
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
    }

    #[inline(always)]
    pub(crate) fn slot_count(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.free.clear();
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
}
