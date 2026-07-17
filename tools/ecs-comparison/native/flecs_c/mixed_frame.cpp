#include <flecs.h>

#include "math.hpp"

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <initializer_list>
#include <vector>

namespace sky_ecs_bench::flecs_c::mixed_frame {

constexpr std::size_t MOVER_COUNT = 16'000;
constexpr std::size_t ENEMY_COUNT = 4'000;
constexpr std::size_t ALLY_COUNT = 4'000;
constexpr std::size_t HEAVY_COUNT = 1'000;
constexpr std::size_t RANDOM_COUNT = 512;
constexpr std::size_t CHURN_COUNT = 256;
constexpr std::size_t SPAWN_COUNT = 64;
constexpr std::size_t INVERT_COUNT = 8;
constexpr std::size_t HEALTH_PHASE_REPETITIONS = 8;
constexpr std::size_t SPAWN_PHASE_REPETITIONS = 32;

struct Transform {
    Mat4 matrix;
};

struct Position {
    float x;
    float y;
    float z;
};

struct Velocity {
    float x;
    float y;
    float z;
};

struct Health {
    float value;
};

struct Damage {
    float value;
};

struct Regen {
    float value;
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

ecs_entity_t define_tag(ecs_world_t* world, const char* name) {
    ecs_entity_desc_t descriptor{};
    descriptor.name = name;
    descriptor.symbol = name;
    descriptor.use_low_id = true;
    return ecs_entity_init(world, &descriptor);
}

struct ComponentIds {
    ecs_entity_t transform = 0;
    ecs_entity_t position = 0;
    ecs_entity_t velocity = 0;
    ecs_entity_t health = 0;
    ecs_entity_t damage = 0;
    ecs_entity_t regen = 0;
    ecs_entity_t enemy = 0;
    ecs_entity_t ally = 0;
};

ComponentIds define_components(ecs_world_t* world) {
    return {
        define_component<Transform>(world, "Transform"),
        define_component<Position>(world, "Position"),
        define_component<Velocity>(world, "Velocity"),
        define_component<Health>(world, "Health"),
        define_component<Damage>(world, "Damage"),
        define_component<Regen>(world, "Regen"),
        define_tag(world, "IsEnemy"),
        define_tag(world, "IsAlly"),
    };
}

ecs_table_t* find_table(
    ecs_world_t* world,
    std::initializer_list<ecs_id_t> component_ids) {
    std::vector<ecs_id_t> sorted_ids(component_ids);
    std::sort(sorted_ids.begin(), sorted_ids.end());
    return ecs_table_find(
        world,
        sorted_ids.data(),
        static_cast<std::int32_t>(sorted_ids.size()));
}

void set_position(
    ecs_world_t* world,
    ecs_entity_t entity,
    ecs_entity_t component,
    Position value) {
    ecs_set_id(world, entity, component, sizeof(Position), &value);
}

void set_velocity(
    ecs_world_t* world,
    ecs_entity_t entity,
    ecs_entity_t component,
    Velocity value) {
    ecs_set_id(world, entity, component, sizeof(Velocity), &value);
}

struct Context {
    ecs_world_t* world = nullptr;
    ComponentIds components;
    ecs_table_t* spawn_table = nullptr;
    ecs_query_t* move_query = nullptr;
    ecs_query_t* enemy_query = nullptr;
    ecs_query_t* ally_query = nullptr;
    ecs_query_t* heavy_query = nullptr;
    std::vector<ecs_ref_t> random_positions;
    std::vector<ecs_entity_t> churn_entities;
    std::vector<ecs_entity_t> spawned_entities;

