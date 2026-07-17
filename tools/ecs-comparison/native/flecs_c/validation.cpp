namespace sky_ecs_bench::flecs_c {

namespace construction {
bool validate();
}

namespace iteration {
bool validate_simple();
bool validate_heavy();
}

namespace fragmented_iteration {
bool validate();
}

namespace random_fragmentation {
bool validate();
}

namespace random_access {
bool validate();
}

namespace entity_operations {
bool validate();
}

namespace mixed_frame {
bool validate();
}

} // namespace sky_ecs_bench::flecs_c

extern "C" bool sky_flecs_c_validate() {
    return sky_ecs_bench::flecs_c::construction::validate() &&
        sky_ecs_bench::flecs_c::iteration::validate_simple() &&
        sky_ecs_bench::flecs_c::fragmented_iteration::validate() &&
        sky_ecs_bench::flecs_c::random_fragmentation::validate() &&
        sky_ecs_bench::flecs_c::iteration::validate_heavy() &&
        sky_ecs_bench::flecs_c::random_access::validate() &&
        sky_ecs_bench::flecs_c::entity_operations::validate() &&
        sky_ecs_bench::flecs_c::mixed_frame::validate();
}
