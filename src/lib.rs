// This is free and unencumbered software released into the public domain.

mod subcommands_provider;
pub use subcommands_provider::*;

use clientele::SysexitsError;

pub type Result<T = SysexitsError, E = SysexitsError> = std::result::Result<T, E>;
