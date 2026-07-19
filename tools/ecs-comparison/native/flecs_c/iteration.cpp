#include <flecs.h>

#include "math.hpp"

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <iterator>

namespace sky_ecs_bench::flecs_c::iteration {

constexpr std::size_t HEAVY_ENTITY_COUNT = 1'000;
constexpr std::size_t HEAVY_INVERT_COUNT = 100;

struct Transform {
    Mat4 matrix;
};

struct Position {
    float x;
    float y;
    float z;
};

struct Rotation {
    float x;
    float y;
    float z;
};

struct Velocity {
    float x;
    float y;
    float z;
};

static_assert(sizeof(Transform) == 64 && alignof(Transform) == 4);
static_assert(sizeof(Position) == 12 && alignof(Position) == 4);
static_assert(sizeof(Rotation) == 12 && alignof(Rotation) == 4);
static_assert(sizeof(Velocity) == 12 && alignof(Velocity) == 4);

template<typename Component>
ecs_entity_t define_component(ecs_world_t* world, const char* name) {
    ecs_entity_desc_t entity{};
    entity.name = name;
    entity.symbol = name;
    entity.use_low_id = true;

    ecs_component_desc_t component{};
    component.entity = ecs_entity_init(world, &entity);
    component.type.size = sizeof(Component);
    component.type.alignment = alignof(Component);
    return ecs_component_init(world, &component);
}

struct ComponentIds {
    ecs_entity_t transform = 0;
    ecs_entity_t position = 0;
    ecs_entity_t rotation = 0;
    ecs_entity_t velocity = 0;
};

ComponentIds define_components(ecs_world_t* world) {
    return {
        define_component<Transform>(world, "Transform"),
        define_component<Position>(world, "Position"),
        define_component<Rotation>(world, "Rotation"),
        define_component<Velocity>(world, "Velocity"),
    };
}

ecs_table_t* find_entity_table(ecs_world_t* world, const ComponentIds& ids) {
    ecs_id_t table_ids[]{ids.transform, ids.position, ids.rotation, ids.velocity};
    std::sort(std::begin(table_ids), std::end(table_ids));
    return ecs_table_find(world, table_ids, 4);
}

void spawn_entity(
    ecs_world_t* world,
    ecs_table_t* table,
    const ComponentIds& ids,
    const Mat4& transform,
    Position position_value) {
    const ecs_entity_t entity = ecs_new_w_table(world, table);
    const Transform transform_value{transform};
    const Rotation rotation_value{1.0f, 0.0f, 0.0f};
    const Velocity velocity_value{1.0f, 0.0f, 0.0f};
    ecs_set_id(world, entity, ids.transform, sizeof(Transform), &transform_value);
    ecs_set_id(world, entity, ids.position, sizeof(Position), &position_value);
    ecs_set_id(world, entity, ids.rotation, sizeof(Rotation), &rotation_value);
    ecs_set_id(world, entity, ids.velocity, sizeof(Velocity), &velocity_value);
}

ecs_query_t* prepare_move_query(ecs_world_t* world, const ComponentIds& ids) {
    ecs_query_desc_t descriptor{};
    descriptor.cache_kind = EcsQueryCacheAll;
    descriptor.terms[0].id = ids.position;
    descriptor.terms[0].inout = EcsInOut;
    descriptor.terms[1].id = ids.velocity;
    descriptor.terms[1].inout = EcsIn;
    return ecs_query_init(world, &descriptor);
}

ecs_query_t* prepare_heavy_query(ecs_world_t* world, const ComponentIds& ids) {
    ecs_query_desc_t descriptor{};
    descriptor.cache_kind = EcsQueryCacheAll;
    descriptor.terms[0].id = ids.position;
    descriptor.terms[0].inout = EcsInOut;
    descriptor.terms[1].id = ids.transform;
    descriptor.terms[1].inout = EcsIn;
    return ecs_query_init(world, &descriptor);
}

// Keep component work separate from iterator bookkeeping so the optimizer sees
// one plain contiguous-array loop. Flecs stores distinct component columns in
// non-overlapping arrays, which is the aliasing contract expressed below.
void move_entities(
    Position* __restrict positions,
    const Velocity* __restrict velocities,
    std::int32_t count) {
    for (std::int32_t row = 0; row < count; ++row) {
        positions[row].x += velocities[row].x;
        positions[row].y += velocities[row].y;
        positions[row].z += velocities[row].z;
    }
}

struct SimpleContext {
    ecs_world_t* world = nullptr;
    ecs_query_t* query = nullptr;
    ComponentIds components;

    ~SimpleContext() {
        if (query) {
            ecs_query_fini(query);
        }
        if (world) {
            ecs_fini(world);
        }
    }
};

SimpleContext* create_simple_context(std::size_t entity_count) {
    auto* context = new SimpleContext();
    context->world = ecs_init();
    context->components = define_components(context->world);
    ecs_table_t* table = find_entity_table(context->world, context->components);
    if (!table) {
        delete context;
        return nullptr;
    }
    for (std::size_t index = 0; index < entity_count; ++index) {
        spawn_entity(
            context->world,
            table,
            context->components,
            Mat4::identity(),
            {1.0f, 0.0f, 0.0f});
    }
    context->query = prepare_move_query(context->world, context->components);
    if (!context->query) {
        delete context;
        return nullptr;
    }
    return context;
}

std::uint64_t iterate(SimpleContext& context, std::size_t repetitions) {
    for (std::size_t repeat = 0; repeat < repetitions; ++repeat) {
        ecs_iter_t iterator = ecs_query_iter(context.world, context.query);
        while (ecs_query_next(&iterator)) {
            Position* positions = ecs_field(&iterator, Position, 0);
            const Velocity* velocities = ecs_field(&iterator, Velocity, 1);
            move_entities(positions, velocities, iterator.count);
        }
    }
    return static_cast<std::uint64_t>(repetitions);
}

struct HeavyContext {
    ecs_world_t* world = nullptr;
    ecs_query_t* query = nullptr;
    ComponentIds components;

