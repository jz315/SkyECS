#include <flecs.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <vector>

namespace sky_ecs_bench::flecs_c::random_fragmentation {

constexpr std::size_t MAX_COMPONENT_COUNT = 16;
constexpr std::size_t CONTRACT_ENTITY_COUNT = 2'048;

enum class Storage : std::size_t {
    Tags = 0,
    Components = 1,
};

struct Value {
    float value;
};

ecs_entity_t define_tag(ecs_world_t* world, const char* name) {
    ecs_entity_desc_t descriptor{};
    descriptor.name = name;
    descriptor.symbol = name;
    descriptor.use_low_id = true;
    return ecs_entity_init(world, &descriptor);
}

ecs_entity_t define_component(ecs_world_t* world, const char* name) {
    ecs_entity_desc_t entity{};
    entity.name = name;
    entity.symbol = name;
    entity.use_low_id = true;

    ecs_component_desc_t component{};
    component.entity = ecs_entity_init(world, &entity);
    component.type.size = sizeof(Value);
    component.type.alignment = alignof(Value);
    return ecs_component_init(world, &component);
}

std::array<ecs_entity_t, MAX_COMPONENT_COUNT> define_ids(
    ecs_world_t* world,
    Storage storage) {
    constexpr std::array<const char*, MAX_COMPONENT_COUNT> tag_names{
        "RandomTagA", "RandomTagB", "RandomTagC", "RandomTagD",
        "RandomTagE", "RandomTagF", "RandomTagG", "RandomTagH",
        "RandomTagI", "RandomTagJ", "RandomTagK", "RandomTagL",
        "RandomTagM", "RandomTagN", "RandomTagO", "RandomTagP",
    };
    constexpr std::array<const char*, MAX_COMPONENT_COUNT> component_names{
        "RandomComponentA", "RandomComponentB", "RandomComponentC",
        "RandomComponentD", "RandomComponentE", "RandomComponentF",
        "RandomComponentG", "RandomComponentH", "RandomComponentI",
        "RandomComponentJ", "RandomComponentK", "RandomComponentL",
        "RandomComponentM", "RandomComponentN", "RandomComponentO",
        "RandomComponentP",
    };

    std::array<ecs_entity_t, MAX_COMPONENT_COUNT> ids{};
    for (std::size_t index = 0; index < ids.size(); ++index) {
        ids[index] = storage == Storage::Tags
            ? define_tag(world, tag_names[index])
            : define_component(world, component_names[index]);
    }
    return ids;
}

void spawn_entities(
    ecs_world_t* world,
    Storage storage,
    const std::array<ecs_entity_t, MAX_COMPONENT_COUNT>& ids,
    std::size_t component_count,
    const std::uint16_t* masks,
    std::size_t entity_count) {
    const Value value{10.0f};

    for (std::size_t entity_index = 0;
         entity_index < entity_count;
         ++entity_index) {
        const std::uint16_t mask = masks[entity_index];
        const ecs_entity_t entity = ecs_new(world);
        for (std::size_t bit = 0; bit < component_count; ++bit) {
            if ((mask & (std::uint16_t{1} << bit)) != 0) {
                if (storage == Storage::Tags) {
                    ecs_add_id(world, entity, ids[bit]);
                } else {
                    ecs_set_id(
                        world,
                        entity,
                        ids[bit],
                        sizeof(Value),
                        &value);
                }
            }
        }
    }
}

ecs_query_t* prepare_query(
    ecs_world_t* world,
    Storage storage,
    const std::array<ecs_entity_t, MAX_COMPONENT_COUNT>& ids,
    std::size_t term_count) {
    ecs_query_desc_t descriptor{};
    descriptor.cache_kind = EcsQueryCacheAll;
    for (std::size_t term = 0; term < term_count; ++term) {
        descriptor.terms[term].id = ids[term];
        if (storage == Storage::Components) {
            descriptor.terms[term].inout = EcsIn;
        }
    }
    return ecs_query_init(world, &descriptor);
}

struct Context {
    ecs_world_t* world = nullptr;
    ecs_query_t* query = nullptr;
    Storage storage = Storage::Tags;
    std::size_t term_count = 1;

