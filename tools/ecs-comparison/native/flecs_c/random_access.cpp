#include <flecs.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <numeric>
#include <utility>
#include <vector>

namespace sky_ecs_bench::flecs_c::random_access {

constexpr std::size_t CONTRACT_ENTITY_COUNT = 128;
constexpr std::size_t ORDER_COUNT = 4;

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

template<typename Value>
void deterministic_shuffle(std::vector<Value>& values, std::uint64_t state) {
    for (std::size_t length = values.size(); length > 1; --length) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        const std::size_t other =
            static_cast<std::size_t>(state) % length;
        std::swap(values[length - 1], values[other]);
    }
}

struct Context {
    ecs_world_t* world = nullptr;
    ecs_entity_t position = 0;
    std::array<std::vector<ecs_entity_t>, ORDER_COUNT> access_orders;
    std::array<std::vector<const Position*>, ORDER_COUNT> fixed_orders;
    std::size_t next_order = 0;
    std::size_t next_fixed_order = 0;

    ~Context() {
        if (world) {
            ecs_fini(world);
        }
    }
};

Context* create_context(std::size_t entity_count) {
    auto* context = new Context();
    context->world = ecs_init();
    context->position = define_component<Position>(context->world, "Position");
    const ecs_entity_t velocity =
        define_component<Velocity>(context->world, "Velocity");

    const Position position_value{1.0f, 0.0f, 0.0f};
    const Velocity velocity_value{1.0f, 0.0f, 0.0f};
    std::vector<Position> positions(entity_count, position_value);
    std::vector<Velocity> velocities(entity_count, velocity_value);

    std::array<std::pair<ecs_id_t, void*>, 2> columns{{
        {context->position, positions.data()},
        {velocity, velocities.data()},
    }};
    std::sort(columns.begin(), columns.end(), [](const auto& left, const auto& right) {
        return left.first < right.first;
    });

    ecs_bulk_desc_t descriptor{};
    descriptor.count = static_cast<std::int32_t>(entity_count);
    std::array<void*, 2> data{};
    for (std::size_t index = 0; index < columns.size(); ++index) {
        descriptor.ids[index] = columns[index].first;
        data[index] = columns[index].second;
    }
    descriptor.data = data.data();
    const ecs_entity_t* entities = ecs_bulk_init(context->world, &descriptor);
    if (!entities) {
        delete context;
        return nullptr;
    }

    std::vector<std::size_t> indices(entity_count);
    std::iota(indices.begin(), indices.end(), std::size_t{0});
    for (std::size_t order = 0; order < ORDER_COUNT; ++order) {
        std::vector<std::size_t> shuffled = indices;
        deterministic_shuffle(
            shuffled,
            0xDEADBEEFCAFEBABEULL ^
                (order * 0x9E3779B97F4A7C15ULL));
        auto& entity_order = context->access_orders[order];
        entity_order.reserve(entity_count);
        for (std::size_t index : shuffled) {
            entity_order.push_back(entities[index]);
        }
        auto& fixed_order = context->fixed_orders[order];
        fixed_order.reserve(entity_count);
        for (ecs_entity_t entity : entity_order) {
            const auto* position = static_cast<const Position*>(
                ecs_get_id(context->world, entity, context->position));
            if (!position) {
                delete context;
                return nullptr;
            }
            fixed_order.push_back(position);
        }
    }
    return context;
}

std::uint64_t read_fixed_positions(Context& context) {
    const auto& order =
        context.fixed_orders[context.next_fixed_order++ % ORDER_COUNT];
    std::uint64_t checksum = 0;
    for (const Position* position : order) {
        std::uint32_t x_bits = 0;
        std::memcpy(&x_bits, &position->x, sizeof(x_bits));
        checksum += x_bits;
    }
    return checksum;
}

std::uint64_t build_and_read_fixed_positions(
    Context& context,
    std::size_t repeats) {
    const auto& entities =
        context.access_orders[context.next_fixed_order++ % ORDER_COUNT];
    std::vector<const Position*> positions;
    positions.reserve(entities.size());
    for (ecs_entity_t entity : entities) {
        const auto* position = static_cast<const Position*>(
            ecs_get_id(context.world, entity, context.position));
        if (!position) {
            return 0;
        }
        positions.push_back(position);
    }
    if (repeats == 0) {
        return positions.size();
    }

    std::uint64_t checksum = 0;
    for (std::size_t repeat = 0; repeat < repeats; ++repeat) {
        for (const Position* position : positions) {
            std::uint32_t x_bits = 0;
            std::memcpy(&x_bits, &position->x, sizeof(x_bits));
            checksum += x_bits;
        }
    }
    return checksum;
}

std::uint64_t read_positions(Context& context) {
    auto& order =
        context.access_orders[context.next_order++ % ORDER_COUNT];
    std::uint64_t checksum = 0;
    for (ecs_entity_t entity : order) {
        const auto* position = static_cast<const Position*>(
            ecs_get_id(context.world, entity, context.position));
        if (!position) {
            return 0;
        }
        std::uint32_t x_bits = 0;
        std::memcpy(&x_bits, &position->x, sizeof(x_bits));
        checksum += x_bits;
    }
    return checksum;
}

bool validate_count(std::size_t entity_count) {
    Context* context = create_context(entity_count);
    if (!context) {
        return false;
    }
    const std::uint64_t expected =
        static_cast<std::uint64_t>(entity_count) * 0x3F800000ULL;
    bool valid = true;
    for (std::size_t order = 0; order < ORDER_COUNT; ++order) {
        valid = valid && read_positions(*context) == expected;
    }
    for (std::size_t order = 0; order < ORDER_COUNT; ++order) {
        valid = valid && read_fixed_positions(*context) == expected;
    }
    for (std::size_t order = 0; order < ORDER_COUNT; ++order) {
        valid = valid &&
            build_and_read_fixed_positions(*context, 1) == expected;
    }
    delete context;
    return valid;
}

bool validate() {
    return validate_count(CONTRACT_ENTITY_COUNT);
}

} // namespace sky_ecs_bench::flecs_c::random_access

using RandomAccessContext = sky_ecs_bench::flecs_c::random_access::Context;

extern "C" {

void* sky_flecs_c_random_new(std::size_t entity_count) {
    return sky_ecs_bench::flecs_c::random_access::create_context(entity_count);
}

void sky_flecs_c_random_delete(void* context) {
    delete static_cast<RandomAccessContext*>(context);
}

std::uint64_t sky_flecs_c_random_run(void* context) {
    return sky_ecs_bench::flecs_c::random_access::read_positions(
        *static_cast<RandomAccessContext*>(context));
}

std::uint64_t sky_flecs_c_fixed_sequence_build_run(
    void* context,
    std::size_t repeats) {
    return sky_ecs_bench::flecs_c::random_access::build_and_read_fixed_positions(
        *static_cast<RandomAccessContext*>(context),
        repeats);
}

std::uint64_t sky_flecs_c_fixed_sequence_steady_run(void* context) {
    return sky_ecs_bench::flecs_c::random_access::read_fixed_positions(
        *static_cast<RandomAccessContext*>(context));
}

} // extern "C"