    ~Context() {
        if (move_query) {
            ecs_query_fini(move_query);
        }
        if (enemy_query) {
            ecs_query_fini(enemy_query);
        }
        if (ally_query) {
            ecs_query_fini(ally_query);
        }
        if (heavy_query) {
            ecs_query_fini(heavy_query);
        }
        if (world) {
            ecs_fini(world);
        }
    }
};

void spawn_movers(Context& context, std::vector<ecs_entity_t>& all_entities) {
    ecs_table_t* table = find_table(
        context.world,
        {context.components.position, context.components.velocity});
    const Position position{0.0f, 1.0f, 0.0f};
    const Velocity velocity{1.0f, 0.5f, 0.25f};

    for (std::size_t index = 0; index < MOVER_COUNT; ++index) {
        const ecs_entity_t entity = ecs_new_w_table(context.world, table);
        set_position(
            context.world,
            entity,
            context.components.position,
            position);
        set_velocity(
            context.world,
            entity,
            context.components.velocity,
            velocity);
        all_entities.push_back(entity);
        if (context.churn_entities.size() < CHURN_COUNT) {
            context.churn_entities.push_back(entity);
        }
    }
}

void spawn_enemies(Context& context, std::vector<ecs_entity_t>& all_entities) {
    ecs_table_t* table = find_table(
        context.world,
        {
            context.components.position,
            context.components.velocity,
            context.components.health,
            context.components.damage,
            context.components.enemy,
        });
    const Position position{2.0f, 0.0f, 0.0f};
    const Velocity velocity{0.25f, 1.0f, 0.0f};
    const Health health{100.0f};
    const Damage damage{0.75f};

    for (std::size_t index = 0; index < ENEMY_COUNT; ++index) {
        const ecs_entity_t entity = ecs_new_w_table(context.world, table);
        set_position(
            context.world,
            entity,
            context.components.position,
            position);
        set_velocity(
            context.world,
            entity,
            context.components.velocity,
            velocity);
        ecs_set_id(
            context.world,
            entity,
            context.components.health,
            sizeof(Health),
            &health);
        ecs_set_id(
            context.world,
            entity,
            context.components.damage,
            sizeof(Damage),
            &damage);
        all_entities.push_back(entity);
    }
}

void spawn_allies(Context& context, std::vector<ecs_entity_t>& all_entities) {
    ecs_table_t* table = find_table(
        context.world,
        {
            context.components.position,
            context.components.velocity,
            context.components.health,
            context.components.regen,
            context.components.ally,
        });
    const Position position{-2.0f, 0.0f, 0.0f};
    const Velocity velocity{0.0f, 0.75f, 0.25f};
    const Health health{60.0f};
    const Regen regen{0.35f};

    for (std::size_t index = 0; index < ALLY_COUNT; ++index) {
        const ecs_entity_t entity = ecs_new_w_table(context.world, table);
        set_position(
            context.world,
            entity,
            context.components.position,
            position);
        set_velocity(
            context.world,
            entity,
            context.components.velocity,
            velocity);
        ecs_set_id(
            context.world,
            entity,
            context.components.health,
            sizeof(Health),
            &health);
        ecs_set_id(
            context.world,
            entity,
            context.components.regen,
            sizeof(Regen),
            &regen);
        all_entities.push_back(entity);
    }
}

void spawn_heavy_entities(
    Context& context,
    std::vector<ecs_entity_t>& all_entities) {
    ecs_table_t* table = find_table(
        context.world,
        {
            context.components.transform,
            context.components.position,
            context.components.velocity,
        });
    const Transform transform{Mat4::rotation_x(1.2f)};
    const Position position{1.0f, 0.0f, 0.0f};
    const Velocity velocity{0.5f, 0.0f, 0.5f};

    for (std::size_t index = 0; index < HEAVY_COUNT; ++index) {
        const ecs_entity_t entity = ecs_new_w_table(context.world, table);
        ecs_set_id(
            context.world,
            entity,
            context.components.transform,
            sizeof(Transform),
            &transform);
        set_position(
            context.world,
            entity,
            context.components.position,
            position);
        set_velocity(
            context.world,
            entity,
            context.components.velocity,
            velocity);
        all_entities.push_back(entity);
    }
}

void deterministic_shuffle(
    std::vector<ecs_entity_t>& entities,
    std::uint64_t state) {
    for (std::size_t length = entities.size(); length > 1; --length) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        const std::size_t other =
            static_cast<std::size_t>(state) % length;
        std::swap(entities[length - 1], entities[other]);
    }
}

void prepare_random_access(
    Context& context,
    const std::vector<ecs_entity_t>& all_entities) {
    std::vector<ecs_entity_t> sampled;
    sampled.reserve(RANDOM_COUNT);
    for (std::size_t index = 0; index < RANDOM_COUNT; ++index) {
        sampled.push_back(
            all_entities[index * all_entities.size() / RANDOM_COUNT]);
    }
    deterministic_shuffle(sampled, 0xDEADBEEFCAFEBABEULL);

    context.random_positions.reserve(sampled.size());
    for (ecs_entity_t entity : sampled) {
        context.random_positions.push_back(ecs_ref_init_id(
            context.world,
            entity,
            context.components.position));
    }
}

ecs_query_t* prepare_query(
    ecs_world_t* world,
    ecs_entity_t first,
    ecs_entity_t second) {
    ecs_query_desc_t descriptor{};
    descriptor.cache_kind = EcsQueryCacheAll;
    descriptor.terms[0].id = first;
    descriptor.terms[0].inout = EcsInOut;
    descriptor.terms[1].id = second;
    descriptor.terms[1].inout = EcsIn;
    return ecs_query_init(world, &descriptor);
}

void prepare_queries(Context& context) {
    context.move_query = prepare_query(
        context.world,
        context.components.position,
        context.components.velocity);
    context.enemy_query = prepare_query(
        context.world,
        context.components.health,
        context.components.damage);
    context.ally_query = prepare_query(
        context.world,
        context.components.health,
        context.components.regen);
    context.heavy_query = prepare_query(
        context.world,
        context.components.position,
        context.components.transform);
}

Context* create_context() {
    auto* context = new Context();
    context->world = ecs_init();
    context->components = define_components(context->world);
    context->spawn_table = find_table(
        context->world,
        {context->components.position, context->components.velocity});
    context->churn_entities.reserve(CHURN_COUNT);
    context->spawned_entities.reserve(SPAWN_COUNT);

    std::vector<ecs_entity_t> all_entities;
    all_entities.reserve(MOVER_COUNT + ENEMY_COUNT + ALLY_COUNT + HEAVY_COUNT);
    spawn_movers(*context, all_entities);
    spawn_enemies(*context, all_entities);
    spawn_allies(*context, all_entities);
    spawn_heavy_entities(*context, all_entities);
    prepare_random_access(*context, all_entities);
    prepare_queries(*context);

    if (!context->spawn_table || !context->move_query ||
        !context->enemy_query || !context->ally_query ||
        !context->heavy_query) {
        delete context;
        return nullptr;
    }
    return context;
}

std::uint64_t movement(Context& context) {
    ecs_iter_t iterator = ecs_query_iter(context.world, context.move_query);
    while (ecs_query_next(&iterator)) {
        Position* positions = ecs_field(&iterator, Position, 0);
        const Velocity* velocities = ecs_field(&iterator, Velocity, 1);
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            positions[row].x += velocities[row].x;
            positions[row].y += velocities[row].y;
            positions[row].z += velocities[row].z;
        }
    }
    return 1;
}

