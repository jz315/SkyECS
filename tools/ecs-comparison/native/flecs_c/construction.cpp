#include <flecs.h>

#include "math.hpp"

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <utility>
#include <vector>

namespace sky_ecs_bench::flecs_c::construction {

constexpr std::size_t ENTITY_COUNT = 10'000;
constexpr std::size_t CONTRACT_ENTITY_COUNT = 128;

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

struct Context {
    ecs_world_t* world = nullptr;
    ecs_entity_t transform_id = 0;
    ecs_entity_t position_id = 0;
    ecs_entity_t rotation_id = 0;
    ecs_entity_t velocity_id = 0;
    std::vector<Transform> transforms;
    std::vector<Position> positions;
    std::vector<Rotation> rotations;
    std::vector<Velocity> velocities;
    std::array<ecs_id_t, 4> bulk_ids{};
    std::array<void*, 4> bulk_data{};

    ~Context() {
        if (world) {
            ecs_fini(world);
        }
    }
};

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

void define_components(Context& context) {
    context.transform_id = define_component<Transform>(context.world, "Transform");
    context.position_id = define_component<Position>(context.world, "Position");
    context.rotation_id = define_component<Rotation>(context.world, "Rotation");
    context.velocity_id = define_component<Velocity>(context.world, "Velocity");
}

void prepare_input(Context& context) {
    context.transforms.assign(ENTITY_COUNT, Transform{Mat4::identity()});
    context.positions.assign(ENTITY_COUNT, Position{1.0f, 0.0f, 0.0f});
    context.rotations.assign(ENTITY_COUNT, Rotation{1.0f, 0.0f, 0.0f});
    context.velocities.assign(ENTITY_COUNT, Velocity{1.0f, 0.0f, 0.0f});
}

void prepare_bulk_columns(Context& context) {
    // Flecs requires sorted IDs. Keep each ID attached to its input column
    // while sorting so the descriptor can never pair a component with the
    // wrong array.
    std::array<std::pair<ecs_id_t, void*>, 4> columns{{
        {context.transform_id, context.transforms.data()},
        {context.position_id, context.positions.data()},
        {context.rotation_id, context.rotations.data()},
        {context.velocity_id, context.velocities.data()},
    }};
    std::sort(columns.begin(), columns.end(), [](const auto& left, const auto& right) {
        return left.first < right.first;
    });
    for (std::size_t index = 0; index < columns.size(); ++index) {
        context.bulk_ids[index] = columns[index].first;
        context.bulk_data[index] = columns[index].second;
    }
}

ecs_table_t* find_target_table(const Context& context) {
    return ecs_table_find(
        context.world,
        context.bulk_ids.data(),
        static_cast<std::int32_t>(context.bulk_ids.size()));
}

Context* create_context() {
    auto* context = new Context();
    context->world = ecs_init();
    define_components(*context);
    prepare_input(*context);
    prepare_bulk_columns(*context);
    return context;
}

const ecs_entity_t* insert_bulk_from_columns(
    Context& context,
    std::size_t entity_count) {
    // Rebuild the per-batch native column mapping inside the measured path.
    // Component registration and source-column generation remain setup work.
    prepare_bulk_columns(context);
    ecs_bulk_desc_t descriptor{};
    descriptor.count = static_cast<std::int32_t>(entity_count);
    std::copy(context.bulk_ids.begin(), context.bulk_ids.end(), descriptor.ids);
    descriptor.data = context.bulk_data.data();
    return ecs_bulk_init(context.world, &descriptor);
}

ecs_entity_t insert_one(
    Context& context,
    ecs_table_t* target_table,
    std::size_t input_index) {
    const ecs_entity_t entity = ecs_new_w_table(context.world, target_table);
    *static_cast<Transform*>(
        ecs_get_mut_id(context.world, entity, context.transform_id)) =
        context.transforms[input_index];
    *static_cast<Position*>(
        ecs_get_mut_id(context.world, entity, context.position_id)) =
        context.positions[input_index];
    *static_cast<Rotation*>(
        ecs_get_mut_id(context.world, entity, context.rotation_id)) =
        context.rotations[input_index];
    *static_cast<Velocity*>(
        ecs_get_mut_id(context.world, entity, context.velocity_id)) =
        context.velocities[input_index];
    return entity;
}

std::uint64_t bulk_from_columns_10k(Context& context) {
    const ecs_entity_t* entities =
        insert_bulk_from_columns(context, ENTITY_COUNT);
    return entities[ENTITY_COUNT - 1];
}

std::uint64_t single_insert_10k(Context& context) {
    ecs_table_t* target_table = find_target_table(context);
    ecs_entity_t last_entity = 0;
    for (std::size_t index = 0; index < ENTITY_COUNT; ++index) {
        last_entity = insert_one(context, target_table, index);
    }
    return last_entity;
}

void assign_distinct_input(Context& context, std::size_t entity_count) {
    for (std::size_t row = 0; row < entity_count; ++row) {
        for (std::size_t column = 0; column < 16; ++column) {
            context.transforms[row].matrix[column] =
                static_cast<float>(1'000 + row * 16 + column);
        }
        context.positions[row] = {
            static_cast<float>(2'000 + row),
            static_cast<float>(2'100 + row),
            static_cast<float>(2'200 + row),
        };
        context.rotations[row] = {
            static_cast<float>(3'000 + row),
            static_cast<float>(3'100 + row),
            static_cast<float>(3'200 + row),
        };
        context.velocities[row] = {
            static_cast<float>(4'000 + row),
            static_cast<float>(4'100 + row),
            static_cast<float>(4'200 + row),
        };
    }
}

bool entity_matches_input(
    const Context& context,
    ecs_entity_t entity,
    std::size_t input_index) {
    const auto* transform = static_cast<const Transform*>(
        ecs_get_id(context.world, entity, context.transform_id));
    const auto* position = static_cast<const Position*>(
        ecs_get_id(context.world, entity, context.position_id));
    const auto* rotation = static_cast<const Rotation*>(
        ecs_get_id(context.world, entity, context.rotation_id));
    const auto* velocity = static_cast<const Velocity*>(
        ecs_get_id(context.world, entity, context.velocity_id));
    if (!transform || !position || !rotation || !velocity) {
        return false;
    }
    return std::equal(
               transform->matrix.begin(),
               transform->matrix.end(),
               context.transforms[input_index].matrix.begin()) &&
        position->x == context.positions[input_index].x &&
        position->y == context.positions[input_index].y &&
        position->z == context.positions[input_index].z &&
        rotation->x == context.rotations[input_index].x &&
        rotation->y == context.rotations[input_index].y &&
        rotation->z == context.rotations[input_index].z &&
        velocity->x == context.velocities[input_index].x &&
        velocity->y == context.velocities[input_index].y &&
        velocity->z == context.velocities[input_index].z;
}

bool has_no_workload_entities(const Context& context) {
    return ecs_count_id(context.world, context.transform_id) == 0 &&
        ecs_count_id(context.world, context.position_id) == 0 &&
        ecs_count_id(context.world, context.rotation_id) == 0 &&
        ecs_count_id(context.world, context.velocity_id) == 0;
}

bool validate_bulk() {
    Context* context = create_context();
    if (!context) {
        return false;
    }
    bool valid = has_no_workload_entities(*context);
    assign_distinct_input(*context, CONTRACT_ENTITY_COUNT);
    const ecs_entity_t* entities =
        insert_bulk_from_columns(*context, CONTRACT_ENTITY_COUNT);
    valid = valid && entities != nullptr;
    for (std::size_t index = 0; valid && index < CONTRACT_ENTITY_COUNT; ++index) {
        valid = entity_matches_input(*context, entities[index], index);
    }
    delete context;
    return valid;
}

bool validate_single() {
    Context* context = create_context();
    if (!context) {
        return false;
    }
    bool valid = has_no_workload_entities(*context);
    assign_distinct_input(*context, CONTRACT_ENTITY_COUNT);
    ecs_table_t* target_table = find_target_table(*context);
    valid = valid && target_table != nullptr;
    for (std::size_t index = 0; valid && index < CONTRACT_ENTITY_COUNT; ++index) {
        const ecs_entity_t entity = insert_one(*context, target_table, index);
        valid = entity != 0 &&
            ecs_get_table(context->world, entity) == target_table &&
            entity_matches_input(*context, entity, index);
    }
    delete context;
    return valid;
}

bool validate() {
    return validate_bulk() && validate_single();
}

} // namespace sky_ecs_bench::flecs_c::construction

using ConstructionContext = sky_ecs_bench::flecs_c::construction::Context;

extern "C" {

void* sky_flecs_c_insert_new() {
    return sky_ecs_bench::flecs_c::construction::create_context();
}

void sky_flecs_c_insert_delete(void* context) {
    delete static_cast<ConstructionContext*>(context);
}

std::uint64_t sky_flecs_c_bulk_from_columns(void* context) {
    return sky_ecs_bench::flecs_c::construction::bulk_from_columns_10k(
        *static_cast<ConstructionContext*>(context));
}

std::uint64_t sky_flecs_c_single_insert(void* context) {
    return sky_ecs_bench::flecs_c::construction::single_insert_10k(
        *static_cast<ConstructionContext*>(context));
}

} // extern "C"
