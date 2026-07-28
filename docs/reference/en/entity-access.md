# Entity access

[API index](../../API.md) · [中文](../zh-CN/entity-access.md) · [Rustdoc](https://docs.rs/sky_ecs)

Module: `sky_ecs`

## Synopsis

```rust
pub struct EntityAccessor<'w, T> { /* private fields */ }
pub struct EntityAccessorMut<'w, T> { /* private fields */ }
pub struct PreparedEntityAccessor<T> { /* private fields */ }
pub struct BoundEntityAccessor<'s, 'w, T> { /* private fields */ }
pub struct BoundEntityAccessorMut<'s, 'w, T> { /* private fields */ }
pub struct PreparedEntityAccess<'w, T> { /* private fields */ }
pub struct PreparedEntityAccessMut<'w, T> { /* private fields */ }
pub struct PreparedEntityView<Q> { /* private fields */ }
pub struct BoundEntityView<'s, 'w, Q> { /* private fields */ }
pub struct BoundEntityViewMut<'s, 'w, Q> { /* private fields */ }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrepareAccessError {
    InvalidEntity { index: usize, entity: EntityId },
    MissingComponent { index: usize, entity: EntityId },
    DuplicateEntity {
        first_index: usize,
        duplicate_index: usize,
        entity: EntityId,
    },
}
```

`EntityAccessor<T>` is the immediate general-purpose path for arbitrary entity IDs.
`PreparedEntityAccessor<T>` retains the single-component route table across binds.
`PreparedEntityAccess<T>` resolves one fixed sequence up front and is the batch path for
repeated ordered access.
`PreparedEntityView<Q>` prepares one or more query components by chunk route and keeps
that allocation reusable across structurally changing frames.

## World entry points

| Declaration | Result |
|---|---|
| `pub fn accessor<T: 'static>(&self) -> EntityAccessor<'_, T>` | Shared arbitrary-ID accessor. |
| `pub fn accessor_mut<T: 'static>(&mut self) -> EntityAccessorMut<'_, T>` | Exclusive arbitrary-ID accessor. |
| `pub fn prepare_access<T: 'static>(&self, entities: &[EntityId]) -> Result<PreparedEntityAccess<'_, T>, PrepareAccessError>` | Validated shared fixed-sequence plan. |
| `pub fn prepare_access_mut<T: 'static>(&mut self, entities: &[EntityId]) -> Result<PreparedEntityAccessMut<'_, T>, PrepareAccessError>` | Validated unique fixed-sequence mutable plan. |

All four values retain the corresponding World borrow for their lifetime. Safe structural
mutation therefore cannot invalidate cached routes or pointers while an accessor/plan is live.
There is no hidden World-owned prepared-access cache and no per-item epoch refresh.

## `PreparedEntityAccessor`

```rust
let mut prepared = PreparedEntityAccessor::<Position>::new();

for frame in frames {
    update_targets(frame);
    let positions = prepared.bind(&world);
    for entity in frame.targets {
        use_position(positions.get(entity)?);
    }
}
```

`bind` and `bind_mut` reacquire the current entity-record slice every time, while retaining
the component route allocation and resolved column bases. Pure row churn therefore needs no
route rebuild. Switching Worlds, chunk creation or retirement, route reuse, tiny promotion,
clear, and explicit route-table shrinking rebuild the cache through the column-base epoch.

The shared bound accessor returns `&T`; the exclusive bound accessor returns `&mut T` tied to
its current mutable borrow. Every lookup still validates the supplied entity generation and
reports `None` for a stale ID or an entity without `T`. `cache_stats()` exposes rebuild and
route-slot diagnostics.

## `PreparedEntityView`

```rust
let mut prepared = PreparedEntityView::<(&TargetSlot, &mut Cooldown)>::new();
let mut view = prepared.bind_mut(&mut world);
let (target, cooldown) = view.get_mut(entity)?;
```

`bind` accepts a read-only `QuerySpec`; `bind_mut` accepts shared, mutable, tuple, and
optional parameters. Binding reuses component bases while the World's column-base epoch is
unchanged. Chunk creation/retirement, route reuse, tiny promotion, clear, and explicit route
shrink rebuild the cache; ordinary row churn does not. `cache_stats()` exposes rebuild and
route-slot diagnostics. Each `get` or `get_mut` performs one generation/route lookup and builds
the complete query item from that route. Filters are intentionally not part of this API.

An optional-only query distinguishes a valid entity with missing components from an invalid
entity: `PreparedEntityView<Option<&A>>::get` returns `Some(None)` for the former and `None`
for the latter. Mutable items are tied to the current mutable borrow of the bound view.

`World::route_table_stats()` reports live, allocated, and vacant chunk-route slots.
`World::shrink_route_tables()` removes only trailing vacant slots and never renumbers a
live chunk; internal holes remain reusable.

## `EntityAccessor`

```rust
pub fn get(&self, entity: EntityId) -> Option<&'w T>;
```

Every call validates the entity generation, resolves its live chunk route, and checks whether
that chunk has `T`. It returns `None` for an invalid/stale ID or a missing component.
Construction allocates a route table with one slot per live chunk-route slot and resolves
matching component columns once.

## `EntityAccessorMut`

```rust
pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut T>;
```

Semantics match `EntityAccessor::get`. The returned reference is tied to the current mutable
borrow of the accessor, so two results cannot overlap through safe code.

## `PreparedEntityAccess`

| Declaration | Effect |
|---|---|
| `pub fn len(&self) -> usize` | Number of prepared entries. |
| `pub fn is_empty(&self) -> bool` | Tests whether the sequence is empty. |
| `pub fn get(&self, index: usize) -> Option<&T>` | Indexed prepared access; `None` only for an out-of-range plan index. |
| `pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator + '_` | Iterates in the original input order. |

Preparation is all-or-nothing. Every ID must be live and contain `T`. Duplicate entities are
allowed because only shared references are produced.

## `PreparedEntityAccessMut`

| Declaration | Effect |
|---|---|
| `pub fn len(&self) -> usize` | Number of prepared entries. |
| `pub fn is_empty(&self) -> bool` | Tests whether the sequence is empty. |
| `pub fn get(&self, index: usize) -> Option<&T>` | Shared indexed access. |
| `pub fn get_mut(&mut self, index: usize) -> Option<&mut T>` | Exclusive indexed access. |
| `pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator + '_` | Shared iteration in input order. |
| `pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut T> + ExactSizeIterator + '_` | Exclusive iteration in input order. |

Mutable preparation rejects a repeated live `EntityId` before making the plan observable.
Returned mutable references are tied to a mutable borrow of the plan.

## Errors

| Variant | Condition |
|---|---|
| `InvalidEntity { index, entity }` | The entry is dead, stale, out of range, or otherwise not live in this World. |
| `MissingComponent { index, entity }` | The entity is live but lacks `T`. |
| `DuplicateEntity { first_index, duplicate_index, entity }` | Mutable preparation sees the same live ID more than once. |

Validation stops at the first failing input position. Read-only preparation never produces
`DuplicateEntity`.

## Complexity and allocation

Let `R` be the World chunk-route slot count and `N` the input length.

| Operation | Complexity / allocation |
|---|---|
| `accessor*` construction | O(R + matching chunks), one route-table allocation. |
| `EntityAccessor*::get*` | O(1), no allocation. |
| `PreparedEntityAccessor::bind*` | O(1) while the column-base epoch is unchanged; otherwise O(R + matching chunks). |
| Bound prepared accessor `get*` | O(1), no allocation and one entity-route validation. |
| `prepare_access` | O(R + matching chunks + N), one boxed pointer array. |
| `prepare_access_mut` | Expected O(R + matching chunks + N), one pointer array plus a temporary hash table for duplicate detection. |
| Prepared `get*` | O(1), no allocation. |
| Prepared `iter*` | O(N), no allocation during iteration and no entity/route/component checks per item. |
| `PreparedEntityView::bind*` | O(1) while the column-base epoch is unchanged; otherwise O(R + matching chunks × query width). |
| Bound entity-view `get*` | O(query width), no allocation and one entity-route validation. |

## Minimal example

```rust
use sky_ecs::World;

struct Position(u32);

let mut world = World::new();
let ids = [world.spawn((Position(1),)), world.spawn((Position(2),))];

let prepared = world.prepare_access::<Position>(&ids).unwrap();
assert_eq!(prepared.iter().map(|p| p.0).collect::<Vec<_>>(), [1, 2]);
```

## See also

- [Core `World::get` and `World::get_mut`](core.md)
- [Typed queries](queries.md)