void update_enemies(Context& context) {
    ecs_iter_t iterator = ecs_query_iter(context.world, context.enemy_query);
    while (ecs_query_next(&iterator)) {
        Health* health = ecs_field(&iterator, Health, 0);
        const Damage* damage = ecs_field(&iterator, Damage, 1);
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            health[row].value -= damage[row].value;
        }
    }
}

void update_allies(Context& context) {
    ecs_iter_t iterator = ecs_query_iter(context.world, context.ally_query);
    while (ecs_query_next(&iterator)) {
        Health* health = ecs_field(&iterator, Health, 0);
        const Regen* regen = ecs_field(&iterator, Regen, 1);
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            health[row].value += regen[row].value;
        }
    }
}

std::uint64_t health_repeated(Context& context, std::size_t repetitions) {
    for (std::size_t repeat = 0; repeat < repetitions; ++repeat) {
        update_enemies(context);
        update_allies(context);
    }
    return 1;
}

std::uint64_t heavy(Context& context) {
    ecs_iter_t iterator = ecs_query_iter(context.world, context.heavy_query);
    while (ecs_query_next(&iterator)) {
        Position* positions = ecs_field(&iterator, Position, 0);
        const Transform* transforms = ecs_field(&iterator, Transform, 1);
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            Mat4 matrix = transforms[row].matrix;
            for (std::size_t index = 0; index < INVERT_COUNT; ++index) {
                matrix = matrix.inverse();
            }
            const Vec3 transformed = matrix.transform_vector({
                positions[row].x,
                positions[row].y,
                positions[row].z,
            });
            positions[row] = {
                transformed.x,
                transformed.y,
                transformed.z,
            };
        }
    }
    return 1;
}

