use sky_ecs_comparison::{bevy, flecs_c, freecs, hecs, shipyard, sky};

#[test]
fn sky_adapter_satisfies_contract() {
    sky::validate_contract();
}

#[test]
fn hecs_adapter_satisfies_contract() {
    hecs::validate_contract();
}

#[test]
fn bevy_adapter_satisfies_contract() {
    bevy::validate_contract();
}

#[test]
fn flecs_c_adapter_satisfies_contract() {
    flecs_c::validate_contract();
}

#[test]
fn freecs_adapter_satisfies_contract() {
    freecs::validate_contract();
}

#[test]
fn shipyard_adapter_satisfies_contract() {
    shipyard::validate_contract();
}
