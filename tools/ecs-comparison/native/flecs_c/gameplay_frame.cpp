#include <flecs.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <initializer_list>
#include <utility>
#include <vector>

namespace sky_ecs_bench::flecs_c::gameplay_frame {

constexpr std::size_t ENTITY_COUNT = 65'536;
constexpr std::size_t FRAME_COUNT = 256;
constexpr std::size_t MOVER_START = 0;
constexpr std::size_t MOVER_COUNT = 20'480;
constexpr std::size_t COMBAT_START = MOVER_START + MOVER_COUNT;
constexpr std::size_t COMBAT_COUNT = 16'384;
constexpr std::size_t ENEMY_COUNT = COMBAT_COUNT / 2;
constexpr std::size_t AI_START = COMBAT_START + COMBAT_COUNT;
constexpr std::size_t AI_COUNT = 8'192;
constexpr std::size_t PROJECTILE_START = AI_START + AI_COUNT;
constexpr std::size_t PROJECTILE_COUNT = 8'192;
constexpr std::size_t STATIC_START = PROJECTILE_START + PROJECTILE_COUNT;
constexpr std::size_t STATIC_COUNT = 8'192;
constexpr std::size_t EFFECT_START = STATIC_START + STATIC_COUNT;
constexpr std::size_t EFFECT_COUNT = 4'096;
constexpr std::size_t AI_LOOKUPS = 2'048;
constexpr std::size_t STATUS_CHANGES = 128;
constexpr std::size_t PROJECTILE_RECYCLES = 128;
constexpr std::size_t STATUS_DURATION = 8;
constexpr std::size_t PROJECTILE_LIFETIME = 64;
constexpr std::size_t STATUS_COHORTS = STATUS_DURATION * 2;
constexpr std::size_t AI_COHORTS = AI_COUNT / AI_LOOKUPS;

static_assert(EFFECT_START + EFFECT_COUNT == ENTITY_COUNT);

struct Position { float x; float y; float z; };
struct Velocity { float x; float y; float z; };
struct Health { float value; };
struct Damage { float value; };
struct Regen { float value; };
struct Lifetime { std::uint32_t value; };
struct TargetSlot { std::uint32_t value; };
struct Cooldown { std::uint32_t value; };
struct OwnerSlot { std::uint32_t value; };

static_assert(sizeof(Position) == 12 && alignof(Position) == 4);
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

ecs_entity_t define_tag(ecs_world_t* world, const char* name) {
    ecs_entity_desc_t descriptor{};
    descriptor.name = name;
    descriptor.symbol = name;
    descriptor.use_low_id = true;
    return ecs_entity_init(world, &descriptor);
}

struct ComponentIds {
    ecs_entity_t position = 0;
    ecs_entity_t velocity = 0;
    ecs_entity_t health = 0;
    ecs_entity_t damage = 0;
    ecs_entity_t regen = 0;
    ecs_entity_t lifetime = 0;
    ecs_entity_t target_slot = 0;
    ecs_entity_t cooldown = 0;
    ecs_entity_t owner_slot = 0;
    ecs_entity_t enemy = 0;
    ecs_entity_t ally = 0;
    ecs_entity_t stunned = 0;
    ecs_entity_t tag_a = 0;
    ecs_entity_t tag_b = 0;
};

ComponentIds define_components(ecs_world_t* world) {
    return {
        define_component<Position>(world, "GameplayPosition"),
        define_component<Velocity>(world, "GameplayVelocity"),
        define_component<Health>(world, "GameplayHealth"),
        define_component<Damage>(world, "GameplayDamage"),
        define_component<Regen>(world, "GameplayRegen"),
        define_component<Lifetime>(world, "GameplayLifetime"),
        define_component<TargetSlot>(world, "GameplayTargetSlot"),
        define_component<Cooldown>(world, "GameplayCooldown"),
        define_component<OwnerSlot>(world, "GameplayOwnerSlot"),
        define_tag(world, "GameplayIsEnemy"),
        define_tag(world, "GameplayIsAlly"),
        define_tag(world, "GameplayStunned"),
        define_tag(world, "GameplayTagA"),
        define_tag(world, "GameplayTagB"),
    };
}

enum class Kind : std::uint8_t {
    Mover,
    Enemy,
    Ally,
    Ai,
    Projectile,
    Static,
    Effect,
};

Kind kind_for(std::size_t slot) {
    if (slot < COMBAT_START) return Kind::Mover;
    if (slot < COMBAT_START + ENEMY_COUNT) return Kind::Enemy;
    if (slot < AI_START) return Kind::Ally;
    if (slot < PROJECTILE_START) return Kind::Ai;
    if (slot < STATIC_START) return Kind::Projectile;
    if (slot < EFFECT_START) return Kind::Static;
    return Kind::Effect;
}

Position position_for(std::size_t slot, std::uint32_t generation) {
    return {
        static_cast<float>(slot & 0xffU) * 0.25f +
            static_cast<float>(generation) * 0.5f,
        static_cast<float>((slot >> 8U) & 0xffU) * 0.125f,
        static_cast<float>(slot >> 16U) * 0.5f,
    };
}

std::size_t ai_target_slot(std::size_t slot) {
    return (slot * static_cast<std::size_t>(1'103'515'245) + 12'345) %
        PROJECTILE_START;
}

bool initially_stunned(std::size_t slot) {
    if (slot < COMBAT_START || slot >= COMBAT_START + STATUS_CHANGES * STATUS_COHORTS) {
        return false;
    }
    return (slot - COMBAT_START) % STATUS_COHORTS >= STATUS_DURATION;
}

ecs_table_t* find_table(ecs_world_t* world, std::vector<ecs_id_t> ids) {
    std::sort(ids.begin(), ids.end());
    return ecs_table_find(world, ids.data(), static_cast<std::int32_t>(ids.size()));
}

ecs_query_t* prepare_query(
    ecs_world_t* world,
    std::initializer_list<std::pair<ecs_entity_t, ecs_inout_kind_t>> terms) {
    ecs_query_desc_t descriptor{};
    descriptor.cache_kind = EcsQueryCacheAll;
    std::size_t index = 0;
    for (const auto& term : terms) {
        descriptor.terms[index].id = term.first;
        descriptor.terms[index].inout = term.second;
        ++index;
    }
    return ecs_query_init(world, &descriptor);
}

struct Digest {
    std::uint64_t entity_count = 0;
    std::uint64_t moving_count = 0;
    std::uint64_t health_count = 0;
    std::uint64_t lifetime_count = 0;
    std::uint64_t stunned_count = 0;
    std::uint64_t position_checksum = 0;
    std::uint64_t health_checksum = 0;
    std::uint64_t lifetime_checksum = 0;
    std::uint64_t generation_checksum = 0;
    std::uint64_t ai_lookup_checksum = 0;
};

struct Context {
    ecs_world_t* world = nullptr;
    ComponentIds components;
    std::array<ecs_table_t*, 7 * 4 * 2> tables{};
    ecs_query_t* movement = nullptr;
    ecs_query_t* enemies = nullptr;
    ecs_query_t* allies = nullptr;
    ecs_query_t* lifetimes = nullptr;
    std::vector<ecs_entity_t> entities;
    std::vector<std::uint32_t> generations;
    std::vector<ecs_entity_t> target_entities;
    std::size_t frame_index = 0;
    std::uint64_t ai_lookup_checksum = 0;

    ~Context() {
        if (movement) ecs_query_fini(movement);
        if (enemies) ecs_query_fini(enemies);
        if (allies) ecs_query_fini(allies);
        if (lifetimes) ecs_query_fini(lifetimes);
        if (world) ecs_fini(world);
    }
};

std::size_t table_index(Kind kind, std::size_t variant, bool stunned) {
    return (static_cast<std::size_t>(kind) * 4 + variant) * 2 +
        static_cast<std::size_t>(stunned);
}

ecs_table_t* table_for(Context& context, Kind kind, std::size_t variant, bool stunned) {
    ecs_table_t*& cached = context.tables[table_index(kind, variant, stunned)];
    if (cached) return cached;
    const ComponentIds& c = context.components;
    std::vector<ecs_id_t> ids{c.position};
    switch (kind) {
    case Kind::Mover:
        ids.push_back(c.velocity);
        break;
    case Kind::Enemy:
        ids.insert(ids.end(), {c.velocity, c.health, c.damage, c.enemy});
        break;
    case Kind::Ally:
        ids.insert(ids.end(), {c.velocity, c.health, c.regen, c.ally});
        break;
    case Kind::Ai:
        ids.insert(ids.end(), {c.velocity, c.health, c.target_slot, c.cooldown});
        break;
    case Kind::Projectile:
        ids.insert(ids.end(), {c.velocity, c.damage, c.lifetime, c.owner_slot});
        break;
    case Kind::Static:
        break;
    case Kind::Effect:
        ids.push_back(c.lifetime);
        break;
    }
    if (variant == 1 || variant == 3) ids.push_back(c.tag_a);
    if (variant == 2 || variant == 3) ids.push_back(c.tag_b);
    if (stunned) ids.push_back(c.stunned);
    cached = find_table(context.world, std::move(ids));
    return cached;
}

template<typename T>
void write_component(ecs_world_t* world, ecs_entity_t entity, ecs_entity_t id, T value) {
    *static_cast<T*>(ecs_get_mut_id(world, entity, id)) = value;
}

ecs_entity_t spawn_entity(
    Context& context,
    std::size_t slot,
    std::uint32_t generation,
    bool stunned) {
    const Kind kind = kind_for(slot);
    const std::size_t variant = slot % 4;
    ecs_table_t* table = table_for(context, kind, variant, stunned);
    if (!table) return 0;
    const ecs_entity_t entity = ecs_new_w_table(context.world, table);
    const ComponentIds& c = context.components;
    write_component(context.world, entity, c.position, position_for(slot, generation));
    switch (kind) {
    case Kind::Mover:
        write_component(context.world, entity, c.velocity, Velocity{1.0f, 0.5f, 0.25f});
        break;
    case Kind::Enemy:
        write_component(context.world, entity, c.velocity, Velocity{0.25f, 1.0f, 0.0f});
        write_component(context.world, entity, c.health, Health{100.0f});
        write_component(context.world, entity, c.damage, Damage{0.75f});
        break;
    case Kind::Ally:
        write_component(context.world, entity, c.velocity, Velocity{0.0f, 0.75f, 0.25f});
        write_component(context.world, entity, c.health, Health{60.0f});
        write_component(context.world, entity, c.regen, Regen{0.25f});
        break;
    case Kind::Ai:
        write_component(context.world, entity, c.velocity, Velocity{0.125f, 0.25f, 0.0f});
        write_component(context.world, entity, c.health, Health{80.0f});
        write_component(
            context.world,
            entity,
            c.target_slot,
            TargetSlot{static_cast<std::uint32_t>(ai_target_slot(slot))});
        write_component(
            context.world,
            entity,
            c.cooldown,
            Cooldown{static_cast<std::uint32_t>((slot - AI_START) % 32)});
        break;
    case Kind::Projectile:
        write_component(context.world, entity, c.velocity, Velocity{2.0f, 0.0f, 0.0f});
        write_component(context.world, entity, c.damage, Damage{1.0f});
        write_component(
            context.world,
            entity,
            c.lifetime,
            Lifetime{static_cast<std::uint32_t>(PROJECTILE_LIFETIME)});
        write_component(
            context.world,
            entity,
            c.owner_slot,
            OwnerSlot{static_cast<std::uint32_t>(
                COMBAT_START + (slot - PROJECTILE_START) % COMBAT_COUNT)});
        break;
    case Kind::Static:
        break;
    case Kind::Effect:
        write_component(context.world, entity, c.lifetime, Lifetime{256});
        break;
    }
    return entity;
}

Context* create_context() {
    auto* context = new Context();
    context->world = ecs_init();
    context->components = define_components(context->world);
    context->entities.resize(ENTITY_COUNT);
    context->generations.assign(ENTITY_COUNT, 0);
    for (std::size_t slot = 0; slot < ENTITY_COUNT; ++slot) {
        context->entities[slot] = spawn_entity(*context, slot, 0, initially_stunned(slot));
        if (!context->entities[slot]) {
            delete context;
            return nullptr;
        }
    }

    const ComponentIds& c = context->components;
    context->movement = prepare_query(
        context->world,
        {{c.position, EcsInOut}, {c.velocity, EcsIn}});
    context->enemies = prepare_query(
        context->world,
        {{c.health, EcsInOut}, {c.damage, EcsIn}});
    context->allies = prepare_query(
        context->world,
        {{c.health, EcsInOut}, {c.regen, EcsIn}});
    context->lifetimes = prepare_query(
        context->world,
        {{c.lifetime, EcsInOut}});
    if (!context->movement || !context->enemies || !context->allies ||
        !context->lifetimes) {
        delete context;
        return nullptr;
    }

    context->target_entities.reserve(AI_LOOKUPS);
    return context;
}

std::uint32_t bits(float value) {
    std::uint32_t result = 0;
    std::memcpy(&result, &value, sizeof(result));
    return result;
}

std::uint64_t mix_checksum(
    std::uint64_t checksum,
    std::uint64_t slot,
    std::uint64_t value) {
    const std::uint64_t rotated = (checksum << 7U) | (checksum >> 57U);
    return (rotated + slot * UINT64_C(0x9e3779b97f4a7c15)) ^
        (value * UINT64_C(0xbf58476d1ce4e5b9));
}

void run_movement(Context& context) {
    ecs_iter_t iterator = ecs_query_iter(context.world, context.movement);
    while (ecs_query_next(&iterator)) {
        Position* __restrict positions = ecs_field(&iterator, Position, 0);
        const Velocity* __restrict velocities = ecs_field(&iterator, Velocity, 1);
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            positions[row].x += velocities[row].x;
            positions[row].y += velocities[row].y;
            positions[row].z += velocities[row].z;
        }
    }
}

template<typename Rate>
void run_health_query(ecs_world_t* world, ecs_query_t* query, bool subtract) {
    ecs_iter_t iterator = ecs_query_iter(world, query);
    while (ecs_query_next(&iterator)) {
        Health* __restrict health = ecs_field(&iterator, Health, 0);
        const Rate* __restrict rate = ecs_field(&iterator, Rate, 1);
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            if (subtract) health[row].value -= rate[row].value;
            else health[row].value += rate[row].value;
        }
    }
}