    ~HeavyContext() {
        if (query) {
            ecs_query_fini(query);
        }
        if (world) {
            ecs_fini(world);
        }
    }
};

HeavyContext* create_heavy_context() {
    auto* context = new HeavyContext();
    context->world = ecs_init();
    context->components = define_components(context->world);
    ecs_table_t* table = find_entity_table(context->world, context->components);
    if (!table) {
        delete context;
        return nullptr;
    }
    const Mat4 transform = Mat4::benchmark_rotation_x();
    for (std::size_t index = 0; index < HEAVY_ENTITY_COUNT; ++index) {
        spawn_entity(
            context->world,
            table,
            context->components,
            transform,
            {1.0f, 2.0f, 3.0f});
    }
    context->query = prepare_heavy_query(context->world, context->components);
    if (!context->query) {
        delete context;
        return nullptr;
    }
    return context;
}

std::uint64_t heavy_compute(HeavyContext& context) {
    std::uint64_t checksum = 0;
    ecs_iter_t iterator = ecs_query_iter(context.world, context.query);
    while (ecs_query_next(&iterator)) {
        // Flecs stores different component fields in disjoint table columns.
        Position* __restrict positions = ecs_field(&iterator, Position, 0);
        const Transform* __restrict transforms =
            ecs_field(&iterator, Transform, 1);
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            Mat4 matrix = transforms[row].matrix;
            for (std::size_t index = 0; index < HEAVY_INVERT_COUNT; ++index) {
                matrix = matrix.inverse();
            }
            const Vec3 output = matrix.transform_vector({
                positions[row].x,
                positions[row].y,
                positions[row].z,
            });
            positions[row] = {output.x, output.y, output.z};
            checksum = add_vector_checksum(checksum, output);
        }
    }
    return checksum;
}

bool validate_simple() {
    SimpleContext* context = create_simple_context(128);
    if (!context || iterate(*context, 1) != 1) {
        delete context;
        return false;
    }
    std::size_t count = 0;
    float sum = 0.0f;
    ecs_iter_t iterator = ecs_query_iter(context->world, context->query);
    while (ecs_query_next(&iterator)) {
        const Position* positions = ecs_field(&iterator, Position, 0);
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            ++count;
            sum += positions[row].x;
        }
    }
    delete context;
    return count == 128 && sum == 256.0f;
}

bool validate_heavy() {
    const Mat4 inverse = Mat4::benchmark_rotation_x().inverse();
    const Vec3 output = inverse.transform_vector({1.0f, 2.0f, 3.0f});
    float cosine = 0.0f;
    float sine = 0.0f;
    const std::uint32_t cosine_bits = Mat4::BENCHMARK_COSINE_BITS;
    const std::uint32_t sine_bits = Mat4::BENCHMARK_SINE_BITS;
    std::memcpy(&cosine, &cosine_bits, sizeof(cosine));
    std::memcpy(&sine, &sine_bits, sizeof(sine));
    if (std::fabs(output.x - 1.0f) > 1.0e-5f ||
        std::fabs(output.y - (2.0f * cosine + 3.0f * sine)) > 1.0e-5f ||
        std::fabs(output.z - (-2.0f * sine + 3.0f * cosine)) > 1.0e-5f) {
        return false;
    }
    HeavyContext* context = create_heavy_context();
    const bool valid = context && heavy_compute(*context) != 0 &&
        ecs_query_count(context->query).entities ==
            static_cast<std::int32_t>(HEAVY_ENTITY_COUNT);
    delete context;
    return valid;
}

} // namespace sky_ecs_bench::flecs_c::iteration

using SimpleContext = sky_ecs_bench::flecs_c::iteration::SimpleContext;
using HeavyContext = sky_ecs_bench::flecs_c::iteration::HeavyContext;

extern "C" {

void* sky_flecs_c_simple_new(std::size_t entity_count) {
    return sky_ecs_bench::flecs_c::iteration::create_simple_context(entity_count);
}

void sky_flecs_c_simple_delete(void* context) {
    delete static_cast<SimpleContext*>(context);
}

std::uint64_t sky_flecs_c_simple_run(void* context, std::size_t repetitions) {
    return sky_ecs_bench::flecs_c::iteration::iterate(
        *static_cast<SimpleContext*>(context),
        repetitions);
}

void* sky_flecs_c_heavy_new() {
    return sky_ecs_bench::flecs_c::iteration::create_heavy_context();
}

void sky_flecs_c_heavy_delete(void* context) {
    delete static_cast<HeavyContext*>(context);
}

std::uint64_t sky_flecs_c_heavy_run(void* context) {
    return sky_ecs_bench::flecs_c::iteration::heavy_compute(
        *static_cast<HeavyContext*>(context));
}

} // extern "C"
