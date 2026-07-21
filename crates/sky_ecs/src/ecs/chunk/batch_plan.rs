use super::{Archetype, ChunkLayout, CHUNK_SIZE_TIERS, CHUNK_TIER_COUNT, MAX_CHUNK_SIZE};

const MAX_REMAINDER_CHUNKS: usize = 4;
const OVERSIZED_CLASS_COUNT: usize = 3;
const BATCH_CLASS_COUNT: usize = CHUNK_TIER_COUNT + OVERSIZED_CLASS_COUNT;
const OVERSIZED_CHUNK_SIZES: [usize; OVERSIZED_CLASS_COUNT] = [
    MAX_CHUNK_SIZE * 4,
    MAX_CHUNK_SIZE * 4 * 4,
    MAX_CHUNK_SIZE * 4 * 4 * 4,
];

/// The fixed-size-class plan for one known batch operation.
///
/// Full 256 MiB classes cover the unavoidable prefix. The remainder uses a
/// deterministic two-tier plan with at most four chunks: one largest class
/// that does not exceed the remainder, followed by either that class or its
/// immediate lower class. Four equal classes carry into one next-tier class.
#[derive(Clone, Copy, Default)]
pub(crate) struct BatchGrowthPlan {
    layouts: [Option<ChunkLayout>; BATCH_CLASS_COUNT],
    counts: [usize; BATCH_CLASS_COUNT],
}

impl BatchGrowthPlan {
    pub(super) fn for_remaining(
        archetype: Archetype,
        remaining: usize,
        minimum_chunk_size: usize,
    ) -> Self {
        if remaining == 0 {
            return Self::default();
        }

        let Some(component_bytes) = archetype
            .components
            .iter()
            .try_fold(0usize, |bytes, component| bytes.checked_add(component.size))
        else {
            return Self::default();
        };

        let mut layouts = [None; BATCH_CLASS_COUNT];
        let chunk_sizes = CHUNK_SIZE_TIERS.into_iter().chain(OVERSIZED_CHUNK_SIZES);
        for (slot, chunk_size) in layouts.iter_mut().zip(chunk_sizes) {
            if chunk_size < minimum_chunk_size {
                continue;
            }
            *slot = ChunkLayout::try_for_archetype_with_component_bytes(
                archetype,
                chunk_size,
                component_bytes,
            );
        }

        let Some(counts) = plan_counts(&layouts, remaining) else {
            return Self::default();
        };
        Self { layouts, counts }
    }

    pub(super) fn remaining_chunk_count(&self) -> usize {
        self.counts
            .iter()
            .copied()
            .fold(0usize, usize::saturating_add)
    }

    /// Returns the next standard layout from largest to smallest.
    pub(super) fn take_next_layout(&mut self) -> Option<ChunkLayout> {
        for index in (0..BATCH_CLASS_COUNT).rev() {
            if self.counts[index] > 0 {
                self.counts[index] -= 1;
                return self.layouts[index];
            }
        }
        None
    }
}

fn plan_counts(
    layouts: &[Option<ChunkLayout>; BATCH_CLASS_COUNT],
    requested_entities: usize,
) -> Option<[usize; BATCH_CLASS_COUNT]> {
    let largest_index = BATCH_CLASS_COUNT - 1;
    let largest_capacity = layouts[largest_index]?.max_entity_count();
    let full_largest_chunks = requested_entities / largest_capacity;
    let remainder = requested_entities % largest_capacity;

    let mut counts = [0usize; BATCH_CLASS_COUNT];
    counts[largest_index] = full_largest_chunks;
    if remainder > 0 {
        for (count, remainder_count) in counts
            .iter_mut()
            .zip(greedy_remainder_counts(layouts, remainder)?)
        {
            *count = count.saturating_add(remainder_count);
        }
    }
    Some(counts)
}

