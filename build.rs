// This is free and unencumbered software released into the public domain.

use cfg_aliases::cfg_aliases;
use shadow_rs::ShadowBuilder;
use std::collections::BTreeSet;

fn main() {
    // See: https://github.com/katharostech/cfg_aliases
    cfg_aliases! {
        android: { target_os = "android" },
        darwin: { any(
            target_os = "ios",
            target_os = "macos",
            target_os = "tvos",
            target_os = "watchos")
        },
        ios: { target_os = "ios" },
        linux: { target_os = "linux" },
        macos: { target_os = "macos" },
        tvos: { target_os = "tvos" },
        wasm: { target_family = "wasm" },
        watchos: { target_os = "watchos" },
    }

    // See: https://github.com/baoyachi/shadow-rs
    // Omit all nonpublic and/or sensitive information:
    let mut omit = BTreeSet::new();
    omit.insert(shadow_rs::CARGO_TREE);
    omit.insert(shadow_rs::CARGO_MANIFEST_DIR);
    omit.insert(shadow_rs::COMMIT_AUTHOR);
    omit.insert(shadow_rs::COMMIT_EMAIL);
    omit.insert(shadow_rs::GIT_STATUS_FILE);
    ShadowBuilder::builder().deny_const(omit).build().unwrap();
}