std::uint64_t random_access(Context& context) {
    std::uint64_t checksum = 0;
    for (ecs_ref_t& reference : context.random_positions) {
        const auto* position = static_cast<const Position*>(ecs_ref_get_id(
            context.world,
            &reference,
            context.components.position));
        if (!position) {
            return 0;
        }
        std::uint32_t x_bits = 0;
        std::memcpy(&x_bits, &position->x, sizeof(x_bits));
        checksum += x_bits;
    }
    return checksum;
}

std::uint64_t churn(Context& context) {
    const Health health{100.0f};
    for (ecs_entity_t entity : context.churn_entities) {
        ecs_set_id(
            context.world,
            entity,
            context.components.health,
            sizeof(Health),
            &health);
    }
    for (ecs_entity_t entity : context.churn_entities) {
        ecs_remove_id(context.world, entity, context.components.health);
    }
    return context.churn_entities.size();
}

ecs_entity_t spawn_light(Context& context) {
    const Position position{1.0f, 0.0f, 0.0f};
    const Velocity velocity{1.0f, 0.0f, 0.0f};
    const ecs_entity_t entity =
        ecs_new_w_table(context.world, context.spawn_table);
    *static_cast<Position*>(ecs_get_mut_id(
        context.world,
        entity,
        context.components.position)) = position;
    *static_cast<Velocity*>(ecs_get_mut_id(
        context.world,
        entity,
        context.components.velocity)) = velocity;
    return entity;
}

std::uint64_t spawn_repeated(Context& context, std::size_t repetitions) {
    std::uint64_t last_entity = 0;
    for (std::size_t repeat = 0; repeat < repetitions; ++repeat) {
        context.spawned_entities.clear();
        for (std::size_t index = 0; index < SPAWN_COUNT; ++index) {
            const ecs_entity_t entity = spawn_light(context);
            context.spawned_entities.push_back(entity);
        }
        last_entity = context.spawned_entities.back();
        for (ecs_entity_t entity : context.spawned_entities) {
            ecs_delete(context.world, entity);
        }
    }
    return last_entity;
}

std::uint64_t frame(Context& context) {
    movement(context);
    health_repeated(context, 1);
    heavy(context);
    const std::uint64_t checksum = random_access(context);
    churn(context);
    spawn_repeated(context, 1);
    return checksum;
}

std::uint64_t health(Context& context) {
    return health_repeated(context, HEALTH_PHASE_REPETITIONS);
}

std::uint64_t spawn(Context& context) {
    return spawn_repeated(context, SPAWN_PHASE_REPETITIONS);
}

ecs_query_t* single_component_query(
    ecs_world_t* world,
    ecs_entity_t component) {
    ecs_query_desc_t descriptor{};
    descriptor.cache_kind = EcsQueryCacheNone;
    descriptor.terms[0].id = component;
    descriptor.terms[0].inout = EcsIn;
    return ecs_query_init(world, &descriptor);
}

