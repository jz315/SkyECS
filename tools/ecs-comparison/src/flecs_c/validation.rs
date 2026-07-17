use super::native::*;

pub fn validate_contract() {
    // SAFETY: Validation creates and destroys all native state internally.
    assert!(unsafe { sky_flecs_c_validate() });
}
