#include <flecs.h>

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <iterator>
#include <numeric>
#include <utility>
#include <vector>

namespace {

constexpr std::size_t SIMPLE_ENTITY_COUNT = 10'000;
constexpr std::size_t REPEATED_ITERATION_COUNT = 32;
constexpr std::size_t FRAGMENTED_ENTITIES_PER_VARIANT = 400;
constexpr std::size_t HEAVY_ENTITY_COUNT = 1'000;
constexpr std::size_t HEAVY_INVERT_COUNT = 100;
constexpr std::size_t ENTITY_OP_COUNT = 1'000;
constexpr std::size_t MIXED_FRAME_MOVERS = 16'000;
constexpr std::size_t MIXED_FRAME_ENEMIES = 4'000;
constexpr std::size_t MIXED_FRAME_ALLIES = 4'000;
constexpr std::size_t MIXED_FRAME_HEAVY = 1'000;
constexpr std::size_t MIXED_FRAME_RANDOM_COUNT = 512;
constexpr std::size_t MIXED_FRAME_CHURN_COUNT = 256;
constexpr std::size_t MIXED_FRAME_SPAWN_COUNT = 64;
constexpr std::size_t MIXED_FRAME_INVERT_COUNT = 8;
constexpr std::size_t MIXED_PHASE_HEALTH_REPEAT = 8;
constexpr std::size_t MIXED_PHASE_SPAWN_REPEAT = 32;
constexpr std::size_t CONTRACT_ENTITY_COUNT = 128;
constexpr std::size_t CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT = 2'048;

struct Vec3 {
    float x;
    float y;
    float z;
};

struct Mat4 {
    float value[16];
};

struct TransformComponent { Mat4 value; };
struct PositionComponent { Vec3 value; };
struct RotationComponent { Vec3 value; };
struct VelocityComponent { Vec3 value; };
struct DataComponent { float value; };
struct Health { float value; };
struct Damage { float value; };
struct Regen { float value; };
struct IsEnemy { };
struct IsAlly { };

#define DEFINE_FRAGMENT_COMPONENT(name) struct name { float value; }
DEFINE_FRAGMENT_COMPONENT(A); DEFINE_FRAGMENT_COMPONENT(B);
DEFINE_FRAGMENT_COMPONENT(C); DEFINE_FRAGMENT_COMPONENT(D);
DEFINE_FRAGMENT_COMPONENT(E); DEFINE_FRAGMENT_COMPONENT(F);
DEFINE_FRAGMENT_COMPONENT(G); DEFINE_FRAGMENT_COMPONENT(H);
DEFINE_FRAGMENT_COMPONENT(I); DEFINE_FRAGMENT_COMPONENT(J);
DEFINE_FRAGMENT_COMPONENT(K); DEFINE_FRAGMENT_COMPONENT(L);
DEFINE_FRAGMENT_COMPONENT(M); DEFINE_FRAGMENT_COMPONENT(N);
DEFINE_FRAGMENT_COMPONENT(O); DEFINE_FRAGMENT_COMPONENT(P);
DEFINE_FRAGMENT_COMPONENT(Q); DEFINE_FRAGMENT_COMPONENT(R);
DEFINE_FRAGMENT_COMPONENT(S); DEFINE_FRAGMENT_COMPONENT(T);
DEFINE_FRAGMENT_COMPONENT(U); DEFINE_FRAGMENT_COMPONENT(V);
DEFINE_FRAGMENT_COMPONENT(W); DEFINE_FRAGMENT_COMPONENT(X);
DEFINE_FRAGMENT_COMPONENT(Y); DEFINE_FRAGMENT_COMPONENT(Z);
#undef DEFINE_FRAGMENT_COMPONENT

Mat4 identity_matrix() {
    Mat4 matrix{};
    matrix.value[0] = 1.0f;
    matrix.value[5] = 1.0f;
    matrix.value[10] = 1.0f;
    matrix.value[15] = 1.0f;
    return matrix;
}

Mat4 heavy_matrix() {
    Mat4 matrix = identity_matrix();
    const float cosine = std::cos(1.2f);
    const float sine = std::sin(1.2f);
    matrix.value[5] = cosine;
    matrix.value[6] = sine;
    matrix.value[9] = -sine;
    matrix.value[10] = cosine;
    return matrix;
}

bool invert(const Mat4& source, Mat4& destination) {
    const float* m = source.value;
    float inverse[16];

    inverse[0] = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] -
        m[9] * m[6] * m[15] + m[9] * m[7] * m[14] +
        m[13] * m[6] * m[11] - m[13] * m[7] * m[10];
    inverse[4] = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] +
        m[8] * m[6] * m[15] - m[8] * m[7] * m[14] -
        m[12] * m[6] * m[11] + m[12] * m[7] * m[10];
    inverse[8] = m[4] * m[9] * m[15] - m[4] * m[11] * m[13] -
        m[8] * m[5] * m[15] + m[8] * m[7] * m[13] +
        m[12] * m[5] * m[11] - m[12] * m[7] * m[9];
    inverse[12] = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] +
        m[8] * m[5] * m[14] - m[8] * m[6] * m[13] -
        m[12] * m[5] * m[10] + m[12] * m[6] * m[9];
    inverse[1] = -m[1] * m[10] * m[15] + m[1] * m[11] * m[14] +
        m[9] * m[2] * m[15] - m[9] * m[3] * m[14] -
        m[13] * m[2] * m[11] + m[13] * m[3] * m[10];
    inverse[5] = m[0] * m[10] * m[15] - m[0] * m[11] * m[14] -
        m[8] * m[2] * m[15] + m[8] * m[3] * m[14] +
        m[12] * m[2] * m[11] - m[12] * m[3] * m[10];
    inverse[9] = -m[0] * m[9] * m[15] + m[0] * m[11] * m[13] +
        m[8] * m[1] * m[15] - m[8] * m[3] * m[13] -
        m[12] * m[1] * m[11] + m[12] * m[3] * m[9];
    inverse[13] = m[0] * m[9] * m[14] - m[0] * m[10] * m[13] -
        m[8] * m[1] * m[14] + m[8] * m[2] * m[13] +
        m[12] * m[1] * m[10] - m[12] * m[2] * m[9];
    inverse[2] = m[1] * m[6] * m[15] - m[1] * m[7] * m[14] -
        m[5] * m[2] * m[15] + m[5] * m[3] * m[14] +
        m[13] * m[2] * m[7] - m[13] * m[3] * m[6];
    inverse[6] = -m[0] * m[6] * m[15] + m[0] * m[7] * m[14] +
        m[4] * m[2] * m[15] - m[4] * m[3] * m[14] -
        m[12] * m[2] * m[7] + m[12] * m[3] * m[6];
    inverse[10] = m[0] * m[5] * m[15] - m[0] * m[7] * m[13] -
        m[4] * m[1] * m[15] + m[4] * m[3] * m[13] +
        m[12] * m[1] * m[7] - m[12] * m[3] * m[5];
    inverse[14] = -m[0] * m[5] * m[14] + m[0] * m[6] * m[13] +
        m[4] * m[1] * m[14] - m[4] * m[2] * m[13] -
        m[12] * m[1] * m[6] + m[12] * m[2] * m[5];
    inverse[3] = -m[1] * m[6] * m[11] + m[1] * m[7] * m[10] +
        m[5] * m[2] * m[11] - m[5] * m[3] * m[10] -
        m[9] * m[2] * m[7] + m[9] * m[3] * m[6];
    inverse[7] = m[0] * m[6] * m[11] - m[0] * m[7] * m[10] -
        m[4] * m[2] * m[11] + m[4] * m[3] * m[10] +
        m[8] * m[2] * m[7] - m[8] * m[3] * m[6];
    inverse[11] = -m[0] * m[5] * m[11] + m[0] * m[7] * m[9] +
        m[4] * m[1] * m[11] - m[4] * m[3] * m[9] -
        m[8] * m[1] * m[7] + m[8] * m[3] * m[5];
    inverse[15] = m[0] * m[5] * m[10] - m[0] * m[6] * m[9] -
        m[4] * m[1] * m[10] + m[4] * m[2] * m[9] +
        m[8] * m[1] * m[6] - m[8] * m[2] * m[5];

    const float determinant = m[0] * inverse[0] + m[1] * inverse[4] +
        m[2] * inverse[8] + m[3] * inverse[12];
    if (determinant == 0.0f) {
        return false;
    }
    const float reciprocal = 1.0f / determinant;
    for (std::size_t index = 0; index < 16; ++index) {
        destination.value[index] = inverse[index] * reciprocal;
    }
    return true;
}

