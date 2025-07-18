// This is free and unencumbered software released into the public domain.

/// The set of features that are enabled in this build of the program.
#[allow(unused)]
pub static FEATURES: &[&str] = &[
    #[cfg(feature = "fetch")]
    "fetch",
    #[cfg(feature = "unstable")]
    "unstable",
];
