pub mod flamedriver;

pub use flamedriver::FlamedriverProfile;

use crate::model::driver::DriverRegistry;

/// The suite's driver registry: THE list of drivers the app supports.
///
/// Single source of truth on purpose. This used to be built inline at three
/// separate call sites, each re-typing the same registration, so an invariant
/// asserted over "the registered drivers" could only ever cover the list the
/// asserting code happened to repeat -- and a driver added later would be
/// covered by none of them (audit F34). Register a new profile HERE and every
/// caller, including the guards, picks it up.
pub fn default_registry() -> DriverRegistry {
    let mut registry = DriverRegistry::new();
    registry.register(Box::new(FlamedriverProfile));
    registry
}
