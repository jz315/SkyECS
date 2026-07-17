use cgmath::{Matrix4, Vector3};

#[derive(Clone, Copy, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct TransformComponent(pub Matrix4<f32>);

#[derive(Clone, Copy, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct PositionComponent(pub Vector3<f32>);

#[derive(Clone, Copy, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct RotationComponent(pub Vector3<f32>);

#[derive(Clone, Copy, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct VelocityComponent(pub Vector3<f32>);

#[derive(Clone, Copy, bevy_ecs::prelude::Component, shipyard::Component)]
pub struct DataComponent(pub f32);

impl Default for TransformComponent {
    fn default() -> Self {
        Self(Matrix4::from_scale(1.0))
    }
}

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