void run_lifetimes(Context& context) {
    ecs_iter_t iterator = ecs_query_iter(context.world, context.lifetimes);
    while (ecs_query_next(&iterator)) {
        Lifetime* lifetimes = ecs_field(&iterator, Lifetime, 0);
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            lifetimes[row].value = lifetimes[row].value == 0
                ? 0
                : lifetimes[row].value - 1;
            if (lifetimes[row].value == 0) lifetimes[row].value = 256;
        }
    }
}

void run_iteration(Context& context) {
    run_movement(context);
    run_health_query<Damage>(context.world, context.enemies, true);
    run_health_query<Regen>(context.world, context.allies, false);
    run_lifetimes(context);
}

void run_ai_source(Context& context) {
    const std::size_t frame = context.frame_index;
    context.target_entities.clear();
    const std::size_t ai_cohort = frame % AI_COHORTS;
    for (std::size_t index = 0; index < AI_LOOKUPS; ++index) {
        const std::size_t slot = AI_START + ai_cohort + index * AI_COHORTS;
        const auto* target = static_cast<const TargetSlot*>(ecs_get_id(
            context.world,
            context.entities[slot],
            context.components.target_slot));
        auto* cooldown = static_cast<Cooldown*>(ecs_get_mut_id(
            context.world,
            context.entities[slot],
            context.components.cooldown));
        context.target_entities.push_back(context.entities[target->value]);
        cooldown->value = cooldown->value == 0 ? 0 : cooldown->value - 1;
    }
}

