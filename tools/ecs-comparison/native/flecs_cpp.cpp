#include <flecs.h>

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstring>
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
    auto entity = world.entity().set<PositionComponent>(position()).set<VelocityComponent>(velocity());
    if (ids) {
        ids->push_back(entity.id());
    }
}

template<typename Query>
auto uncached(Query&& builder) {
    return builder.cache_kind(flecs::QueryCacheNone).build();
}

struct InsertContext {
    flecs::world world;
    std::vector<TransformComponent> transforms;
    std::vector<PositionComponent> positions;
    std::vector<RotationComponent> rotations;
    std::vector<VelocityComponent> velocities;

    InsertContext()
        : transforms(SIMPLE_ENTITY_COUNT, {identity_matrix()}),
          positions(SIMPLE_ENTITY_COUNT, position()),
          rotations(SIMPLE_ENTITY_COUNT, RotationComponent{{1.0f, 0.0f, 0.0f}}),
          velocities(SIMPLE_ENTITY_COUNT, velocity()) {
        register_components(world);
    }
};

std::uint64_t bulk_insert(InsertContext& context) {
    ecs_bulk_desc_t descriptor{};
    descriptor.count = static_cast<std::int32_t>(SIMPLE_ENTITY_COUNT);
    descriptor.ids[0] = context.world.component<TransformComponent>().id();
    descriptor.ids[1] = context.world.component<PositionComponent>().id();
    descriptor.ids[2] = context.world.component<RotationComponent>().id();
    descriptor.ids[3] = context.world.component<VelocityComponent>().id();
    void* data[] = {
        context.transforms.data(), context.positions.data(),
        context.rotations.data(), context.velocities.data(),
    };
    descriptor.data = data;
    const ecs_entity_t* entities = ecs_bulk_init(context.world.c_ptr(), &descriptor);
    return entities ? entities[SIMPLE_ENTITY_COUNT - 1] : 0;
}

struct SimpleContext {
    flecs::world world;
    flecs::query<PositionComponent, const VelocityComponent> query;

    explicit SimpleContext(std::size_t count) {
        register_components(world);
        for (std::size_t index = 0; index < count; ++index) {
            spawn_full(world);
        }
        query = uncached(world.query_builder<PositionComponent, const VelocityComponent>());
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
        world.entity().set<Tag>({0.0f}).set<DataComponent>({1.0f});
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
        query = uncached(world.query_builder<DataComponent>());
    }
};

std::uint64_t run_fragmented(FragmentedContext& context) {
    std::uint64_t count = 0;
    context.query.run([&](flecs::iter& iterator) {
        while (iterator.next()) {
            auto values = iterator.field<DataComponent>(0);
            for (auto index : iterator) {
                values[index].value *= 2.0f;
                ++count;
            }
        }
    });
    return count;
}

struct HeavyContext {
    flecs::world world;
    flecs::query<PositionComponent, TransformComponent> query;

    HeavyContext() {
        register_components(world);
        for (std::size_t index = 0; index < HEAVY_ENTITY_COUNT; ++index) {
            spawn_full(world, true);
        }
        query = uncached(world.query_builder<PositionComponent, TransformComponent>());
    }
};

std::uint64_t run_heavy(HeavyContext& context, std::size_t repetitions) {
    std::uint64_t count = 0;
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
                ++count;
            }
        }
    });
    return count;
}

void shuffle(std::vector<flecs::entity_t>& values, std::uint64_t state) {
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
    std::vector<std::vector<flecs::entity_t>> orders;
    std::size_t next_order = 0;

    explicit RandomContext(std::size_t count) {
        register_components(world);
        std::vector<flecs::entity_t> ids;
        ids.reserve(count);
        for (std::size_t index = 0; index < count; ++index) {
            spawn_light(world, &ids);
        }
        for (std::uint64_t order = 0; order < 4; ++order) {
            orders.push_back(ids);
            shuffle(orders.back(), 0xDEADBEEFCAFEBABEULL ^
                (order * 0x9E3779B97F4A7C15ULL));
        }
    }
};

std::uint64_t run_random(RandomContext& context) {
    const auto& ids = context.orders[context.next_order++ % context.orders.size()];
    float checksum = 0.0f;
    for (flecs::entity_t id : ids) {
        const PositionComponent* value = context.world.entity(id).try_get<PositionComponent>();
        if (value) {
            checksum += value->value.x;
        }
    }
    std::uint32_t bits = 0;
    std::memcpy(&bits, &checksum, sizeof(bits));
    return bits;
}

struct EntityOpsContext {
    flecs::world world;
    std::vector<flecs::entity_t> ids;

    EntityOpsContext() {
        register_components(world);
        ids.reserve(ENTITY_OP_COUNT);
    }
};

