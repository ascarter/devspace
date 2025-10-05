pub mod config;
pub mod environment;
pub mod profile;

pub use environment::{Shell, ShellEnvironment};
pub use profile::{
    create_profile, get_active_profile, get_profile, list_profiles, set_active_profile,
    switch_profile, Profile,
};