void run_target_positions(Context& context) {
    const std::size_t frame = context.frame_index;
    const std::size_t ai_cohort = frame % AI_COHORTS;
    for (std::size_t index = 0; index < AI_LOOKUPS; ++index) {
        const std::size_t slot = AI_START + ai_cohort + index * AI_COHORTS;
        const auto* position = static_cast<const Position*>(ecs_get_id(
            context.world,
            context.target_entities[index],
            context.components.position));
        context.ai_lookup_checksum = mix_checksum(
            context.ai_lookup_checksum,
            slot,
            bits(position->x));
    }
}

void run_status_transition(Context& context) {
    const std::size_t frame = context.frame_index;
    const std::size_t remove_cohort = (frame + STATUS_DURATION) % STATUS_COHORTS;
    const std::size_t add_cohort = frame % STATUS_COHORTS;
    for (std::size_t index = 0; index < STATUS_CHANGES; ++index) {
        const std::size_t slot = COMBAT_START + remove_cohort + index * STATUS_COHORTS;
        ecs_remove_id(context.world, context.entities[slot], context.components.stunned);
    }
    for (std::size_t index = 0; index < STATUS_CHANGES; ++index) {
        const std::size_t slot = COMBAT_START + add_cohort + index * STATUS_COHORTS;
        ecs_add_id(context.world, context.entities[slot], context.components.stunned);
    }
}

