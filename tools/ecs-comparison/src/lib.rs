#![allow(clippy::too_many_arguments)]

pub mod bevy;
pub mod common;
pub mod flecs;
pub mod freecs;
pub mod hecs;
pub mod shared;
pub mod shipyard;
pub mod sky;

use std::env;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Engine {
    Sky,
    Hecs,
    Bevy,
    Flecs,
    Freecs,
    Shipyard,
}

impl Engine {
    pub const ALL: [Self; 6] = [
        Self::Sky,
        Self::Hecs,
        Self::Bevy,
        Self::Flecs,
        Self::Freecs,
        Self::Shipyard,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Sky => "sky",
            Self::Hecs => "hecs",
            Self::Bevy => "bevy",
            Self::Flecs => "flecs",
            Self::Freecs => "freecs",
            Self::Shipyard => "shipyard",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|engine| engine.name() == value)
    }
}

pub fn engine_order() -> Vec<Engine> {
    let Ok(value) = env::var("SKY_ECS_ORDER") else {
        return Engine::ALL.to_vec();
    };

    let engines: Vec<_> = value
        .split(',')
        .map(str::trim)
        .map(|name| Engine::parse(name).unwrap_or_else(|| panic!("unknown ECS engine `{name}`")))
        .collect();
    assert_eq!(
        engines.len(),
        Engine::ALL.len(),
        "SKY_ECS_ORDER must list all six engines"
    );
    for engine in Engine::ALL {
        assert_eq!(
            engines
                .iter()
                .filter(|&&candidate| candidate == engine)
                .count(),
            1,
            "SKY_ECS_ORDER must list each engine exactly once"
        );
    }
    engines
}
