#include <flecs.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>

namespace sky_ecs_bench::flecs_c::fragmented_iteration {

constexpr std::size_t VARIANT_COUNT = 26;
constexpr std::size_t ENTITIES_PER_VARIANT = 400;

struct Data {
    float value;
};

struct Marker {
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

std::array<ecs_entity_t, VARIANT_COUNT> define_markers(ecs_world_t* world) {
    constexpr std::array<const char*, VARIANT_COUNT> names{
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K",
        "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V",
        "W", "X", "Y", "Z",
    };
    std::array<ecs_entity_t, VARIANT_COUNT> markers{};
    for (std::size_t index = 0; index < markers.size(); ++index) {
        markers[index] = define_component<Marker>(world, names[index]);
    }
    return markers;
}

void spawn_variants(
    ecs_world_t* world,
    ecs_entity_t data_id,
    const std::array<ecs_entity_t, VARIANT_COUNT>& markers) {
    const Data data{1.0f};
    const Marker marker{0.0f};

    for (ecs_entity_t marker_id : markers) {
        std::array<ecs_id_t, 2> table_ids{data_id, marker_id};
        std::sort(table_ids.begin(), table_ids.end());
        ecs_table_t* table = ecs_table_find(
            world,
            table_ids.data(),
            static_cast<std::int32_t>(table_ids.size()));
        if (!table) {
            return;
        }

        for (std::size_t row = 0; row < ENTITIES_PER_VARIANT; ++row) {
            const ecs_entity_t entity = ecs_new_w_table(world, table);
            ecs_set_id(world, entity, data_id, sizeof(Data), &data);
            ecs_set_id(world, entity, marker_id, sizeof(Marker), &marker);
        }
    }
}

ecs_query_t* prepare_query(ecs_world_t* world, ecs_entity_t data_id) {
    ecs_query_desc_t descriptor{};
    descriptor.cache_kind = EcsQueryCacheAll;
    descriptor.terms[0].id = data_id;
    descriptor.terms[0].inout = EcsInOut;
    return ecs_query_init(world, &descriptor);
}

struct Context {
    ecs_world_t* world = nullptr;
    ecs_query_t* query = nullptr;

    ~Context() {
        if (query) {
            ecs_query_fini(query);
        }
        if (world) {
            ecs_fini(world);
        }
    }
};

Context* create_context() {
    auto* context = new Context();
    context->world = ecs_init();
    const ecs_entity_t data =
        define_component<Data>(context->world, "Data");
    const auto markers = define_markers(context->world);
    spawn_variants(context->world, data, markers);
    context->query = prepare_query(context->world, data);
    if (!context->query) {
        delete context;
        return nullptr;
    }
    return context;
}

std::uint64_t iterate(Context& context) {
    ecs_iter_t iterator = ecs_query_iter(context.world, context.query);
    while (ecs_query_next(&iterator)) {
        Data* values = ecs_field(&iterator, Data, 0);
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            values[row].value = -values[row].value;
        }
    }
    return 1;
}

bool validate() {
    Context* context = create_context();
    if (!context || iterate(*context) != 1) {
        delete context;
        return false;
    }

    std::size_t entity_count = 0;
    float sum = 0.0f;
    ecs_iter_t iterator = ecs_query_iter(context->world, context->query);
    while (ecs_query_next(&iterator)) {
        const Data* values = ecs_field(&iterator, Data, 0);
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            ++entity_count;
            sum += values[row].value;
        }
    }
    delete context;

    constexpr std::size_t expected = VARIANT_COUNT * ENTITIES_PER_VARIANT;
    return entity_count == expected && sum == -static_cast<float>(expected);
}

} // namespace sky_ecs_bench::flecs_c::fragmented_iteration

using FragmentedContext =
    sky_ecs_bench::flecs_c::fragmented_iteration::Context;

extern "C" {

void* sky_flecs_c_fragmented_new() {
    return sky_ecs_bench::flecs_c::fragmented_iteration::create_context();
}

void sky_flecs_c_fragmented_delete(void* context) {
    delete static_cast<FragmentedContext*>(context);
}

std::uint64_t sky_flecs_c_fragmented_run(void* context) {
    return sky_ecs_bench::flecs_c::fragmented_iteration::iterate(
        *static_cast<FragmentedContext*>(context));
}

} // extern "C"