void run_projectile_recycle(Context& context) {
    const std::size_t frame = context.frame_index;
    const std::size_t projectile_cohort = frame % PROJECTILE_LIFETIME;
    for (std::size_t index = 0; index < PROJECTILE_RECYCLES; ++index) {
        const std::size_t slot =
            PROJECTILE_START + projectile_cohort + index * PROJECTILE_LIFETIME;
        ecs_delete(context.world, context.entities[slot]);
        const std::uint32_t generation = ++context.generations[slot];
        context.entities[slot] = spawn_entity(context, slot, generation, false);
    }
    context.frame_index = (context.frame_index + 1) % FRAME_COUNT;
}

std::uint64_t run_frame(Context& context) {
    run_iteration(context);
    run_ai_source(context);
    run_target_positions(context);
    run_status_transition(context);
    run_projectile_recycle(context);
    return context.ai_lookup_checksum;
}

Digest digest(const Context& context) {
    Digest result{};
    result.entity_count = context.entities.size();
    for (std::size_t slot = 0; slot < context.entities.size(); ++slot) {
        const ecs_entity_t entity = context.entities[slot];
        const auto* position = static_cast<const Position*>(ecs_get_id(
            context.world, entity, context.components.position));
        if (!position) continue;
        result.moving_count += ecs_has_id(
            context.world, entity, context.components.velocity);
        result.health_count += ecs_has_id(
            context.world, entity, context.components.health);
        result.lifetime_count += ecs_has_id(
            context.world, entity, context.components.lifetime);
        result.stunned_count += ecs_has_id(
            context.world, entity, context.components.stunned);
        result.position_checksum = mix_checksum(
            result.position_checksum,
            slot,
            static_cast<std::uint64_t>(bits(position->x)) ^
                (static_cast<std::uint64_t>(bits(position->y)) << 1U) ^
                (static_cast<std::uint64_t>(bits(position->z)) << 2U));
        if (const auto* health = static_cast<const Health*>(ecs_get_id(
                context.world, entity, context.components.health))) {
            result.health_checksum = mix_checksum(
                result.health_checksum, slot, bits(health->value));
        }
        if (const auto* lifetime = static_cast<const Lifetime*>(ecs_get_id(
                context.world, entity, context.components.lifetime))) {
            result.lifetime_checksum = mix_checksum(
                result.lifetime_checksum, slot, lifetime->value);
        }
        result.generation_checksum = mix_checksum(
            result.generation_checksum, slot, context.generations[slot]);
    }
    result.ai_lookup_checksum = context.ai_lookup_checksum;
    return result;
}

} // namespace sky_ecs_bench::flecs_c::gameplay_frame

