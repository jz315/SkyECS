#include <flecs.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <utility>
#include <vector>

namespace sky_ecs_bench::flecs_c::parallel {

constexpr std::size_t DENSE_ENTITY_COUNT = 1'048'576;
constexpr std::size_t COMPUTE_ENTITY_COUNT = 262'144;
constexpr std::size_t FRAGMENT_SHAPES = 64;
constexpr std::size_t ENTITIES_PER_FRAGMENT = 1'024;

enum class Workload : std::uint32_t {
    DenseBandwidth = 0,
    DenseCompute = 1,
    FragmentedBandwidth = 2,
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

struct Rotation {
    float x;
    float y;
    float z;
};

struct Data {
    float value;
};

static_assert(sizeof(Position) == 12 && alignof(Position) == 4);
static_assert(sizeof(Velocity) == 12 && alignof(Velocity) == 4);
static_assert(sizeof(Rotation) == 12 && alignof(Rotation) == 4);
static_assert(sizeof(Data) == 4 && alignof(Data) == 4);

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
    ecs_entity_t rotation = 0;
    ecs_entity_t data = 0;
    std::array<ecs_entity_t, 6> tags{};
};

struct Context {
    ecs_world_t* world = nullptr;
    ecs_query_t* checksum_query = nullptr;
    ecs_entity_t system = 0;
    ComponentIds ids;
    Workload workload = Workload::DenseBandwidth;
    std::uint64_t frames = 0;