#if defined(_MSC_VER)
#define SKY_NOINLINE __declspec(noinline)
#else
#define SKY_NOINLINE __attribute__((noinline))
#endif

SKY_NOINLINE bool invert_opaque(const Mat4& source, Mat4& destination) {
    return invert(source, destination);
}

Vec3 transform_vector(const Mat4& matrix, const Vec3& vector) {
    return {
        matrix.value[0] * vector.x + matrix.value[4] * vector.y + matrix.value[8] * vector.z,
        matrix.value[1] * vector.x + matrix.value[5] * vector.y + matrix.value[9] * vector.z,
        matrix.value[2] * vector.x + matrix.value[6] * vector.y + matrix.value[10] * vector.z,
    };
}

void register_components(flecs::world& world) {
    world.component<TransformComponent>();
    world.component<PositionComponent>();
    world.component<RotationComponent>();
    world.component<VelocityComponent>();
    world.component<DataComponent>();
    world.component<Health>();
    world.component<Damage>();
    world.component<Regen>();
    world.component<IsEnemy>();
    world.component<IsAlly>();
}

PositionComponent position(float x = 1.0f, float y = 0.0f, float z = 0.0f) {
    return {{x, y, z}};
}

VelocityComponent velocity(float x = 1.0f, float y = 0.0f, float z = 0.0f) {
    return {{x, y, z}};
}

void spawn_full(flecs::world& world, bool heavy = false) {
    world.entity()
        .set<TransformComponent>({heavy ? heavy_matrix() : identity_matrix()})
        .set<PositionComponent>(position())
        .set<RotationComponent>({{1.0f, 0.0f, 0.0f}})
        .set<VelocityComponent>(velocity());
}

void spawn_light(flecs::world& world, std::vector<flecs::entity_t>* ids = nullptr) {
    auto entity = world.entity();
    entity.insert([](PositionComponent& current_position, VelocityComponent& current_velocity) {
        current_position = position();
        current_velocity = velocity();
    });
    if (ids) {
        ids->push_back(entity.id());
    }
}

template<std::size_t N>
ecs_table_t* find_table(flecs::world& world, std::array<ecs_id_t, N> ids) {
    std::sort(ids.begin(), ids.end());
    ecs_table_t* table = ecs_table_find(
        world.c_ptr(), ids.data(), static_cast<std::int32_t>(ids.size()));
    if (!table) {
        std::abort();
    }
    return table;
}

struct LightTarget {
    ecs_table_t* table;

    explicit LightTarget(flecs::world& world) : table(nullptr) {
        const auto position_id = world.component<PositionComponent>().id();
        const auto velocity_id = world.component<VelocityComponent>().id();
        table = find_table(
            world, std::array<ecs_id_t, 2>{position_id, velocity_id});
    }

    flecs::entity_t spawn(flecs::world& world) const {
        const ecs_entity_t entity = ecs_new_w_table(world.c_ptr(), table);
        if (!entity) {
            std::abort();
        }
        return entity;
    }
};

struct InsertData;

struct SuiteTarget {
    ecs_entity_t transform_id;
    ecs_entity_t position_id;
    ecs_entity_t rotation_id;
    ecs_entity_t velocity_id;
    std::array<ecs_id_t, 4> table_ids;
    ecs_table_t* table;

    explicit SuiteTarget(flecs::world& world)
        : transform_id(world.component<TransformComponent>().id()),
          position_id(world.component<PositionComponent>().id()),
          rotation_id(world.component<RotationComponent>().id()),
          velocity_id(world.component<VelocityComponent>().id()),
          table_ids{transform_id, position_id, rotation_id, velocity_id},
          table(nullptr) {
        std::sort(table_ids.begin(), table_ids.end());
        table = find_table(world, table_ids);
    }

