pub mod dotfiles;
pub mod profile;

pub use profile::{
    create_profile, get_active_profile, get_profile, list_profiles, set_active_profile,
    switch_profile, Profile,
};
