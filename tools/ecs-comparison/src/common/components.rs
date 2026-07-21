use super::BenchmarkMatrix;
use cgmath::Vector3;

#[derive(Clone, Copy, Default, bevy_ecs::prelude::Component, shipyard::Component)]
#[repr(transparent)]
pub struct TransformComponent(pub BenchmarkMatrix);

#[derive(Clone, Copy, bevy_ecs::prelude::Component, shipyard::Component)]
#[repr(transparent)]
pub struct PositionComponent(pub Vector3<f32>);

#[derive(Clone, Copy, bevy_ecs::prelude::Component, shipyard::Component)]
#[repr(transparent)]
pub struct RotationComponent(pub Vector3<f32>);

#[derive(Clone, Copy, bevy_ecs::prelude::Component, shipyard::Component)]
#[repr(transparent)]
pub struct VelocityComponent(pub Vector3<f32>);

#[derive(Clone, Copy, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct DataComponent(pub f32);

impl Default for PositionComponent {
    fn default() -> Self {
        Self(Vector3::new(0.0, 0.0, 0.0))
    }
}

impl Default for RotationComponent {
    fn default() -> Self {
        Self(Vector3::new(0.0, 0.0, 0.0))
    }
}

impl Default for VelocityComponent {
    fn default() -> Self {
        Self(Vector3::new(0.0, 0.0, 0.0))
    }
}

impl Default for DataComponent {
    fn default() -> Self {
        Self(0.0)
    }
}

#[derive(Clone, Copy, Default, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct Health(pub f32);

#[derive(Clone, Copy, Default, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct Damage(pub f32);

#[derive(Clone, Copy, Default, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct Regen(pub f32);

#[derive(Clone, Copy, Default, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct IsEnemy;

#[derive(Clone, Copy, Default, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct IsAlly;

/// Remaining simulation frames for short-lived gameplay entities.
#[derive(Clone, Copy, Default, bevy_ecs::prelude::Component, shipyard::Component)]
#[repr(transparent)]
pub struct Lifetime(pub u32);

/// Logical slot targeted by an AI entity. Logical slots are stable when the
/// underlying ECS entity handle changes after despawn/respawn.
#[derive(Clone, Copy, Default, bevy_ecs::prelude::Component, shipyard::Component)]
#[repr(transparent)]
pub struct TargetSlot(pub u32);

#[derive(Clone, Copy, Default, bevy_ecs::prelude::Component, shipyard::Component)]
#[repr(transparent)]
pub struct Cooldown(pub u32);

#[derive(Clone, Copy, Default, bevy_ecs::prelude::Component, shipyard::Component)]
#[repr(transparent)]
pub struct OwnerSlot(pub u32);

/// A transient status component. It is removed eight frames after insertion,
/// so structural changes never cancel each other in the same frame.
#[derive(Clone, Copy, Default, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct Stunned;

#[derive(Clone, Copy, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct Position2D {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct Velocity2D {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct AuxA {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct AuxB {
    pub x: f32,
    pub y: f32,
}

macro_rules! define_fragment_components {
    ($($name:ident),+ $(,)?) => {
        $(
            #[derive(
                Clone,
                Copy,
                Default,
                bevy_ecs::prelude::Component,
                shipyard::Component,
            )]
            pub struct $name(pub f32);
        )+
    };
}

define_fragment_components!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
);

macro_rules! define_random_fragment_tags {
    ($($name:ident),+ $(,)?) => {
        $(
            #[derive(
                Clone,
                Copy,
                Default,
                bevy_ecs::prelude::Component,
                shipyard::Component,
            )]
            pub struct $name;
        )+
    };
}

define_random_fragment_tags!(
    TagA, TagB, TagC, TagD, TagE, TagF, TagG, TagH, TagI, TagJ, TagK, TagL, TagM, TagN, TagO, TagP,
);

#[cfg(test)]
mod tests {
    use super::{PositionComponent, RotationComponent, TransformComponent, VelocityComponent};

    #[test]
    fn heavy_component_layout_matches_native_adapter() {
        assert_eq!(std::mem::size_of::<TransformComponent>(), 64);
        assert_eq!(std::mem::align_of::<TransformComponent>(), 4);
        for (size, alignment) in [
            (
                std::mem::size_of::<PositionComponent>(),
                std::mem::align_of::<PositionComponent>(),
            ),
            (
                std::mem::size_of::<RotationComponent>(),
                std::mem::align_of::<RotationComponent>(),
            ),
            (
                std::mem::size_of::<VelocityComponent>(),
                std::mem::align_of::<VelocityComponent>(),
            ),
        ] {
            assert_eq!(size, 12);
            assert_eq!(alignment, 4);
        }
    }
}