    void* data_for(
        ecs_id_t id,
        InsertData& input) const;

    ecs_entity_t spawn(flecs::world& world) const {
        const ecs_entity_t entity = ecs_new_w_table(world.c_ptr(), table);
        if (!entity) {
            std::abort();
        }
        return entity;
    }
};

template<typename Query>
auto uncached(Query&& builder) {
    return builder.cache_kind(flecs::QueryCacheNone).build();
}

template<typename Query>
auto cached(Query&& builder) {
    return builder.cache_kind(flecs::QueryCacheAll).build();
}

struct InsertData {
    std::vector<TransformComponent> transforms;
    std::vector<PositionComponent> positions;
    std::vector<RotationComponent> rotations;
    std::vector<VelocityComponent> velocities;

    explicit InsertData(std::size_t count)
        : transforms(count, {identity_matrix()}),
          positions(count, position()),
          rotations(count, RotationComponent{{1.0f, 0.0f, 0.0f}}),
          velocities(count, velocity()) { }
};

InsertData& insert_data() {
    static InsertData data(SIMPLE_ENTITY_COUNT);
    return data;
}

struct InsertContext {
    flecs::world world;
    SuiteTarget target;

    InsertContext() : world(), target(world) {
        static_cast<void>(insert_data());
    }
};

void* SuiteTarget::data_for(ecs_id_t id, InsertData& input) const {
    if (id == transform_id) { return input.transforms.data(); }
    if (id == position_id) { return input.positions.data(); }
    if (id == rotation_id) { return input.rotations.data(); }
    if (id == velocity_id) { return input.velocities.data(); }
    std::abort();
}

const ecs_entity_t* bulk_insert_raw(
    InsertContext& context,
    const SuiteTarget& target,
    InsertData& input,
    std::size_t count) {
    ecs_bulk_desc_t descriptor{};
    descriptor.count = static_cast<std::int32_t>(count);
    std::copy(
        target.table_ids.begin(),
        target.table_ids.end(),
        descriptor.ids);
    void* component_data[] = {
        target.data_for(target.table_ids[0], input),
        target.data_for(target.table_ids[1], input),
        target.data_for(target.table_ids[2], input),
        target.data_for(target.table_ids[3], input),
    };
    descriptor.data = component_data;
    // IDs and component data share the same sorted order. InsertContext has
    // already created the target table, so this follows cached table edges.
    return ecs_bulk_init(context.world.c_ptr(), &descriptor);
}

std::uint64_t bulk_insert(InsertContext& context) {
    auto& input = insert_data();
    const ecs_entity_t* entities = bulk_insert_raw(
        context, context.target, input, SIMPLE_ENTITY_COUNT);
    return entities ? entities[SIMPLE_ENTITY_COUNT - 1] : 0;
}

template<typename T>
void set_existing(
    flecs::world& world,
    ecs_entity_t entity,
    ecs_entity_t component,
    const T& value) {
    ecs_set_id(world.c_ptr(), entity, component, sizeof(T), &value);
}

std::uint64_t single_insert_raw(
    InsertContext& context,
    const SuiteTarget& target,
    const InsertData& input,
    std::size_t count,
    std::vector<flecs::entity_t>* ids = nullptr) {
    std::uint64_t last = 0;
    for (std::size_t index = 0; index < count; ++index) {
        const ecs_entity_t entity = target.spawn(context.world);
        set_existing(
            context.world, entity, target.transform_id, input.transforms[index]);
        set_existing(
            context.world, entity, target.position_id, input.positions[index]);
        set_existing(
            context.world, entity, target.rotation_id, input.rotations[index]);
        set_existing(
            context.world, entity, target.velocity_id, input.velocities[index]);
        last = entity;
        if (ids) {
            ids->push_back(entity);
        }
    }
    return last;
}

bool validate_suite_values(flecs::world& world, std::size_t expected_count) {
    auto query = uncached(world.query_builder<
        const TransformComponent,
        const PositionComponent,
        const RotationComponent,
        const VelocityComponent>());
    std::size_t count = 0;
    bool values_match = true;
    query.each([&](
        const TransformComponent& transform,
        const PositionComponent& current_position,
        const RotationComponent& rotation,
        const VelocityComponent& current_velocity) {
        ++count;
        const Mat4 identity = identity_matrix();
        values_match = values_match &&
            std::equal(
                std::begin(transform.value.value),
                std::end(transform.value.value),
                std::begin(identity.value)) &&
            current_position.value.x == 1.0f &&
            current_position.value.y == 0.0f &&
            current_position.value.z == 0.0f &&
            rotation.value.x == 1.0f &&
            rotation.value.y == 0.0f &&
            rotation.value.z == 0.0f &&
            current_velocity.value.x == 1.0f &&
            current_velocity.value.y == 0.0f &&
            current_velocity.value.z == 0.0f;
    });
    return count == expected_count && values_match;
}

