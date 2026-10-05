# Working method
- This project is developed using atomic commits with human review: don't
  suggest humongous changes in one go; instead, interpret the user's request in
  the most narrow way possible and produce a coherent, review-friendly patch
  with a suggested commit message (one line, verb-first, terse, max 72 chars).

# Project files
- Don't ask to examine parent directories, stick to the project directory.
- Keep to an 80-column wordwrap in comments and Markdown files, where feasible.

# `AGENTS.md`
- When updating `AGENTS.md`, keep in mind that the file is meant to especially
  benefit lesser models than yourself, such as GPT-6 Sol, Terra, and Luna; but
  they may also have smaller context windows, so be terse and token-efficient.

# `TODO.md`
- Record review results & enhancement suggestions into a backlog in `TODO.md`.
- This file is meant primarily for agents and can be used as a scratchpad.
- Don't preserve mentions of already fully completed work in the file.

# `CHANGES.md`
- Only changes that affect users matter: no need to mention CI changes, etc.
- Don't include superfluous technical detail; be terse and aim for brevity.

# `README.md`
- Don't update `README.md` casually: beneficial additions require significant
  judgment and discernment, possibly beyond your capabilities. Longer and more
  detailed does *not* in fact equal better, because humans are not LLMs!
- Any documentation you might wish to add to the README likely better belongs
  as inline comments to the modules and/or types in question, where you are
  allowed elaborated length. For example, in Rust code `rustdoc` coverage of
  every public symbol is a worthy goal, but brevity and quality matter.
- The TOC structure of the README is not to be changed without asking approval.
- If you must update e.g. `rust/README.md`, update also the Liquid template it
  is generated from, in the workspace root's `.config/readmer/rust/README.md`.
  Re-generate using `make -B README.md` in that directory, which invokes
  Readmer (https://github.com/artob/readmer).

# Rust code
- Don't ever directly read the contents of `Cargo.lock`, it can very large.
- Our current MSRV is Rust 1.97; update and enforce everywhere as needed.
- Our crates collect their default features under an `all` feature, and
  their `[features]` should start with the line `default = ["all", "std"]`.
- Our crates are meant to always be buildable with `#![no_std]` and hence
  always export an explicit `std` feature flag. Some of our lower-level crates
  may also have an explicit `alloc` feature, but for many crates it's not
  possible to do anything useful without heap allocations and they hence
  implicitly assume and omit such a feature.
- All references to `std`, `alloc`, and `core` types should always use
  qualified names or explicit, least-power imports. For example, prefer
  `core::error::Error` and `alloc::string::String` over `std` analogs.
- After making changes to a crate, as a last step run `cargo doc` on it.

# Project map
- Single Rust 2024 package: `asimov_cli` library and `asimov` binary.
- `src/aliases.rs` rewrites argv before Clap parsing in `src/main.rs`.
  Preserve aliases, help behavior, and external `asimov-*` command dispatch.
- `src/commands/{module,proxy,source}.rs` define command enums and dispatch;
  handlers live in matching directories. Wire new commands through the group,
  `src/commands.rs`, `src/main.rs`, and Cargo feature gates as applicable.
- Handlers under `src/commands/`: `module/` wraps SDK installation/config;
  `source/` wraps SDK runners and snapshots (`source/snap/`); `proxy/` serves
  an OpenAI-compatible endpoint (Axum/Hyper/rustls) and app config templates.
- `src/shared.rs` holds module selection, HTTP, and jq/Jev filtering helpers.
  `src/registry.rs::fetch_modules()` is disabled; active module management
  uses the SDK's `asimov-registry` crate.
- `all = ["module", "proxy", "source"]`; `source` includes `source-snap`.
  `--all-features` also enables incomplete `unstable` commands.
- Root `README.md` comes from `.config/readmer/README.md.liquid`, with snippets
  in `etc/readmer/`. Regenerate with `make -B README.md` at the project root.

# Command conventions
- Reuse `clientele::StandardOptions` and the existing async handler pattern.
  Preserve `SysexitsError` values through `BoxError` and `main.rs::sysexits`.
  Keep machine-readable stdout separate from stderr diagnostics.
- Reuse SDK resolvers/runners and shared module-selection helpers. Drain runner
  output streams, even with inherited stdout, to observe subprocess failures.
  Keep proxy responses streaming.
- Module config precedence is environment, stored value, then default.
  Preserve secret masking except for explicit `config get`, validate variable
  names before file access, and validate whole batches before writing.
  Keep module config directories/files at 0700/0600 on Unix.
- Patch application configs through the existing JSONC/CST editing APIs to
  preserve comments and formatting.

# Verification
- For crate changes, run from the root: `cargo fmt --all -- --check`,
  `cargo check --locked`, and `cargo test --locked`. Finish with
  `cargo doc --locked --no-deps`. Use default features for normal checks;
  also check feature sets you change.
- Targeted tests: `cargo test --locked --lib` or
  `cargo test --locked --test module` (`tests/module/main.rs`).
- CLI test fixtures use `env!("CARGO_BIN_EXE_asimov")` and temporary
  `ASIMOV_ROOT` directories. Startup loads `.env`; operational CLI tests require
  a working OS keyring. `tests/shared.rs` creates fake executables and reruns
  PATH-dependent library checks in children with private environments.

# Baseline alignment to review
- `Cargo.toml` and CI/release workflows currently pin Rust 1.97.1; the baseline
  specifies 1.97. Keep toolchain requirements synchronized when changing them.
- This CLI currently depends on `std`, declares no `std` feature, and uses
  `default = ["all"]`. Review how to reconcile this with the baseline's
  feature/no_std policy; `--no-default-features` alone is not `no_std` support.

# Project repositories
- ASIMOV CLI: <https://github.com/asimov-platform/asimov-cli>
- ASIMOV SDK: <https://github.com/asimov-platform/asimov-sdk>
- ASIMOV Specs: <https://github.com/asimov-specs/asimov-specs>
- ASIMOV Modules: <https://github.com/asimov-modules/asimov-modules>
- Async-Flow: <https://github.com/artob/async-flow>
- Bitcache: <https://github.com/artob/bitcache>
- RDF.rs: <https://github.com/rust-rdf/rdf.rs>
- Readmer: <https://github.com/artob/readmer>
- SPARQL.rs: <https://github.com/rust-rdf/sparql.rs>
