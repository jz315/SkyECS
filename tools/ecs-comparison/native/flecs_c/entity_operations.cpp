#include <flecs.h>

#include <algorithm>
#include <array>
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
    std::array<ecs_id_t, 2> bulk_ids{};
    bool position_first = true;
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
    context->position_first = context->position < context->velocity;
    context->bulk_ids = context->position_first
        ? std::array<ecs_id_t, 2>{context->position, context->velocity}
        : std::array<ecs_id_t, 2>{context->velocity, context->position};
    context->entities.reserve(ENTITY_COUNT);
    context->deletion_order = make_entity_order(ENTITY_DELETION_SEED);

    if (!context->target_table) {
        delete context;
        return nullptr;
    }
    return context;
}

ecs_entity_t spawn_light_get_mut(SpawnContext& context) {
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

ecs_entity_t spawn_light_set_id(SpawnContext& context) {
    const Position position{1.0f, 0.0f, 0.0f};
    const Velocity velocity{1.0f, 0.0f, 0.0f};
    const ecs_entity_t entity =
        ecs_new_w_table(context.world, context.target_table);
    ecs_set_id(
        context.world,
        entity,
        context.position,
        sizeof(Position),
        &position);
    ecs_set_id(
        context.world,
        entity,
        context.velocity,
        sizeof(Velocity),
        &velocity);
    return entity;
}

ecs_entity_t spawn_light_bulk_one(SpawnContext& context) {
    Position position{1.0f, 0.0f, 0.0f};
    Velocity velocity{1.0f, 0.0f, 0.0f};
    std::array<void*, 2> data = context.position_first
        ? std::array<void*, 2>{&position, &velocity}
        : std::array<void*, 2>{&velocity, &position};
    ecs_bulk_desc_t descriptor{};
    descriptor.count = 1;
    std::copy(
        context.bulk_ids.begin(),
        context.bulk_ids.end(),
        descriptor.ids);
    descriptor.data = data.data();
    return ecs_bulk_init(context.world, &descriptor)[0];
}

template<ecs_entity_t (*Spawn)(SpawnContext&)>
std::uint64_t spawn_despawn_1k_with(SpawnContext& context) {
    context.entities.clear();
    for (std::size_t index = 0; index < ENTITY_COUNT; ++index) {
        const ecs_entity_t entity = Spawn(context);
        context.entities.push_back(entity);
    }

    const std::uint64_t last_entity = context.entities.back();
    for (std::size_t index : context.deletion_order) {
        ecs_delete(context.world, context.entities[index]);
    }
    return last_entity;
}

std::uint64_t spawn_despawn_1k(SpawnContext& context) {
    return spawn_despawn_1k_with<spawn_light_get_mut>(context);
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

void add_health_set_id(AddRemoveContext& context) {
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

void add_health_emplace(AddRemoveContext& context) {
    const Health health{100.0f};
    for (std::size_t index : context.add_order) {
        const ecs_entity_t entity = context.entities[index];
        auto* destination = static_cast<Health*>(
            ecs_emplace_id(
                context.world,
                entity,
                context.health,
                sizeof(Health),
                nullptr));
        *destination = health;
    }
}

void add_health_add_get_mut(AddRemoveContext& context) {
    const Health health{100.0f};
    for (std::size_t index : context.add_order) {
        const ecs_entity_t entity = context.entities[index];
        ecs_add_id(context.world, entity, context.health);
        *static_cast<Health*>(
            ecs_get_mut_id(context.world, entity, context.health)) = health;
    }
}

void remove_health(AddRemoveContext& context) {
    for (std::size_t index : context.remove_order) {
        const ecs_entity_t entity = context.entities[index];
        ecs_remove_id(context.world, entity, context.health);
    }
}

template<void (*Add)(AddRemoveContext&)>
std::uint64_t add_remove_1k_with(AddRemoveContext& context) {
    Add(context);
    remove_health(context);
    return context.entities.size();
}

std::uint64_t add_remove_1k(AddRemoveContext& context) {
    return add_remove_1k_with<add_health_emplace>(context);
}

template<ecs_entity_t (*Spawn)(SpawnContext&)>
bool validate_spawn() {
    SpawnContext* spawn = create_spawn_context();
    if (!spawn || spawn_despawn_1k_with<Spawn>(*spawn) == 0 ||
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
    const ecs_entity_t initialized = Spawn(*spawn);
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
    return true;
}

template<void (*Add)(AddRemoveContext&)>
bool validate_add_remove() {
    AddRemoveContext* add_remove = create_add_remove_context();
    if (!add_remove) {
        return false;
    }
    Add(*add_remove);
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

bool validate() {
    return validate_spawn<spawn_light_get_mut>() &&
        validate_spawn<spawn_light_set_id>() &&
        validate_spawn<spawn_light_bulk_one>() &&
        validate_add_remove<add_health_set_id>() &&
        validate_add_remove<add_health_emplace>() &&
        validate_add_remove<add_health_add_get_mut>();
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

std::uint64_t sky_flecs_c_spawn_despawn_get_mut(void* context) {
    return sky_ecs_bench::flecs_c::entity_operations::
        spawn_despawn_1k_with<
            sky_ecs_bench::flecs_c::entity_operations::spawn_light_get_mut>(
                *static_cast<SpawnContext*>(context));
}

std::uint64_t sky_flecs_c_spawn_despawn_set_id(void* context) {
    return sky_ecs_bench::flecs_c::entity_operations::
        spawn_despawn_1k_with<
            sky_ecs_bench::flecs_c::entity_operations::spawn_light_set_id>(
                *static_cast<SpawnContext*>(context));
}

std::uint64_t sky_flecs_c_spawn_despawn_bulk_one(void* context) {
    return sky_ecs_bench::flecs_c::entity_operations::
        spawn_despawn_1k_with<
            sky_ecs_bench::flecs_c::entity_operations::spawn_light_bulk_one>(
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

std::uint64_t sky_flecs_c_add_remove_set_id(void* context) {
    return sky_ecs_bench::flecs_c::entity_operations::
        add_remove_1k_with<
            sky_ecs_bench::flecs_c::entity_operations::add_health_set_id>(
                *static_cast<AddRemoveContext*>(context));
}

std::uint64_t sky_flecs_c_add_remove_emplace(void* context) {
    return sky_ecs_bench::flecs_c::entity_operations::
        add_remove_1k_with<
            sky_ecs_bench::flecs_c::entity_operations::add_health_emplace>(
                *static_cast<AddRemoveContext*>(context));
}

std::uint64_t sky_flecs_c_add_remove_add_get_mut(void* context) {
    return sky_ecs_bench::flecs_c::entity_operations::
        add_remove_1k_with<
            sky_ecs_bench::flecs_c::entity_operations::add_health_add_get_mut>(
                *static_cast<AddRemoveContext*>(context));
}

} // extern "C"