void make_distinct_insert_data(InsertData& data) {
    for (std::size_t row = 0; row < data.transforms.size(); ++row) {
        for (std::size_t column = 0; column < 16; ++column) {
            data.transforms[row].value.value[column] =
                static_cast<float>(1'000 + row * 16 + column);
        }
        data.positions[row] = position(
            static_cast<float>(2'000 + row),
            static_cast<float>(2'100 + row),
            static_cast<float>(2'200 + row));
        data.rotations[row] = {{
            static_cast<float>(3'000 + row),
            static_cast<float>(3'100 + row),
            static_cast<float>(3'200 + row)}};
        data.velocities[row] = velocity(
            static_cast<float>(4'000 + row),
            static_cast<float>(4'100 + row),
            static_cast<float>(4'200 + row));
    }
}

bool validate_distinct_entity(
    flecs::world& world,
    flecs::entity_t id,
    const InsertData& expected,
    std::size_t row) {
    auto entity = world.entity(id);
    const auto& transform = entity.get<TransformComponent>();
    const auto& current_position = entity.get<PositionComponent>();
    const auto& rotation = entity.get<RotationComponent>();
    const auto& current_velocity = entity.get<VelocityComponent>();
    return std::equal(
               std::begin(transform.value.value),
               std::end(transform.value.value),
               std::begin(expected.transforms[row].value.value)) &&
        current_position.value.x == expected.positions[row].value.x &&
        current_position.value.y == expected.positions[row].value.y &&
        current_position.value.z == expected.positions[row].value.z &&
        rotation.value.x == expected.rotations[row].value.x &&
        rotation.value.y == expected.rotations[row].value.y &&
        rotation.value.z == expected.rotations[row].value.z &&
        current_velocity.value.x == expected.velocities[row].value.x &&
        current_velocity.value.y == expected.velocities[row].value.y &&
        current_velocity.value.z == expected.velocities[row].value.z;
}

struct SimpleContext {
    flecs::world world;
    flecs::query<PositionComponent, const VelocityComponent> query;

    explicit SimpleContext(std::size_t count) {
        register_components(world);
        for (std::size_t index = 0; index < count; ++index) {
            spawn_full(world);
        }
        query = cached(world.query_builder<PositionComponent, const VelocityComponent>());
    }
};

std::uint64_t run_simple(SimpleContext& context, std::size_t repetitions) {
    for (std::size_t repeat = 0; repeat < repetitions; ++repeat) {
        context.query.each([](PositionComponent& current, const VelocityComponent& speed) {
            current.value.x += speed.value.x;
            current.value.y += speed.value.y;
            current.value.z += speed.value.z;
        });
    }
    return static_cast<std::uint64_t>(repetitions);
}

template<typename Tag>
void add_fragment(flecs::world& world) {
    world.component<Tag>();
    for (std::size_t index = 0; index < FRAGMENTED_ENTITIES_PER_VARIANT; ++index) {
        world.entity().set<Tag>({0.0f}).template set<DataComponent>({1.0f});
    }
}

struct FragmentedContext {
    flecs::world world;
    flecs::query<DataComponent> query;

    FragmentedContext() {
        register_components(world);
        add_fragment<A>(world); add_fragment<B>(world); add_fragment<C>(world);
        add_fragment<D>(world); add_fragment<E>(world); add_fragment<F>(world);
        add_fragment<G>(world); add_fragment<H>(world); add_fragment<I>(world);
        add_fragment<J>(world); add_fragment<K>(world); add_fragment<L>(world);
        add_fragment<M>(world); add_fragment<N>(world); add_fragment<O>(world);
        add_fragment<P>(world); add_fragment<Q>(world); add_fragment<R>(world);
        add_fragment<S>(world); add_fragment<T>(world); add_fragment<U>(world);
        add_fragment<V>(world); add_fragment<W>(world); add_fragment<X>(world);
        add_fragment<Y>(world); add_fragment<Z>(world);
        query = cached(world.query_builder<DataComponent>());
    }
};

std::uint64_t run_fragmented(FragmentedContext& context) {
    context.query.run([&](flecs::iter& iterator) {
        while (iterator.next()) {
            auto values = iterator.field<DataComponent>(0);
            for (auto index : iterator) {
                values[index].value = -values[index].value;
            }
        }
    });
    return 1;
}

std::uint16_t next_fragment_mask(std::uint64_t& state) {
    state += 0x9E3779B97F4A7C15ULL;
    std::uint64_t value = state;
    value = (value ^ (value >> 30)) * 0xBF58476D1CE4E5B9ULL;
    value = (value ^ (value >> 27)) * 0x94D049BB133111EBULL;
    return static_cast<std::uint16_t>(value ^ (value >> 31));
}

struct RandomFragmentedContext {
    flecs::world world;
    flecs::query<const A, const B, const C, const D> query;
    std::size_t expected = 0;
    std::uint64_t checksum = 0;

    RandomFragmentedContext(std::size_t component_count, std::size_t entity_count) {
        const std::uint16_t active_mask = component_count == 16
            ? UINT16_MAX
            : static_cast<std::uint16_t>((std::uint32_t{1} << component_count) - 1);
        std::uint64_t state = 0x243F6A8885A308D3ULL;
        for (std::size_t index = 0; index < entity_count; ++index) {
            const std::uint16_t mask = next_fragment_mask(state) & active_mask;
            if ((mask & 0x0Fu) == 0x0Fu) {
                ++expected;
            }
            auto entity = world.entity();
#define SET_RANDOM_FRAGMENT(bit, component) \
            if ((mask & (std::uint16_t{1} << bit)) != 0) { entity.set<component>({10.0f}); }
            SET_RANDOM_FRAGMENT(0, A); SET_RANDOM_FRAGMENT(1, B);
            SET_RANDOM_FRAGMENT(2, C); SET_RANDOM_FRAGMENT(3, D);
            SET_RANDOM_FRAGMENT(4, E); SET_RANDOM_FRAGMENT(5, F);
            SET_RANDOM_FRAGMENT(6, G); SET_RANDOM_FRAGMENT(7, H);
            SET_RANDOM_FRAGMENT(8, I); SET_RANDOM_FRAGMENT(9, J);
            SET_RANDOM_FRAGMENT(10, K); SET_RANDOM_FRAGMENT(11, L);
            SET_RANDOM_FRAGMENT(12, M); SET_RANDOM_FRAGMENT(13, N);
            SET_RANDOM_FRAGMENT(14, O); SET_RANDOM_FRAGMENT(15, P);
#undef SET_RANDOM_FRAGMENT
        }
        query = cached(world.query_builder<const A, const B, const C, const D>());
    }
};

std::uint64_t run_random_fragmented(RandomFragmentedContext& context) {
    context.query.each([&](flecs::entity entity, const A& a, const B& b, const C& c, const D& d) {
        context.checksum += entity.id();
        context.checksum += static_cast<std::uint64_t>(a.value);
        context.checksum += static_cast<std::uint64_t>(b.value);
        context.checksum += static_cast<std::uint64_t>(c.value);
        context.checksum += static_cast<std::uint64_t>(d.value);
    });
    return context.checksum;
}

struct HeavyContext {
    flecs::world world;
    flecs::query<PositionComponent, TransformComponent> query;

    HeavyContext() {
        register_components(world);
        for (std::size_t index = 0; index < HEAVY_ENTITY_COUNT; ++index) {
            spawn_full(world, true);
        }
        query = cached(world.query_builder<PositionComponent, TransformComponent>());
    }
};

std::uint64_t run_heavy(HeavyContext& context, std::size_t repetitions) {
    context.query.run([&](flecs::iter& iterator) {
        while (iterator.next()) {
            auto positions = iterator.field<PositionComponent>(0);
            auto transforms = iterator.field<TransformComponent>(1);
            for (auto row : iterator) {
                Mat4 matrix{};
                for (std::size_t index = 0; index < repetitions; ++index) {
                    invert_opaque(transforms[row].value, matrix);
                }
                positions[row].value = transform_vector(matrix, positions[row].value);
            }
        }
    });
    return 1;
}

template<typename T>
void shuffle(std::vector<T>& values, std::uint64_t state) {
    for (std::size_t index = values.size(); index > 1; --index) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        const std::size_t target = static_cast<std::size_t>(state % index);
        std::swap(values[index - 1], values[target]);
    }
}

struct RandomContext {
    flecs::world world;
    std::vector<flecs::ref<PositionComponent>> references;
    std::vector<std::vector<std::size_t>> orders;
    std::size_t next_order = 0;

    explicit RandomContext(std::size_t count) {
        register_components(world);
        std::vector<flecs::entity_t> ids;
        ids.reserve(count);
        references.reserve(count);
        for (std::size_t index = 0; index < count; ++index) {
            auto entity = world.entity();
            entity.insert([](
                PositionComponent& current_position,
                VelocityComponent& current_velocity) {
                current_position = position();
                current_velocity = velocity();
            });
            ids.push_back(entity.id());
        }
        // Prepare refs only after construction is complete. This matches the
        // other adapters' prepared-access boundary and avoids retaining cache
        // pointers while the target table is still repeatedly reallocating.
        for (flecs::entity_t id : ids) {
            references.push_back(
                world.entity(id).get_ref<PositionComponent>());
        }
        std::vector<std::size_t> indices(count);
        std::iota(indices.begin(), indices.end(), std::size_t{0});
        for (std::uint64_t order = 0; order < 4; ++order) {
            orders.push_back(indices);
            shuffle(orders.back(), 0xDEADBEEFCAFEBABEULL ^
                (order * 0x9E3779B97F4A7C15ULL));
        }
    }
};

std::uint64_t run_random(RandomContext& context) {
    const auto& indices = context.orders[context.next_order++ % context.orders.size()];
    std::uint64_t checksum = 0;
    for (std::size_t index : indices) {
        const PositionComponent* value = context.references[index].get();
        if (!value) {
            std::abort();
        }
        std::uint32_t bits = 0;
        std::memcpy(&bits, &value->value.x, sizeof(bits));
        checksum += bits;
    }
    return checksum;
}

struct EntityOpsContext {
    flecs::world world;
    LightTarget target;
    std::vector<flecs::entity_t> ids;

    EntityOpsContext() : target(world) {
        register_components(world);
        ids.reserve(ENTITY_OP_COUNT);
    }
};

std::uint64_t spawn_despawn(EntityOpsContext& context) {
    context.ids.clear();
    for (std::size_t index = 0; index < ENTITY_OP_COUNT; ++index) {
        context.ids.push_back(context.target.spawn(context.world));
    }
    const std::uint64_t last = context.ids.back();
    for (flecs::entity_t id : context.ids) {
        ecs_delete(context.world.c_ptr(), id);
    }
    return last;
}

struct AddRemoveContext : EntityOpsContext {
    AddRemoveContext() {
        for (std::size_t index = 0; index < ENTITY_OP_COUNT; ++index) {
            spawn_light(world, &ids);
        }
    }
};

void add_health(AddRemoveContext& context) {
    for (flecs::entity_t id : context.ids) {
        context.world.entity(id).set<Health>({100.0f});
    }
}

void remove_health(AddRemoveContext& context) {
    for (flecs::entity_t id : context.ids) {
        context.world.entity(id).remove<Health>();
    }
}

std::uint64_t add_remove(AddRemoveContext& context) {
    add_health(context);
    remove_health(context);
    return context.ids.size();
}

std::vector<flecs::entity_t> sample_entities(const std::vector<flecs::entity_t>& ids) {
    std::vector<flecs::entity_t> sampled;
    sampled.reserve(MIXED_FRAME_RANDOM_COUNT);
    for (std::size_t index = 0; index < MIXED_FRAME_RANDOM_COUNT; ++index) {
        sampled.push_back(ids[index * ids.size() / MIXED_FRAME_RANDOM_COUNT]);
    }
    shuffle(sampled, 0xDEADBEEFCAFEBABEULL);
    return sampled;
}

struct MixedContext {
    flecs::world world;
    LightTarget light_target;
    std::vector<flecs::ref<PositionComponent>> random_references;
    std::vector<flecs::entity_t> churn_ids;
    std::vector<flecs::entity_t> spawned_ids;
    flecs::query<PositionComponent, const VelocityComponent> move_query;
    flecs::query<Health, const Damage> enemy_query;
    flecs::query<Health, const Regen> ally_query;
    flecs::query<PositionComponent, const TransformComponent> heavy_query;

    MixedContext() : light_target(world) {
        register_components(world);
        std::vector<flecs::entity_t> all_ids;
        all_ids.reserve(MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES +
            MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY);
        churn_ids.reserve(MIXED_FRAME_CHURN_COUNT);
        spawned_ids.reserve(MIXED_FRAME_SPAWN_COUNT);

        for (std::size_t index = 0; index < MIXED_FRAME_MOVERS; ++index) {
            auto entity = world.entity()
                .set<PositionComponent>(position(0.0f, 1.0f, 0.0f))
                .set<VelocityComponent>(velocity(1.0f, 0.5f, 0.25f));
            all_ids.push_back(entity.id());
            if (churn_ids.size() < MIXED_FRAME_CHURN_COUNT) {
                churn_ids.push_back(entity.id());
            }
        }
        for (std::size_t index = 0; index < MIXED_FRAME_ENEMIES; ++index) {
            auto entity = world.entity()
                .set<PositionComponent>(position(2.0f, 0.0f, 0.0f))
                .set<VelocityComponent>(velocity(0.25f, 1.0f, 0.0f))
                .set<Health>({100.0f}).set<Damage>({0.75f}).add<IsEnemy>();
            all_ids.push_back(entity.id());
        }
        for (std::size_t index = 0; index < MIXED_FRAME_ALLIES; ++index) {
            auto entity = world.entity()
                .set<PositionComponent>(position(-2.0f, 0.0f, 0.0f))
                .set<VelocityComponent>(velocity(0.0f, 0.75f, 0.25f))
                .set<Health>({60.0f}).set<Regen>({0.35f}).add<IsAlly>();
            all_ids.push_back(entity.id());
        }
        for (std::size_t index = 0; index < MIXED_FRAME_HEAVY; ++index) {
            auto entity = world.entity()
                .set<TransformComponent>({heavy_matrix()})
                .set<PositionComponent>(position())
                .set<VelocityComponent>(velocity(0.5f, 0.0f, 0.5f));
            all_ids.push_back(entity.id());
        }

        const auto random_ids = sample_entities(all_ids);
        random_references.reserve(random_ids.size());
        for (flecs::entity_t id : random_ids) {
            random_references.push_back(
                world.entity(id).get_ref<PositionComponent>());
        }
        move_query = cached(world.query_builder<PositionComponent, const VelocityComponent>());
        enemy_query = cached(world.query_builder<Health, const Damage>());
        ally_query = cached(world.query_builder<Health, const Regen>());
        heavy_query = cached(world.query_builder<PositionComponent, const TransformComponent>());
    }
};

std::uint64_t mixed_movement(MixedContext& context) {
    context.move_query.run([&](flecs::iter& iterator) {
        while (iterator.next()) {
            auto positions = iterator.field<PositionComponent>(0);
            auto velocities = iterator.field<const VelocityComponent>(1);
            for (auto index : iterator) {
                positions[index].value.x += velocities[index].value.x;
                positions[index].value.y += velocities[index].value.y;
                positions[index].value.z += velocities[index].value.z;
            }
        }
    });
    return 1;
}

std::uint64_t mixed_health(MixedContext& context, std::size_t repetitions) {
    for (std::size_t repeat = 0; repeat < repetitions; ++repeat) {
        context.enemy_query.run([&](flecs::iter& iterator) {
            while (iterator.next()) {
                auto health = iterator.field<Health>(0);
                auto damage = iterator.field<const Damage>(1);
                for (auto index : iterator) {
                    health[index].value -= damage[index].value;
                }
            }
        });
        context.ally_query.run([&](flecs::iter& iterator) {
            while (iterator.next()) {
                auto health = iterator.field<Health>(0);
                auto regen = iterator.field<const Regen>(1);
                for (auto index : iterator) {
                    health[index].value += regen[index].value;
                }
            }
        });
    }
    return 1;
}

std::uint64_t mixed_heavy(MixedContext& context) {
    context.heavy_query.run([&](flecs::iter& iterator) {
        while (iterator.next()) {
            auto positions = iterator.field<PositionComponent>(0);
            auto transforms = iterator.field<const TransformComponent>(1);
            for (auto row : iterator) {
                Mat4 matrix{};
                for (std::size_t index = 0; index < MIXED_FRAME_INVERT_COUNT; ++index) {
                    invert_opaque(transforms[row].value, matrix);
                }
                positions[row].value = transform_vector(matrix, positions[row].value);
            }
        }
    });
    return 1;
}

std::uint64_t mixed_random(MixedContext& context) {
    std::uint64_t checksum = 0;
    for (auto& reference : context.random_references) {
        const PositionComponent* value = reference.get();
        if (!value) {
            std::abort();
        }
        std::uint32_t bits = 0;
        std::memcpy(&bits, &value->value.x, sizeof(bits));
        checksum += bits;
    }
    return checksum;
}

std::uint64_t mixed_churn(MixedContext& context) {
    for (flecs::entity_t id : context.churn_ids) {
        context.world.entity(id).set<Health>({100.0f});
    }
    for (flecs::entity_t id : context.churn_ids) {
        context.world.entity(id).remove<Health>();
    }
    return context.churn_ids.size();
}

std::uint64_t mixed_spawn(MixedContext& context, std::size_t repetitions) {
    std::uint64_t last = 0;
    for (std::size_t repeat = 0; repeat < repetitions; ++repeat) {
        context.spawned_ids.clear();
        for (std::size_t index = 0; index < MIXED_FRAME_SPAWN_COUNT; ++index) {
            context.spawned_ids.push_back(context.light_target.spawn(context.world));
        }
        last = context.spawned_ids.back();
        for (flecs::entity_t id : context.spawned_ids) {
            ecs_delete(context.world.c_ptr(), id);
        }
    }
    return last;
}

std::uint64_t mixed_frame(MixedContext& context) {
    mixed_movement(context);
    mixed_health(context, 1);
    mixed_heavy(context);
    const std::uint64_t checksum = mixed_random(context);
    mixed_churn(context);
    mixed_spawn(context, 1);
    return checksum;
}

} // namespace

extern "C" {

void* sky_flecs_cpp_insert_new() { return new InsertContext(); }
void sky_flecs_cpp_insert_delete(void* context) { delete static_cast<InsertContext*>(context); }
std::uint64_t sky_flecs_cpp_bulk_insert(void* context) { return bulk_insert(*static_cast<InsertContext*>(context)); }
std::uint64_t sky_flecs_cpp_single_insert(void* pointer) {
    auto& context = *static_cast<InsertContext*>(pointer);
    const auto& input = insert_data();
    return single_insert_raw(
        context, context.target, input, SIMPLE_ENTITY_COUNT);
}

void* sky_flecs_cpp_simple_new(std::size_t count) { return new SimpleContext(count); }
void sky_flecs_cpp_simple_delete(void* context) { delete static_cast<SimpleContext*>(context); }
std::uint64_t sky_flecs_cpp_simple_run(void* context, std::size_t repetitions) {
    return run_simple(*static_cast<SimpleContext*>(context), repetitions);
}

void* sky_flecs_cpp_fragmented_new() { return new FragmentedContext(); }
void sky_flecs_cpp_fragmented_delete(void* context) { delete static_cast<FragmentedContext*>(context); }
std::uint64_t sky_flecs_cpp_fragmented_run(void* context) { return run_fragmented(*static_cast<FragmentedContext*>(context)); }

void* sky_flecs_cpp_random_fragmented_new(std::size_t component_count, std::size_t entity_count) {
    return new RandomFragmentedContext(component_count, entity_count);
}
void sky_flecs_cpp_random_fragmented_delete(void* context) { delete static_cast<RandomFragmentedContext*>(context); }
std::uint64_t sky_flecs_cpp_random_fragmented_run(void* context) {
    return run_random_fragmented(*static_cast<RandomFragmentedContext*>(context));
}
std::uint64_t sky_flecs_cpp_random_fragmented_count(void* context) {
    return static_cast<std::uint64_t>(static_cast<RandomFragmentedContext*>(context)->query.count());
}

void* sky_flecs_cpp_heavy_new() { return new HeavyContext(); }
void sky_flecs_cpp_heavy_delete(void* context) { delete static_cast<HeavyContext*>(context); }
std::uint64_t sky_flecs_cpp_heavy_run(void* context) { return run_heavy(*static_cast<HeavyContext*>(context), HEAVY_INVERT_COUNT); }

void* sky_flecs_cpp_random_new(std::size_t count) { return new RandomContext(count); }
void sky_flecs_cpp_random_delete(void* context) { delete static_cast<RandomContext*>(context); }
std::uint64_t sky_flecs_cpp_random_run(void* context) { return run_random(*static_cast<RandomContext*>(context)); }

void* sky_flecs_cpp_entity_ops_new() { return new EntityOpsContext(); }
void sky_flecs_cpp_entity_ops_delete(void* context) { delete static_cast<EntityOpsContext*>(context); }
std::uint64_t sky_flecs_cpp_spawn_despawn(void* context) { return spawn_despawn(*static_cast<EntityOpsContext*>(context)); }

void* sky_flecs_cpp_add_remove_new() { return new AddRemoveContext(); }
void sky_flecs_cpp_add_remove_delete(void* context) { delete static_cast<AddRemoveContext*>(context); }
std::uint64_t sky_flecs_cpp_add_remove(void* context) { return add_remove(*static_cast<AddRemoveContext*>(context)); }

void* sky_flecs_cpp_mixed_new() { return new MixedContext(); }
void sky_flecs_cpp_mixed_delete(void* context) { delete static_cast<MixedContext*>(context); }
std::uint64_t sky_flecs_cpp_mixed_frame(void* context) { return mixed_frame(*static_cast<MixedContext*>(context)); }
std::uint64_t sky_flecs_cpp_mixed_movement(void* context) { return mixed_movement(*static_cast<MixedContext*>(context)); }
std::uint64_t sky_flecs_cpp_mixed_health(void* context) { return mixed_health(*static_cast<MixedContext*>(context), MIXED_PHASE_HEALTH_REPEAT); }
std::uint64_t sky_flecs_cpp_mixed_heavy(void* context) { return mixed_heavy(*static_cast<MixedContext*>(context)); }
std::uint64_t sky_flecs_cpp_mixed_random(void* context) { return mixed_random(*static_cast<MixedContext*>(context)); }
std::uint64_t sky_flecs_cpp_mixed_churn(void* context) { return mixed_churn(*static_cast<MixedContext*>(context)); }
std::uint64_t sky_flecs_cpp_mixed_spawn(void* context) { return mixed_spawn(*static_cast<MixedContext*>(context), MIXED_PHASE_SPAWN_REPEAT); }

bool sky_flecs_cpp_validate() {
    InsertContext bulk;
    const auto last_bulk_entity = bulk_insert(bulk);
    if (last_bulk_entity == 0 || !bulk.world.is_alive(last_bulk_entity)) { return false; }
    if (!validate_suite_values(bulk.world, SIMPLE_ENTITY_COUNT)) { return false; }

    InsertContext mapped_bulk;
    InsertData distinct_bulk(CONTRACT_ENTITY_COUNT);
    make_distinct_insert_data(distinct_bulk);
    const ecs_entity_t* mapped_entities = bulk_insert_raw(
        mapped_bulk, mapped_bulk.target, distinct_bulk, CONTRACT_ENTITY_COUNT);
    if (!mapped_entities) { return false; }
    std::vector<flecs::entity_t> mapped_ids(
        mapped_entities, mapped_entities + CONTRACT_ENTITY_COUNT);
    for (std::size_t row = 0; row < mapped_ids.size(); ++row) {
        if (!validate_distinct_entity(
                mapped_bulk.world, mapped_ids[row], distinct_bulk, row)) {
            return false;
        }
    }

    InsertContext single;
    InsertData single_input(CONTRACT_ENTITY_COUNT);
    make_distinct_insert_data(single_input);
    std::vector<flecs::entity_t> single_ids;
    single_ids.reserve(CONTRACT_ENTITY_COUNT);
    if (single_insert_raw(
            single,
            single.target,
            single_input,
            CONTRACT_ENTITY_COUNT,
            &single_ids) == 0) {
        return false;
    }
    for (std::size_t row = 0; row < single_ids.size(); ++row) {
        if (!validate_distinct_entity(single.world, single_ids[row], single_input, row)) {
            return false;
        }
    }

    SimpleContext simple(128);
    if (run_simple(simple, 1) != 1) { return false; }
    std::size_t matched = 0;
    float checksum = 0.0f;
    simple.query.each([&](PositionComponent& current, const VelocityComponent&) {
        ++matched;
        checksum += current.value.x;
    });
    if (matched != 128 || checksum != 256.0f) { return false; }

    auto entity = simple.world.entity().set<PositionComponent>(position()).set<VelocityComponent>(velocity());
    const flecs::entity_t id = entity.id();
    entity.set<Health>({100.0f});
    if (!entity.has<Health>()) { return false; }
    entity.remove<Health>();
    if (entity.has<Health>()) { return false; }
    entity.destruct();
    if (simple.world.is_alive(id)) { return false; }

    FragmentedContext fragmented;
    run_fragmented(fragmented);
    std::size_t fragmented_count = 0;
    float fragmented_sum = 0.0f;
    fragmented.query.each([&](DataComponent& value) {
        ++fragmented_count;
        fragmented_sum += value.value;
    });
    const auto expected_fragmented_count = 26 * FRAGMENTED_ENTITIES_PER_VARIANT;
    if (fragmented_count != expected_fragmented_count ||
        fragmented_sum != -static_cast<float>(expected_fragmented_count)) {
        return false;
    }

    for (std::size_t component_count : {std::size_t{6}, std::size_t{8}, std::size_t{10}, std::size_t{16}}) {
        RandomFragmentedContext random_fragmented(
            component_count,
            CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT);
        std::size_t random_matched = 0;
        float random_values = 0.0f;
        std::uint64_t random_checksum = 0;
        random_fragmented.query.each([&](
            flecs::entity entity,
            const A& a,
            const B& b,
            const C& c,
            const D& d) {
            ++random_matched;
            random_values += a.value + b.value + c.value + d.value;
            random_checksum += entity.id();
            random_checksum += static_cast<std::uint64_t>(a.value);
            random_checksum += static_cast<std::uint64_t>(b.value);
            random_checksum += static_cast<std::uint64_t>(c.value);
            random_checksum += static_cast<std::uint64_t>(d.value);
        });
        if (random_matched != random_fragmented.expected ||
            random_values != static_cast<float>(random_matched) * 40.0f) {
            return false;
        }
        if (run_random_fragmented(random_fragmented) != random_checksum) {
            return false;
        }
    }

    Mat4 inverse{};
    if (!invert(heavy_matrix(), inverse)) { return false; }
    const Vec3 input{1.0f, 2.0f, 3.0f};
    const Vec3 output = transform_vector(inverse, input);
    const float cosine = std::cos(1.2f);
    const float sine = std::sin(1.2f);
    if (std::fabs(output.x - 1.0f) > 1.0e-5f ||
        std::fabs(output.y - (2.0f * cosine + 3.0f * sine)) > 1.0e-5f ||
        std::fabs(output.z - (-2.0f * sine + 3.0f * cosine)) > 1.0e-5f) {
        return false;
    }
    HeavyContext heavy;
    if (run_heavy(heavy, 2) != 1) { return false; }
    std::size_t heavy_count = 0;
    bool heavy_values_valid = true;
    heavy.query.each([&](PositionComponent& current, TransformComponent&) {
        ++heavy_count;
        heavy_values_valid = heavy_values_valid &&
            std::isfinite(current.value.x) &&
            std::isfinite(current.value.y) &&
            std::isfinite(current.value.z) &&
            std::fabs(current.value.x - 1.0f) <= 1.0e-5f &&
            std::fabs(current.value.y) <= 1.0e-5f &&
            std::fabs(current.value.z) <= 1.0e-5f;
    });
    if (heavy_count != HEAVY_ENTITY_COUNT || !heavy_values_valid) {
        return false;
    }

    RandomContext random(CONTRACT_ENTITY_COUNT);
    if (run_random(random) != static_cast<std::uint64_t>(CONTRACT_ENTITY_COUNT) * 0x3F800000ULL) {
        return false;
    }

    EntityOpsContext entity_ops;
    const auto target_probe = entity_ops.target.spawn(entity_ops.world);
    const auto target_probe_view = entity_ops.world.entity(target_probe);
    if (!target_probe_view.has<PositionComponent>() ||
        !target_probe_view.has<VelocityComponent>()) {
        return false;
    }
    ecs_delete(entity_ops.world.c_ptr(), target_probe);
    spawn_despawn(entity_ops);
    if (entity_ops.ids.size() != ENTITY_OP_COUNT) { return false; }
    for (flecs::entity_t old_id : entity_ops.ids) {
        if (entity_ops.world.is_alive(old_id)) { return false; }
    }
    if (uncached(entity_ops.world.query_builder<const PositionComponent>()).count() != 0) {
        return false;
    }

    AddRemoveContext add_remove_context;
    add_health(add_remove_context);
    for (flecs::entity_t add_remove_id : add_remove_context.ids) {
        auto current = add_remove_context.world.entity(add_remove_id);
        if (!current.is_alive() ||
            !current.has<PositionComponent>() ||
            !current.has<VelocityComponent>() ||
            !current.has<Health>() ||
            current.get<Health>().value != 100.0f) {
            return false;
        }
    }
    remove_health(add_remove_context);
    for (flecs::entity_t add_remove_id : add_remove_context.ids) {
        auto current = add_remove_context.world.entity(add_remove_id);
        if (!current.is_alive() || current.has<Health>()) {
            return false;
        }
    }

    MixedContext mixed;
    const auto before = uncached(mixed.world.query_builder<const PositionComponent>()).count();
    for (flecs::entity_t churn_id : mixed.churn_ids) {
        mixed.world.entity(churn_id).set<Health>({100.0f});
        if (!mixed.world.entity(churn_id).has<Health>()) { return false; }
    }
    for (flecs::entity_t churn_id : mixed.churn_ids) {
        mixed.world.entity(churn_id).remove<Health>();
    }
    if (mixed_frame(mixed) == 0) { return false; }
    const auto after = uncached(mixed.world.query_builder<const PositionComponent>()).count();
    if (before != after || !mixed.spawned_ids.size() ||
        mixed.world.entity(mixed.churn_ids.front()).has<Health>()) {
        return false;
    }
    for (flecs::entity_t old_id : mixed.spawned_ids) {
        if (mixed.world.is_alive(old_id)) { return false; }
    }
    if (mixed_random(mixed) == 0) { return false; }

    std::size_t position_count = 0;
    float position_sum = 0.0f;
    uncached(mixed.world.query_builder<const PositionComponent>()).each(
        [&](const PositionComponent& current) {
            ++position_count;
            position_sum += current.value.x;
        });
    if (position_count != MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES +
            MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY ||
        std::fabs(position_sum - 18'500.0f) > 0.2f) {
        return false;
    }

    std::size_t health_count = 0;
    float health_sum = 0.0f;
    uncached(mixed.world.query_builder<const Health>()).each([&](const Health& health) {
        ++health_count;
        health_sum += health.value;
    });
    return health_count == MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES &&
        std::fabs(health_sum - 638'400.0f) <= 64.0f;
}

} // extern "C"