fn greedy_remainder_counts(
    layouts: &[Option<ChunkLayout>; BATCH_CLASS_COUNT],
    requested_entities: usize,
) -> Option<[usize; BATCH_CLASS_COUNT]> {
    let mut class_index = layouts
        .iter()
        .rposition(|layout| {
            layout.is_some_and(|layout| layout.max_entity_count() <= requested_entities)
        })
        .or_else(|| layouts.iter().position(Option::is_some))?;

    let initial_capacity = layouts[class_index]?.max_entity_count();
    if initial_capacity.saturating_mul(MAX_REMAINDER_CHUNKS) < requested_entities {
        class_index =
            ((class_index + 1)..BATCH_CLASS_COUNT).find(|&index| layouts[index].is_some())?;
    }

    let mut counts = [0usize; BATCH_CLASS_COUNT];
    let class_capacity = layouts[class_index]?.max_entity_count();
    counts[class_index] = 1;
    let remaining = requested_entities.saturating_sub(class_capacity);
    if remaining > 0 {
        let lower_index = (0..class_index)
            .rev()
            .find(|&index| layouts[index].is_some());
        let (remainder_index, remainder_chunks) = if let Some(lower_index) = lower_index {
            let lower_capacity = layouts[lower_index]?.max_entity_count();
            let lower_chunks = div_ceil(remaining, lower_capacity);
            if lower_chunks < MAX_REMAINDER_CHUNKS {
                (lower_index, lower_chunks)
            } else {
                (class_index, div_ceil(remaining, class_capacity))
            }
        } else {
            (class_index, div_ceil(remaining, class_capacity))
        };
        if remainder_chunks >= MAX_REMAINDER_CHUNKS {
            return None;
        }
        counts[remainder_index] = counts[remainder_index].saturating_add(remainder_chunks);
    }

    carry_equal_classes(layouts, &mut counts);
    Some(counts)
}

fn div_ceil(value: usize, divisor: usize) -> usize {
    value / divisor + usize::from(value % divisor != 0)
}

fn carry_equal_classes(
    layouts: &[Option<ChunkLayout>; BATCH_CLASS_COUNT],
    counts: &mut [usize; BATCH_CLASS_COUNT],
) {
    for index in 0..BATCH_CLASS_COUNT - 1 {
        if counts[index] < 4 {
            continue;
        }
        let (Some(layout), Some(next_layout)) = (layouts[index], layouts[index + 1]) else {
            continue;
        };
        if next_layout.chunk_size() == layout.chunk_size().saturating_mul(4)
            && next_layout.max_entity_count() >= layout.max_entity_count().saturating_mul(4)
        {
            let carries = counts[index] / 4;
            counts[index] %= 4;
            counts[index + 1] = counts[index + 1].saturating_add(carries);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn radix_four_layouts() -> [Option<ChunkLayout>; BATCH_CLASS_COUNT] {
        std::array::from_fn(|index| {
            let size = 1usize << (index * 2);
            Some(ChunkLayout {
                chunk_size: size as u32,
                max_entity_count: size as u32,
            })
        })
    }

    #[test]
    fn equal_classes_carry_into_one_larger_chunk() {
        let layouts = radix_four_layouts();

        let counts = greedy_remainder_counts(&layouts, 4).unwrap();

        assert_eq!(counts[0], 0);
        assert_eq!(counts[1], 1);
        assert_eq!(counts.iter().sum::<usize>(), 1);
    }

    #[test]
    fn greedy_uses_only_the_largest_and_its_immediate_lower_class() {
        let layouts = radix_four_layouts();

        let counts = greedy_remainder_counts(&layouts, 10).unwrap();

        assert_eq!(counts[0], 0);
        assert_eq!(counts[1], 3);
        assert_eq!(counts.iter().sum::<usize>(), 3);
    }

    #[test]
    fn chunk_limit_avoids_a_nine_chunk_radix_remainder() {
        let layouts = radix_four_layouts();

        let counts = greedy_remainder_counts(&layouts, 252).unwrap();

        assert_eq!(counts[4], 1);
        assert_eq!(counts.iter().sum::<usize>(), 1);
    }

    #[test]
    fn largest_prefix_adds_no_more_than_four_remainder_chunks() {
        let layouts = radix_four_layouts();
        let largest_capacity = layouts.last().unwrap().unwrap().max_entity_count();
        let requested = largest_capacity * 2 + 10;

        let counts = plan_counts(&layouts, requested).unwrap();

        assert_eq!(counts.last().copied(), Some(2));
        assert!(counts.iter().sum::<usize>() <= 2 + MAX_REMAINDER_CHUNKS);
    }
}