    ~Context() {
        if (query) {
            ecs_query_fini(query);
        }
        if (world) {
            ecs_fini(world);
        }
    }
};

Context* create_context(
    std::size_t storage_value,
    std::size_t component_count,
    std::size_t term_count,
    const std::uint16_t* masks,
    std::size_t entity_count) {
    if (storage_value > static_cast<std::size_t>(Storage::Components) ||
        component_count == 0 || component_count > MAX_COMPONENT_COUNT ||
        !(term_count == 1 || term_count == 4 || term_count == 8) ||
        term_count > component_count || !masks || entity_count == 0) {
        return nullptr;
    }
    const std::uint16_t active_mask = component_count == MAX_COMPONENT_COUNT
        ? std::numeric_limits<std::uint16_t>::max()
        : static_cast<std::uint16_t>(
            (std::uint32_t{1} << component_count) - 1);
    for (std::size_t index = 0; index < entity_count; ++index) {
        if ((masks[index] & static_cast<std::uint16_t>(~active_mask)) != 0) {
            return nullptr;
        }
    }

    auto* context = new Context();
    context->world = ecs_init();
    context->storage = static_cast<Storage>(storage_value);
    context->term_count = term_count;

    const auto ids = define_ids(context->world, context->storage);
    spawn_entities(
        context->world,
        context->storage,
        ids,
        component_count,
        masks,
        entity_count);
    context->query = prepare_query(
        context->world,
        context->storage,
        ids,
        term_count);
    if (!context->query) {
        delete context;
        return nullptr;
    }
    return context;
}

void read_tags(Context& context, std::uint64_t& checksum) {
    ecs_iter_t iterator = ecs_query_iter(context.world, context.query);
    while (ecs_query_next(&iterator)) {
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            checksum += iterator.entities[row];
        }
    }
}

void read_one_component(Context& context, std::uint64_t& checksum) {
    ecs_iter_t iterator = ecs_query_iter(context.world, context.query);
    while (ecs_query_next(&iterator)) {
        const Value* a = ecs_field(&iterator, Value, 0);
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            checksum += iterator.entities[row] +
                static_cast<std::uint64_t>(a[row].value);
        }
    }
}

void read_four_components(Context& context, std::uint64_t& checksum) {
    ecs_iter_t iterator = ecs_query_iter(context.world, context.query);
    while (ecs_query_next(&iterator)) {
        const Value* a = ecs_field(&iterator, Value, 0);
        const Value* b = ecs_field(&iterator, Value, 1);
        const Value* c = ecs_field(&iterator, Value, 2);
        const Value* d = ecs_field(&iterator, Value, 3);
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            checksum += iterator.entities[row] +
                static_cast<std::uint64_t>(a[row].value) +
                static_cast<std::uint64_t>(b[row].value) +
                static_cast<std::uint64_t>(c[row].value) +
                static_cast<std::uint64_t>(d[row].value);
        }
    }
}

void read_eight_components(Context& context, std::uint64_t& checksum) {
    ecs_iter_t iterator = ecs_query_iter(context.world, context.query);
    while (ecs_query_next(&iterator)) {
        const Value* a = ecs_field(&iterator, Value, 0);
        const Value* b = ecs_field(&iterator, Value, 1);
        const Value* c = ecs_field(&iterator, Value, 2);
        const Value* d = ecs_field(&iterator, Value, 3);
        const Value* e = ecs_field(&iterator, Value, 4);
        const Value* f = ecs_field(&iterator, Value, 5);
        const Value* g = ecs_field(&iterator, Value, 6);
        const Value* h = ecs_field(&iterator, Value, 7);
        for (std::int32_t row = 0; row < iterator.count; ++row) {
            checksum += iterator.entities[row] +
                static_cast<std::uint64_t>(a[row].value) +
                static_cast<std::uint64_t>(b[row].value) +
                static_cast<std::uint64_t>(c[row].value) +
                static_cast<std::uint64_t>(d[row].value) +
                static_cast<std::uint64_t>(e[row].value) +
                static_cast<std::uint64_t>(f[row].value) +
                static_cast<std::uint64_t>(g[row].value) +
                static_cast<std::uint64_t>(h[row].value);
        }
    }
}

