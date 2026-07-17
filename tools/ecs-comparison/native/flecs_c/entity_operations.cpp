#include <flecs.h>

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <initializer_list>
#include <vector>

namespace sky_ecs_bench::flecs_c::entity_operations {

constexpr std::size_t ENTITY_COUNT = 1'000;
constexpr std::uint64_t ENTITY_DELETION_SEED = 0xDEADBEEFCAFEBABEULL;
constexpr std::uint64_t COMPONENT_ADD_SEED = 0xA0761D6478BD642FULL;
constexpr std::uint64_t COMPONENT_REMOVE_SEED = 0xE7037ED1A0B428DBULL;

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

std::vector<std::size_t> make_entity_order(std::uint64_t state) {
    std::vector<std::size_t> order(ENTITY_COUNT);
    for (std::size_t index = 0; index < order.size(); ++index) {
        order[index] = index;
    }

    for (std::size_t remaining = order.size(); remaining > 1; --remaining) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        const std::size_t other = state % remaining;
        std::swap(order[remaining - 1], order[other]);
    }
    return order;
}

struct SpawnContext {
    ecs_world_t* world = nullptr;
    ecs_entity_t position = 0;
    ecs_entity_t velocity = 0;
    ecs_table_t* target_table = nullptr;
    std::vector<ecs_entity_t> entities;
    std::vector<std::size_t> deletion_order;

    ~SpawnContext() {
        if (world) {
            ecs_fini(world);
        }
    }
};

SpawnContext* create_spawn_context() {
    auto* context = new SpawnContext();
    context->world = ecs_init();

    context->position = define_component<Position>(context->world, "Position");
    context->velocity = define_component<Velocity>(context->world, "Velocity");
    context->target_table = find_table(
        context->world,
        {context->position, context->velocity});
    context->entities.reserve(ENTITY_COUNT);
    context->deletion_order = make_entity_order(ENTITY_DELETION_SEED);

    if (!context->target_table) {
        delete context;
        return nullptr;
    }
    return context;
}

ecs_entity_t spawn_light(SpawnContext& context) {
    const Position position{1.0f, 0.0f, 0.0f};
    const Velocity velocity{1.0f, 0.0f, 0.0f};
    const ecs_entity_t entity =
        ecs_new_w_table(context.world, context.target_table);
    *static_cast<Position*>(
        ecs_get_mut_id(context.world, entity, context.position)) = position;
    *static_cast<Velocity*>(
        ecs_get_mut_id(context.world, entity, context.velocity)) = velocity;
    return entity;
}

std::uint64_t spawn_despawn_1k(SpawnContext& context) {
    context.entities.clear();
    for (std::size_t index = 0; index < ENTITY_COUNT; ++index) {
        const ecs_entity_t entity = spawn_light(context);
        context.entities.push_back(entity);
    }

    const std::uint64_t last_entity = context.entities.back();
    for (std::size_t index : context.deletion_order) {
        ecs_delete(context.world, context.entities[index]);
    }
    return last_entity;
}

struct AddRemoveContext {
    ecs_world_t* world = nullptr;
    ecs_entity_t health = 0;
    std::vector<ecs_entity_t> entities;
    std::vector<std::size_t> add_order;
    std::vector<std::size_t> remove_order;

    ~AddRemoveContext() {
        if (world) {
            ecs_fini(world);
        }
    }
};

AddRemoveContext* create_add_remove_context() {
    auto* context = new AddRemoveContext();
    context->world = ecs_init();

    const ecs_entity_t position =
        define_component<Position>(context->world, "Position");
    const ecs_entity_t velocity =
        define_component<Velocity>(context->world, "Velocity");
    context->health = define_component<Health>(context->world, "Health");

    ecs_table_t* base_table = find_table(
        context->world,
        {position, velocity});
    if (!base_table) {
        delete context;
        return nullptr;
    }

    context->entities.reserve(ENTITY_COUNT);
    context->add_order = make_entity_order(COMPONENT_ADD_SEED);
    context->remove_order = make_entity_order(COMPONENT_REMOVE_SEED);
    const Position position_value{1.0f, 0.0f, 0.0f};
    const Velocity velocity_value{1.0f, 0.0f, 0.0f};
    for (std::size_t index = 0; index < ENTITY_COUNT; ++index) {
        const ecs_entity_t entity = ecs_new_w_table(context->world, base_table);
        ecs_set_id(
            context->world,
            entity,
            position,
            sizeof(Position),
            &position_value);
        ecs_set_id(
            context->world,
            entity,
            velocity,
            sizeof(Velocity),
            &velocity_value);
        context->entities.push_back(entity);
    }
    return context;
}