using GameplayContext = sky_ecs_bench::flecs_c::gameplay_frame::Context;
using GameplayDigest = sky_ecs_bench::flecs_c::gameplay_frame::Digest;

extern "C" {

void* sky_flecs_c_gameplay_new() {
    return sky_ecs_bench::flecs_c::gameplay_frame::create_context();
}

void sky_flecs_c_gameplay_delete(void* context) {
    delete static_cast<GameplayContext*>(context);
}

std::uint64_t sky_flecs_c_gameplay_frame(void* context) {
    return sky_ecs_bench::flecs_c::gameplay_frame::run_frame(
        *static_cast<GameplayContext*>(context));
}

std::uint64_t sky_flecs_c_gameplay_iteration(void* context) {
    auto& gameplay = *static_cast<GameplayContext*>(context);
    sky_ecs_bench::flecs_c::gameplay_frame::run_iteration(gameplay);
    return gameplay.ai_lookup_checksum;
}

std::uint64_t sky_flecs_c_gameplay_ai_source(void* context) {
    auto& gameplay = *static_cast<GameplayContext*>(context);
    sky_ecs_bench::flecs_c::gameplay_frame::run_ai_source(gameplay);
    return gameplay.ai_lookup_checksum;
}

std::uint64_t sky_flecs_c_gameplay_target_positions(void* context) {
    auto& gameplay = *static_cast<GameplayContext*>(context);
    sky_ecs_bench::flecs_c::gameplay_frame::run_target_positions(gameplay);
    return gameplay.ai_lookup_checksum;
}

std::uint64_t sky_flecs_c_gameplay_status_transition(void* context) {
    auto& gameplay = *static_cast<GameplayContext*>(context);
    sky_ecs_bench::flecs_c::gameplay_frame::run_status_transition(gameplay);
    return gameplay.ai_lookup_checksum;
}

std::uint64_t sky_flecs_c_gameplay_projectile_recycle(void* context) {
    auto& gameplay = *static_cast<GameplayContext*>(context);
    sky_ecs_bench::flecs_c::gameplay_frame::run_projectile_recycle(gameplay);
    return gameplay.ai_lookup_checksum;
}

bool sky_flecs_c_gameplay_digest(void* context, GameplayDigest* digest) {
    if (!context || !digest) return false;
    *digest = sky_ecs_bench::flecs_c::gameplay_frame::digest(
        *static_cast<GameplayContext*>(context));
    return true;
}

bool sky_flecs_c_gameplay_run_trace(void* context, GameplayDigest* digest) {
    if (!context || !digest) return false;
    auto& gameplay = *static_cast<GameplayContext*>(context);
    for (std::size_t frame = 0;
         frame < sky_ecs_bench::flecs_c::gameplay_frame::FRAME_COUNT;
         ++frame) {
        sky_ecs_bench::flecs_c::gameplay_frame::run_frame(gameplay);
    }
    *digest = sky_ecs_bench::flecs_c::gameplay_frame::digest(gameplay);
    return true;
}

} // extern "C"