std::uint64_t iterate(Context& context) {
    std::uint64_t checksum = 0;
    if (context.storage == Storage::Tags) {
        read_tags(context, checksum);
    } else if (context.term_count == 1) {
        read_one_component(context, checksum);
    } else if (context.term_count == 4) {
        read_four_components(context, checksum);
    } else {
        read_eight_components(context, checksum);
    }
    return checksum;
}

std::uint64_t matched_count(const Context& context) {
    return static_cast<std::uint64_t>(ecs_query_count(context.query).entities);
}

std::vector<std::uint16_t> contract_masks(std::size_t component_count) {
    const std::uint16_t active_mask = component_count == 16
        ? std::numeric_limits<std::uint16_t>::max()
        : static_cast<std::uint16_t>(
            (std::uint32_t{1} << component_count) - 1);
    std::uint64_t state = 0x243F6A8885A308D3ULL;
    std::vector<std::uint16_t> masks;
    masks.reserve(CONTRACT_ENTITY_COUNT);
    for (std::size_t index = 0; index < CONTRACT_ENTITY_COUNT; ++index) {
        state += 0x9E3779B97F4A7C15ULL;
        std::uint64_t value = state;
        value = (value ^ (value >> 30)) * 0xBF58476D1CE4E5B9ULL;
        value = (value ^ (value >> 27)) * 0x94D049BB133111EBULL;
        masks.push_back(
            static_cast<std::uint16_t>(value ^ (value >> 31)) & active_mask);
    }
    return masks;
}

std::size_t expected_matches(
    const std::vector<std::uint16_t>& masks,
    std::size_t term_count) {
    const std::uint16_t query_mask =
        static_cast<std::uint16_t>((std::uint32_t{1} << term_count) - 1);
    return static_cast<std::size_t>(std::count_if(
        masks.begin(),
        masks.end(),
        [query_mask](std::uint16_t mask) {
            return (mask & query_mask) == query_mask;
        }));
}

bool validate() {
    constexpr std::array<std::pair<std::size_t, std::size_t>, 10> workloads{{
        {6, 1}, {6, 4}, {8, 1}, {8, 4}, {10, 1},
        {10, 4}, {10, 8}, {16, 1}, {16, 4}, {16, 8},
    }};

    for (Storage storage : {Storage::Tags, Storage::Components}) {
        for (const auto& [component_count, term_count] : workloads) {
            const auto masks = contract_masks(component_count);
            Context* context = create_context(
                static_cast<std::size_t>(storage),
                component_count,
                term_count,
                masks.data(),
                masks.size());
            if (!context ||
                matched_count(*context) != expected_matches(masks, term_count)) {
                delete context;
                return false;
            }
            const std::uint64_t first = iterate(*context);
            if (first == 0 || iterate(*context) != first) {
                delete context;
                return false;
            }
            delete context;
        }
    }
    return true;
}

} // namespace sky_ecs_bench::flecs_c::random_fragmentation

using RandomFragmentationContext =
    sky_ecs_bench::flecs_c::random_fragmentation::Context;

extern "C" {

void* sky_flecs_c_random_fragmented_new(
    std::size_t storage,
    std::size_t component_count,
    std::size_t term_count,
    const std::uint16_t* masks,
    std::size_t entity_count) {
    return sky_ecs_bench::flecs_c::random_fragmentation::create_context(
        storage,
        component_count,
        term_count,
        masks,
        entity_count);
}

void sky_flecs_c_random_fragmented_delete(void* context) {
    delete static_cast<RandomFragmentationContext*>(context);
}

std::uint64_t sky_flecs_c_random_fragmented_run(void* context) {
    return sky_ecs_bench::flecs_c::random_fragmentation::iterate(
        *static_cast<RandomFragmentationContext*>(context));
}

std::uint64_t sky_flecs_c_random_fragmented_count(void* context) {
    return sky_ecs_bench::flecs_c::random_fragmentation::matched_count(
        *static_cast<RandomFragmentationContext*>(context));
}

} // extern "C"