bool validate() {
    Context* context = create_context();
    if (!context) {
        return false;
    }

    ecs_query_t* positions = single_component_query(
        context->world,
        context->components.position);
    const std::int32_t before = ecs_query_count(positions).entities;

    if (frame(*context) == 0 ||
        ecs_query_count(positions).entities != before ||
        context->spawned_entities.empty() ||
        ecs_has_id(
            context->world,
            context->churn_entities.front(),
            context->components.health)) {
        ecs_query_fini(positions);
        delete context;
        return false;
    }
    for (ecs_entity_t entity : context->spawned_entities) {
        if (ecs_is_alive(context->world, entity)) {
            ecs_query_fini(positions);
            delete context;
            return false;
        }
    }
    const ecs_entity_t initialized = spawn_light(*context);
    if (!initialized) {
        ecs_query_fini(positions);
        delete context;
        return false;
    }
    const auto* initialized_position = static_cast<const Position*>(ecs_get_id(
        context->world,
        initialized,
        context->components.position));
    const auto* initialized_velocity = static_cast<const Velocity*>(ecs_get_id(
        context->world,
        initialized,
        context->components.velocity));
    const bool initialized_values = initialized_position && initialized_velocity &&
        initialized_position->x == 1.0f && initialized_velocity->x == 1.0f;
    ecs_delete(context->world, initialized);
    if (!initialized_values) {
        ecs_query_fini(positions);
        delete context;
        return false;
    }

    std::size_t position_count = 0;
    float position_sum = 0.0f;
    ecs_iter_t position_iterator = ecs_query_iter(context->world, positions);
    while (ecs_query_next(&position_iterator)) {
        const Position* values = ecs_field(&position_iterator, Position, 0);
        for (std::int32_t row = 0; row < position_iterator.count; ++row) {
            ++position_count;
            position_sum += values[row].x;
        }
    }
    ecs_query_fini(positions);

    ecs_query_t* health_values = single_component_query(
        context->world,
        context->components.health);
    std::size_t health_count = 0;
    float health_sum = 0.0f;
    ecs_iter_t health_iterator = ecs_query_iter(context->world, health_values);
    while (ecs_query_next(&health_iterator)) {
        const Health* values = ecs_field(&health_iterator, Health, 0);
        for (std::int32_t row = 0; row < health_iterator.count; ++row) {
            ++health_count;
            health_sum += values[row].value;
        }
    }
    ecs_query_fini(health_values);
    delete context;

    return position_count == MOVER_COUNT + ENEMY_COUNT + ALLY_COUNT + HEAVY_COUNT &&
        std::fabs(position_sum - 18'500.0f) <= 0.2f &&
        health_count == ENEMY_COUNT + ALLY_COUNT &&
        std::fabs(health_sum - 638'400.0f) <= 64.0f;
}

} // namespace sky_ecs_bench::flecs_c::mixed_frame

using MixedFrameContext = sky_ecs_bench::flecs_c::mixed_frame::Context;

extern "C" {

void* sky_flecs_c_mixed_new() {
    return sky_ecs_bench::flecs_c::mixed_frame::create_context();
}

void sky_flecs_c_mixed_delete(void* context) {
    delete static_cast<MixedFrameContext*>(context);
}

std::uint64_t sky_flecs_c_mixed_frame(void* context) {
    return sky_ecs_bench::flecs_c::mixed_frame::frame(
        *static_cast<MixedFrameContext*>(context));
}

std::uint64_t sky_flecs_c_mixed_movement(void* context) {
    return sky_ecs_bench::flecs_c::mixed_frame::movement(
        *static_cast<MixedFrameContext*>(context));
}

std::uint64_t sky_flecs_c_mixed_health(void* context) {
    return sky_ecs_bench::flecs_c::mixed_frame::health(
        *static_cast<MixedFrameContext*>(context));
}

std::uint64_t sky_flecs_c_mixed_heavy(void* context) {
    return sky_ecs_bench::flecs_c::mixed_frame::heavy(
        *static_cast<MixedFrameContext*>(context));
}

std::uint64_t sky_flecs_c_mixed_random(void* context) {
    return sky_ecs_bench::flecs_c::mixed_frame::random_access(
        *static_cast<MixedFrameContext*>(context));
}

std::uint64_t sky_flecs_c_mixed_churn(void* context) {
    return sky_ecs_bench::flecs_c::mixed_frame::churn(
        *static_cast<MixedFrameContext*>(context));
}

std::uint64_t sky_flecs_c_mixed_spawn(void* context) {
    return sky_ecs_bench::flecs_c::mixed_frame::spawn(
        *static_cast<MixedFrameContext*>(context));
}

} // extern "C"
