# Typed queries

[API index](../../API.md) · [中文](../zh-CN/queries.md) · [Rustdoc](https://docs.rs/sky_ecs)

Module: `sky_ecs`

## Synopsis

```rust
pub struct Query<'w, Q, Flt = (), Shape = Q::Arity> { /* private fields */ }
pub struct QueryMut<'w, Q, Flt = (), Shape = Q::Arity> { /* private fields */ }
pub struct PreparedQuery<Q, Flt = (), Shape = Q::Arity> { /* private fields */ }

pub trait QueryFilter: /* sealed */ { /* hidden implementation members */ }
pub struct With<T>(/* private */);
pub struct Without<T>(/* private */);
pub struct Any<F>(/* private */);
```

Supported query parameters are `&T`, `&mut T`, `Option<&T>`, and
`Option<&mut T>`, either alone or in tuples of up to sixteen component types.
The same component type may not appear twice in one query, regardless of optionality or access
mode. `#[derive(QueryData)]` provides named query declarations and named
entity-level return values; iteration callbacks still receive one argument per
field in declaration order. See [macros and types](plugins-types.md).

## World-bound queries

```rust
pub fn World::query<Q>(&self) -> Query<'_, Q>
where
    Q: ReadOnlyQuerySpec + 'static;

pub fn World::query_mut<Q>(&mut self) -> QueryMut<'_, Q>
where
    Q: QuerySpec + 'static;
```

`Query` accepts only read-only specifications. `QueryMut` accepts read/write mixtures and
holds an exclusive World borrow. Query preparation is lazy; the World reuses match metadata
between recreated queries of the same `(Q, Flt)` type.

### `Query` members

| Declaration | Callback contract |
|---|---|
| `for_each_chunk<F>(&self, f: F)` | One component-slice argument per query parameter. |
| `for_each<F>(&self, f: F)` | One component-reference argument per query parameter. |
| `for_each_with_entity<F>(&self, f: F)` | `EntityId`, followed by the component-reference arguments. |
| `for_each_chunk_with_entities<F>(&self, f: F)` | Entity-ID slice, followed by the aligned component slices. |
| `par_for_each_chunk<F>(&self, f: F)` | Chunk is `Send`; `F: Fn(...) + Send + Sync`. |
| `par_for_each<F>(&self, f: F)` | Item is `Send`; `F: Fn(...) + Send + Sync`. |
| `par_for_each_with_entity<F>(&self, f: F)` | Parallel entity-ID counterpart. |
| `par_for_each_chunk_with_entities<F>(&self, f: F)` | Parallel aligned entity/chunk counterpart. |
| `count(&self) -> usize` | Counts matching live rows. |
| `is_empty(&self) -> bool` | Tests whether any live row matches. |
| `cached_archetype_count(&self) -> usize` | Number of cached matching archetypes. |
| `filter<Flt>(self) -> Query<'w, Q, Flt>` | Applies the complete filter type once. |

`QueryMut` provides the same members and signatures with `&mut self`; its `filter` returns
`QueryMut<'w, Q, Flt>`. Sequential visitation follows dense storage order. Parallel visitation
order is unspecified. In every chunk callback, entity and component slices have identical
length and row alignment.

Typed callbacks are expanded at compile time for query widths 1–16. For
`(&mut Position, &Velocity)`, entity iteration therefore accepts
`FnMut(&mut Position, &Velocity)` and chunk iteration accepts
`FnMut(&mut [Position], &[Velocity])`. A normal function can be passed
directly; a closure may capture state.

## `PreparedQuery`

```rust
impl<Q: QuerySpec, Flt: QueryFilter> PreparedQuery<Q, Flt> {
    pub fn new() -> Self;
    pub fn cached_archetype_count(&self) -> usize;
    pub fn count(&mut self, world: &World) -> usize;
    pub fn is_empty(&mut self, world: &World) -> bool;
}
```

`PreparedQuery` is an explicit reusable plan. Its iteration family matches the World-bound
query family but takes a World argument:

| Declaration | World argument |
|---|---|
| `for_each_chunk<W, F>(&mut self, world: W, f: F)` | `W: QueryWorld<Q>` |
| `for_each<W, F>(&mut self, world: W, f: F)` | `W: QueryWorld<Q>` |
| `for_each_with_entity<W, F>(&mut self, world: W, f: F)` | `W: QueryWorld<Q>` |
| `for_each_chunk_with_entities<W, F>(&mut self, world: W, f: F)` | `W: QueryWorld<Q>` |
| `par_for_each_chunk<W, F>(&mut self, world: W, f: F)` | Parallel chunk constraints. |
| `par_for_each<W, F>(&mut self, world: W, f: F)` | Parallel item constraints. |
| `par_for_each_with_entity<W, F>(&mut self, world: W, f: F)` | Parallel entity-ID counterpart. |
| `par_for_each_chunk_with_entities<W, F>(&mut self, world: W, f: F)` | Parallel aligned chunk counterpart. |

Read-only `Q` accepts `&World` or `&mut World`; a `Q` containing mutable access requires
`&mut World`. The cache refreshes automatically for a different World and after relevant
archetype/storage changes.

`for_each_chunk` itself accepts both ordinary functions and capturing closures.
Passing a reusable function keeps component slices as separate function
parameters for alias-sensitive kernels; captured state no longer requires a
separate API.

## Filters

| Type | Match rule |
|---|---|
| `()` | No additional filtering. |
| `With<T>` | Archetype contains `T`. |
| `Without<T>` | Archetype does not contain `T`. |
| `(F0, F1, ...)` | Logical AND; tuple widths 2–16. |
| `Any<(F0, F1, ...)>` | Logical OR; tuple widths 2–16. |

Filters operate at archetype granularity. `QueryFilter` is sealed and cannot be implemented
outside the crate.

## Complexity and allocation

- Initial preparation scans candidate archetypes and caches their column mapping; later calls
  scan only newly introduced archetypes or rebuild storage-dependent chunk metadata when needed.
- Sequential iteration is O(matching chunks + matching entities); there is no per-entity dynamic
  type lookup.
- `count` and `is_empty` inspect matching chunk row counts without constructing component
  references.
- Parallel calls reuse prepared jobs when storage is unchanged and may fall back to sequential
  execution for small workloads.
- Component value updates do not invalidate query matching. Structural changes may invalidate
  storage-dependent execution metadata.

## Errors and panics

- Invalid query aliasing, duplicate component types, unsupported derive shapes, and excess query
  width are compile-time errors.
- World-bound query borrows prevent structural mutation during iteration.
- Parallel callbacks must satisfy their `Send + Sync` contracts and must not rely on visitation
  order.

## Minimal example

```rust
use sky_ecs::{With, World};

struct Position(f32);
struct Velocity(f32);
struct Active;

let mut world = World::new();
world.spawn((Position(1.0), Velocity(2.0), Active));

world
    .query_mut::<(&mut Position, &Velocity)>()
    .filter::<With<Active>>()
    .for_each(|position, velocity| position.0 += velocity.0);
```

## See also

- [Entity access](entity-access.md)
- [System `View` and `ParView`](systems.md)
- [Dynamic queries](dynamic.md)