void add_health(AddRemoveContext& context) {
    const Health health{100.0f};
    for (std::size_t index : context.add_order) {
        const ecs_entity_t entity = context.entities[index];
        ecs_set_id(
            context.world,
            entity,
            context.health,
            sizeof(Health),
            &health);
    }
}

void remove_health(AddRemoveContext& context) {
    for (std::size_t index : context.remove_order) {
        const ecs_entity_t entity = context.entities[index];
        ecs_remove_id(context.world, entity, context.health);
    }
}

std::uint64_t add_remove_1k(AddRemoveContext& context) {
    add_health(context);
    remove_health(context);
    return context.entities.size();
}

bool validate() {
    SpawnContext* spawn = create_spawn_context();
    if (!spawn || spawn_despawn_1k(*spawn) == 0 ||
        spawn->entities.size() != ENTITY_COUNT) {
        delete spawn;
        return false;
    }
    for (ecs_entity_t entity : spawn->entities) {
        if (ecs_is_alive(spawn->world, entity)) {
            delete spawn;
            return false;
        }
    }
    const ecs_entity_t initialized = spawn_light(*spawn);
    if (!initialized) {
        delete spawn;
        return false;
    }
    const auto* position = static_cast<const Position*>(
        ecs_get_id(spawn->world, initialized, spawn->position));
    const auto* velocity = static_cast<const Velocity*>(
        ecs_get_id(spawn->world, initialized, spawn->velocity));
    if (!position || !velocity || position->x != 1.0f ||
        velocity->x != 1.0f) {
        delete spawn;
        return false;
    }
    ecs_delete(spawn->world, initialized);
    delete spawn;

    AddRemoveContext* add_remove = create_add_remove_context();
    if (!add_remove) {
        return false;
    }
    add_health(*add_remove);
    for (ecs_entity_t entity : add_remove->entities) {
        const auto* health = static_cast<const Health*>(
            ecs_get_id(add_remove->world, entity, add_remove->health));
        if (!health || health->value != 100.0f) {
            delete add_remove;
            return false;
        }
    }
    remove_health(*add_remove);
    for (ecs_entity_t entity : add_remove->entities) {
        if (ecs_has_id(add_remove->world, entity, add_remove->health)) {
            delete add_remove;
            return false;
        }
    }
    delete add_remove;
    return true;
}

} // namespace sky_ecs_bench::flecs_c::entity_operations

using SpawnContext =
    sky_ecs_bench::flecs_c::entity_operations::SpawnContext;
using AddRemoveContext =
    sky_ecs_bench::flecs_c::entity_operations::AddRemoveContext;

extern "C" {

void* sky_flecs_c_entity_ops_new() {
    return sky_ecs_bench::flecs_c::entity_operations::create_spawn_context();
}

void sky_flecs_c_entity_ops_delete(void* context) {
    delete static_cast<SpawnContext*>(context);
}

std::uint64_t sky_flecs_c_spawn_despawn(void* context) {
    return sky_ecs_bench::flecs_c::entity_operations::spawn_despawn_1k(
        *static_cast<SpawnContext*>(context));
}

void* sky_flecs_c_add_remove_new() {
    return sky_ecs_bench::flecs_c::entity_operations::create_add_remove_context();
}

void sky_flecs_c_add_remove_delete(void* context) {
    delete static_cast<AddRemoveContext*>(context);
}

std::uint64_t sky_flecs_c_add_remove(void* context) {
    return sky_ecs_bench::flecs_c::entity_operations::add_remove_1k(
        *static_cast<AddRemoveContext*>(context));
}

} // extern "C"