    ~Context() {
        if (checksum_query) {
            ecs_query_fini(checksum_query);
        }
        if (world) {
            ecs_set_threads(world, 0);
            ecs_fini(world);
        }
    }
};

ComponentIds define_components(ecs_world_t* world) {
    ComponentIds ids{
        define_component<Position>(world, "ParallelPosition"),
        define_component<Velocity>(world, "ParallelVelocity"),
        define_component<Rotation>(world, "ParallelRotation"),
        define_component<Data>(world, "ParallelData"),
        {},
    };
    constexpr std::array<const char*, 6> names{
        "ParallelTagA", "ParallelTagB", "ParallelTagC",
        "ParallelTagD", "ParallelTagE", "ParallelTagF",
    };
    for (std::size_t index = 0; index < names.size(); ++index) {
        ids.tags[index] = define_tag(world, names[index]);
    }
    return ids;
}

void fill_columns(
    std::vector<Position>& positions,
    std::vector<Velocity>& velocities,
    std::vector<Rotation>& rotations,
    std::vector<Data>& data,
    std::size_t logical_start) {
    for (std::size_t row = 0; row < positions.size(); ++row) {
        const float base = static_cast<float>(logical_start + row);
        positions[row] = {base, base * 0.5f, base * 0.25f};
        velocities[row] = {1.0f, 2.0f, 3.0f};
        rotations[row] = {0.25f, 0.5f, 0.75f};
        data[row] = {0.125f};
    }
}

void bulk_insert(
    Context& context,
    std::size_t count,
    std::size_t logical_start,
    std::uint32_t tag_mask) {
    std::vector<Position> positions(count);
    std::vector<Velocity> velocities(count);
    std::vector<Rotation> rotations(count);
    std::vector<Data> data(count);
    fill_columns(positions, velocities, rotations, data, logical_start);

    std::vector<std::pair<ecs_id_t, void*>> columns{
        {context.ids.position, positions.data()},
        {context.ids.velocity, velocities.data()},
        {context.ids.rotation, rotations.data()},
        {context.ids.data, data.data()},
    };
    for (std::size_t bit = 0; bit < context.ids.tags.size(); ++bit) {
        if ((tag_mask & (1u << bit)) != 0) {
            columns.emplace_back(context.ids.tags[bit], nullptr);
        }
    }
    std::sort(columns.begin(), columns.end(), [](const auto& left, const auto& right) {
        return left.first < right.first;
    });

    ecs_bulk_desc_t descriptor{};
    descriptor.count = static_cast<std::int32_t>(count);
    std::array<void*, FLECS_ID_DESC_MAX> component_data{};
    for (std::size_t index = 0; index < columns.size(); ++index) {
        descriptor.ids[index] = columns[index].first;
        component_data[index] = columns[index].second;
    }
    descriptor.data = component_data.data();
    ecs_bulk_init(context.world, &descriptor);
}

void bandwidth_callback(ecs_iter_t* iterator) {
    Position* positions = ecs_field(iterator, Position, 0);
    const Velocity* velocities = ecs_field(iterator, Velocity, 1);
    for (std::int32_t row = 0; row < iterator->count; ++row) {
        positions[row].x += velocities[row].x;
        positions[row].y += velocities[row].y;
        positions[row].z += velocities[row].z;
    }
}

void compute_callback(ecs_iter_t* iterator) {
    Position* positions = ecs_field(iterator, Position, 0);
    const Velocity* velocities = ecs_field(iterator, Velocity, 1);
    const Rotation* rotations = ecs_field(iterator, Rotation, 2);
    const Data* data = ecs_field(iterator, Data, 3);
    for (std::int32_t row = 0; row < iterator->count; ++row) {
        float x = positions[row].x;
        float y = positions[row].y;
        float z = positions[row].z;
        for (std::size_t iteration = 0; iteration < 8; ++iteration) {
            x = (x + velocities[row].x * 0.25f + rotations[row].x * data[row].value) * 0.9995f;
            y = (y + velocities[row].y * 0.25f + rotations[row].y * data[row].value) * 0.9995f;
            z = (z + velocities[row].z * 0.25f + rotations[row].z * data[row].value) * 0.9995f;
        }
        positions[row] = {x, y, z};
    }
}

ecs_query_t* create_checksum_query(Context& context) {
    ecs_query_desc_t descriptor{};
    descriptor.cache_kind = EcsQueryCacheAll;
    descriptor.terms[0].id = context.ids.position;
    descriptor.terms[0].inout = EcsIn;
    return ecs_query_init(context.world, &descriptor);
}

ecs_entity_t create_system(Context& context, std::uint32_t threads) {
    ecs_system_desc_t descriptor{};
    descriptor.phase = EcsOnUpdate;
    descriptor.query.cache_kind = EcsQueryCacheAll;
    descriptor.query.terms[0].id = context.ids.position;
    descriptor.query.terms[0].inout = EcsInOut;
    descriptor.query.terms[1].id = context.ids.velocity;
    descriptor.query.terms[1].inout = EcsIn;
    if (context.workload == Workload::DenseCompute) {
        descriptor.query.terms[2].id = context.ids.rotation;
        descriptor.query.terms[2].inout = EcsIn;
        descriptor.query.terms[3].id = context.ids.data;
        descriptor.query.terms[3].inout = EcsIn;
        descriptor.callback = compute_callback;
    } else {
        descriptor.callback = bandwidth_callback;
    }
    descriptor.multi_threaded = threads > 1;
    return ecs_system_init(context.world, &descriptor);
}

Context* create_context(Workload workload, std::uint32_t threads) {
    if (threads == 0) {
        return nullptr;
    }
    auto* context = new Context();
    context->world = ecs_init();
    context->workload = workload;
    context->ids = define_components(context->world);
    switch (workload) {
    case Workload::DenseBandwidth:
        bulk_insert(*context, DENSE_ENTITY_COUNT, 0, 0);
        break;
    case Workload::DenseCompute:
        bulk_insert(*context, COMPUTE_ENTITY_COUNT, 0, 0);
        break;
    case Workload::FragmentedBandwidth:
        for (std::size_t shape = 0; shape < FRAGMENT_SHAPES; ++shape) {
            bulk_insert(
                *context,
                ENTITIES_PER_FRAGMENT,
                shape * ENTITIES_PER_FRAGMENT,
                static_cast<std::uint32_t>(shape));
        }
        break;
    }
    context->checksum_query = create_checksum_query(*context);
    ecs_set_threads(context->world, static_cast<std::int32_t>(threads));
    context->system = create_system(*context, threads);
    if (!context->checksum_query || !context->system) {
        delete context;
        return nullptr;
    }
    return context;
}

std::uint64_t checksum(Context& context) {
    std::uint64_t result = 0;
    ecs_iter_t iterator = ecs_query_iter(context.world, context.checksum_query);
    while (ecs_query_next(&iterator)) {
        const Position* positions = ecs_field(&iterator, Position, 0);
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            std::array<std::uint32_t, 3> bits{};
            std::memcpy(&bits[0], &positions[row].x, sizeof(float));
            std::memcpy(&bits[1], &positions[row].y, sizeof(float));
            std::memcpy(&bits[2], &positions[row].z, sizeof(float));
            std::uint64_t hash = static_cast<std::uint64_t>(bits[0]) ^ 0x9e3779b97f4a7c15ULL;
            hash = (hash ^ static_cast<std::uint64_t>(bits[1])) * 0xbf58476d1ce4e5b9ULL;
            hash = (hash ^ static_cast<std::uint64_t>(bits[2])) * 0x94d049bb133111ebULL;
            result += hash ^ (hash >> 31);
        }
    }
    return result;
}

} // namespace sky_ecs_bench::flecs_c::parallel

extern "C" {

void* sky_flecs_c_parallel_new(std::uint32_t workload, std::uint32_t threads) {
    using namespace sky_ecs_bench::flecs_c::parallel;
    if (workload > static_cast<std::uint32_t>(Workload::FragmentedBandwidth)) {
        return nullptr;
    }
    return create_context(static_cast<Workload>(workload), threads);
}

void sky_flecs_c_parallel_delete(void* pointer) {
    delete static_cast<sky_ecs_bench::flecs_c::parallel::Context*>(pointer);
}

std::uint64_t sky_flecs_c_parallel_run(void* pointer) {
    using namespace sky_ecs_bench::flecs_c::parallel;
    auto& context = *static_cast<Context*>(pointer);
    ecs_progress(context.world, 0.0f);
    return ++context.frames;
}

std::uint64_t sky_flecs_c_parallel_checksum(void* pointer) {
    using namespace sky_ecs_bench::flecs_c::parallel;
    return checksum(*static_cast<Context*>(pointer));
}

}