std::uint64_t spawn_despawn(EntityOpsContext& context) {
    context.ids.clear();
    for (std::size_t index = 0; index < ENTITY_OP_COUNT; ++index) {
        spawn_light(context.world, &context.ids);
    }
    const std::uint64_t last = context.ids.back();
    for (flecs::entity_t id : context.ids) {
        context.world.entity(id).destruct();
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

std::uint64_t add_remove(AddRemoveContext& context) {
    for (flecs::entity_t id : context.ids) {
        context.world.entity(id).set<Health>({100.0f});
    }
    for (flecs::entity_t id : context.ids) {
        context.world.entity(id).remove<Health>();
    }
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
    std::vector<flecs::entity_t> random_ids;
    std::vector<flecs::entity_t> churn_ids;
    std::vector<flecs::entity_t> spawned_ids;
    flecs::query<PositionComponent, const VelocityComponent> move_query;
    flecs::query<Health, const Damage> enemy_query;
    flecs::query<Health, const Regen> ally_query;
    flecs::query<PositionComponent, const TransformComponent> heavy_query;

    MixedContext() {
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

        random_ids = sample_entities(all_ids);
        move_query = uncached(world.query_builder<PositionComponent, const VelocityComponent>());
        enemy_query = uncached(world.query_builder<Health, const Damage>());
        ally_query = uncached(world.query_builder<Health, const Regen>());
        heavy_query = uncached(world.query_builder<PositionComponent, const TransformComponent>());
    }
};

std::uint64_t mixed_movement(MixedContext& context) {
    std::uint64_t count = 0;
    context.move_query.run([&](flecs::iter& iterator) {
        while (iterator.next()) {
            auto positions = iterator.field<PositionComponent>(0);
            auto velocities = iterator.field<const VelocityComponent>(1);
            for (auto index : iterator) {
                positions[index].value.x += velocities[index].value.x;
                positions[index].value.y += velocities[index].value.y;
                positions[index].value.z += velocities[index].value.z;
                ++count;
            }
        }
    });
    return count;
}

std::uint64_t mixed_health(MixedContext& context, std::size_t repetitions) {
    std::uint64_t count = 0;
    for (std::size_t repeat = 0; repeat < repetitions; ++repeat) {
        context.enemy_query.run([&](flecs::iter& iterator) {
            while (iterator.next()) {
                auto health = iterator.field<Health>(0);
                auto damage = iterator.field<const Damage>(1);
                for (auto index : iterator) {
                    health[index].value -= damage[index].value;
                    ++count;
                }
            }
        });
        context.ally_query.run([&](flecs::iter& iterator) {
            while (iterator.next()) {
                auto health = iterator.field<Health>(0);
                auto regen = iterator.field<const Regen>(1);
                for (auto index : iterator) {
                    health[index].value += regen[index].value;
                    ++count;
                }
            }
        });
    }
    return count;
}

std::uint64_t mixed_heavy(MixedContext& context) {
    std::uint64_t count = 0;
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
                ++count;
            }
        }
    });
    return count;
}

std::uint64_t mixed_random(MixedContext& context) {
    float checksum = 0.0f;
    for (flecs::entity_t id : context.random_ids) {
        const auto* value = context.world.entity(id).try_get<PositionComponent>();
        if (value) {
            checksum += value->value.x;
        }
    }
    std::uint32_t bits = 0;
    std::memcpy(&bits, &checksum, sizeof(bits));
    return bits;
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
            spawn_light(context.world, &context.spawned_ids);
        }
        last = context.spawned_ids.back();
        for (flecs::entity_t id : context.spawned_ids) {
            context.world.entity(id).destruct();
        }
    }
    return last;
}

std::uint64_t mixed_frame(MixedContext& context) {
    std::uint64_t checksum = mixed_movement(context);
    checksum += mixed_health(context, 1);
    checksum += mixed_heavy(context);
    checksum += mixed_random(context);
    checksum += mixed_churn(context);
    checksum += mixed_spawn(context, 1);
    return checksum;
}

} // namespace

extern "C" {

void* sky_flecs_cpp_insert_new() { return new InsertContext(); }
void sky_flecs_cpp_insert_delete(void* context) { delete static_cast<InsertContext*>(context); }
std::uint64_t sky_flecs_cpp_bulk_insert(void* context) { return bulk_insert(*static_cast<InsertContext*>(context)); }
std::uint64_t sky_flecs_cpp_single_insert(void* pointer) {
    auto& context = *static_cast<InsertContext*>(pointer);
    for (std::size_t index = 0; index < SIMPLE_ENTITY_COUNT; ++index) { spawn_full(context.world); }
    return SIMPLE_ENTITY_COUNT;
}

void* sky_flecs_cpp_simple_new(std::size_t count) { return new SimpleContext(count); }
void sky_flecs_cpp_simple_delete(void* context) { delete static_cast<SimpleContext*>(context); }
std::uint64_t sky_flecs_cpp_simple_run(void* context, std::size_t repetitions) {
    return run_simple(*static_cast<SimpleContext*>(context), repetitions);
}

void* sky_flecs_cpp_fragmented_new() { return new FragmentedContext(); }
void sky_flecs_cpp_fragmented_delete(void* context) { delete static_cast<FragmentedContext*>(context); }
std::uint64_t sky_flecs_cpp_fragmented_run(void* context) { return run_fragmented(*static_cast<FragmentedContext*>(context)); }

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
    if (bulk_insert(bulk) == 0) { return false; }
    auto count = uncached(bulk.world.query_builder<const PositionComponent>()).count();
    if (count != SIMPLE_ENTITY_COUNT) { return false; }

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
    if (run_fragmented(fragmented) != 26 * FRAGMENTED_ENTITIES_PER_VARIANT) { return false; }

    MixedContext mixed;
    const auto before = uncached(mixed.world.query_builder<const PositionComponent>()).count();
    mixed_frame(mixed);
    const auto after = uncached(mixed.world.query_builder<const PositionComponent>()).count();
    return before == after && !mixed.world.entity(mixed.churn_ids.front()).has<Health>();
}

} // extern "C"
