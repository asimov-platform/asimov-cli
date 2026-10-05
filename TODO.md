# Enhancement backlog

Review baseline: 2026-10-04. Scope: the Rust library and binary, all stable and
experimental handlers, build configuration, dependencies, tests, proxy
templates, and local CI/release workflows. P1 = data integrity/confidentiality;
P2 = runtime correctness/reliability; P3 = maintenance, portability, and
additional coverage.
Items marked "reproduced" were checked with isolated synthetic fixtures.

## Verification baseline

- Rust/Cargo 1.98.1 on aarch64 macOS.
- `cargo fmt --all -- --check` and `cargo check --locked`: pass.
- `cargo test --locked`: 109 tests pass; one doctest is ignored.
- `cargo test --locked --all-features`: 112 tests pass; one doctest is ignored.
- `cargo +1.97.1 check --locked`: pass on the declared minimum toolchain.
- `cargo check --locked --all-features`: pass.
- `cargo check --locked --no-default-features`: passes without warnings.
  Individual `module`, `proxy`, `source`, `telemetry`, and `source-snap` feature
  checks also pass without warnings.
- `cargo test --locked --no-default-features`: 32 tests pass.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: passes.
- `cargo doc --locked --no-deps`: passes without warnings, including with
  `--all-features`.

## P1: Data integrity and confidentiality

- [ ] Constrain config reads/writes to the intended directory
  (`src/commands/module/config.rs::set_permissions` and
  `src/commands/module/config/{get,set,setup}.rs`). Permission repair skips
  symlinks, while reads follow them and symlinked config directories can redirect
  writes outside the config tree.
  Define and enforce symlink handling for directories and files, including
  replacement races, and test that external targets remain untouched.
  SDK HEAD `713cff00` confines manifest/config reads, but CLI-local stored reads,
  provenance resolution, and writes still bypass that confinement. Align these
  paths with the SDK's directory-relative access policy.

## P2: CLI behavior and process lifecycle

- [ ] Bound and correctly reap help subprocesses
  (`src/commands/{help,help_cmd}.rs`). Collection waits for exit before draining
  pipes, uses one deadline for the entire parallel batch, busy-polls, and kills
  without waiting; explicit external help has no timeout. Drain stdout/stderr
  concurrently, bound output, apply per-child deadlines, and always reap
  children. Exercise large help output, hangs, failed spawns, and termination.

- [ ] Preserve external command exit statuses
  (`src/main.rs::Command::External`, `src/commands/external.rs`). An external
  exit status of 42 becomes `EX_SOFTWARE` (reproduced), and signal statuses are
  similarly forced into `SysexitsError`. Carry raw process exit status across
  this boundary. Test normal failures and Unix signals.

- [ ] Handle broken pipes deliberately in command output paths. `println!` on
  a closed pipe panics with exit 101 (reproduced). Introduce fallible output
  handling, preserving existing explicit sysexits, and add operation/path
  context to filesystem errors at their origin.

- [ ] Apply standard color and verbosity options to remaining module and
  snapshot handlers (`src/commands/module/`, `src/commands/source/snap/`).
  Reuse shared color-aware rendering across stdout/stderr and test redirected
  output as well as help output.

## P2: Module configuration and source execution

- [ ] Test module installation/upgrade against changing release metadata using
  an injectable installer (`src/commands/module/{install,upgrade}.rs`).

- [ ] Bound source subprocess concurrency and make cancellation explicit
  (`src/commands/source/{fetch,list}.rs`). All URL tasks are spawned at once and
  drained in input order; early jq, HTTP, or stdout errors drop remaining join
  handles. Use a bounded, cancellation-aware runner group and verify SDK child
  cleanup on early return. Add fake-runner tests for delayed output, ordering,
  debug/cache/deadline forwarding, and inherited reader I/O.

- [ ] Improve Jev transport errors (`src/shared.rs::post_typesafe`). Expose
  useful, credential-safe status/body-generation errors, and test HTTP failures
  and upload cancellation against an injectable local endpoint.

## P2: Proxy reliability and interoperability

- [ ] Bound graceful proxy shutdown waits for stalled streamed responses
  (`src/commands/proxy/serve.rs`) while allowing active responses to drain.

- [ ] Use the validated proxy endpoint in generated and installed templates
  (`src/commands/proxy/config/`). Templates hardcode port 1920 despite
  environment overrides. Define wildcard-bind versus client-address semantics
  and test custom ports and IPv4/IPv6 client URLs.

- [ ] Execute generated proxy shell templates on Windows to validate quoting,
  current-session `set` behavior, and persistent `setx` assignments
  (`src/commands/proxy/config.rs`).

## P2: Verification and delivery

- [ ] Exercise default, all-features, and supported reduced-feature builds in
  CI, including the feature-gated module and source integration suites.

- [ ] Strengthen and isolate remaining CLI fixtures (`tests/module/` and
  operational CLI tests). Include stderr in remaining status assertions.
  Isolate cwd, dotenv, telemetry, and keyring state.
  SDK HEAD `713cff00` checks the secret backend even with a cached public key;
  `cargo test --locked` now stops in `external_arguments` when macOS keychain
  access is canceled. Provide an injectable test identity/backend so these
  fixtures do not depend on interactive keychain access.

- [ ] Pin and constrain delegated workflow execution
  (`.github/workflows/{ci,release}.yaml`). Reusable workflows use mutable
  `@master` refs and inherit all secrets; CI runs on `pull_request_target` with
  pull-request write permission. Audit the shared workflow's PR-code execution
  boundary, pin reviewed revisions, and pass only necessary secrets/permissions.
  Add automated updates for pinned Actions dependencies.

## P3: Portability, maintenance, and further coverage

- [ ] Reconcile the feature/no_std baseline (`Cargo.toml`, `src/lib.rs`).
  Defaults are `["all"]`, no `std` feature exists, and the library and its
  dependencies rely on std. Design a real std boundary for portable library
  APIs versus process/network/filesystem commands, adopt
  `default = ["all", "std"]`, and verify the portable subset on a no_std target.
  Clarify feature behavior: `source-snap` alone exposes no built-in commands.

- [ ] Align minimum-toolchain policy (`Cargo.toml`, CI/release workflows,
  `AGENTS.md`). The manifest/workflows require 1.97.1 while the baseline says
  1.97. The declared 1.97.1 build passes; establish whether 1.97.0 is supported,
  synchronize the requirement, and continuously check the chosen minimum with
  the locked dependency graph as well as current stable Rust.

- [ ] Reduce unused and unnecessarily unconditional dependencies
  (`Cargo.toml`, `build.rs`). Audit `cc`, `iroh-base`, `secrecy`, `whoami`,
  optional `mime`, and generated shadow metadata, which have no corresponding
  source consumers. Gate experimental SDK dependencies and narrow broad
  Tokio/default features where practical. Measure clean-build time and binary
  size, checking feature unification before removing dependencies.

- [ ] Retire or explicitly deprecate the legacy registry API
  (`src/registry.rs`, `src/registry/`). Public `fetch_modules` always returns an
  empty list and `is_enabled` always returns true, while active CLI handlers use
  the SDK registry. Migrate consumers before removal. If retained, repair HTTP
  status handling, hardcoded package versions/endpoints, dependency-kind and
  requirement parsing, and the use of parent-package versions for modules.

- [ ] Establish readiness contracts before wiring experimental commands
  (`src/commands/unstable.rs`, `src/commands/unstable/`). The root enum is
  empty; several handlers return success without doing anything, and checks
  report missing required files but still succeed. Before exposing each group,
  replace stubs with explicit failures and add dispatch/exit-status tests.
  Protocol follow-ups include validating tickets without panics, honoring
  ignored `--ticket` values, observing subscriber task errors, bounded peer
  waits, and guaranteed node termination on errors.

- [ ] Add deterministic snapshot lifecycle coverage
  (`src/commands/source/snap/`, `src/timestamps.rs`). Exercise save/list/log/
  compact with temporary storage, normalized URLs, corruption, and failures.
  Cover multi-day DST differences and fallible timestamp errors. Consolidate
  the unused parallel `create` implementation with the dispatched `save` path.

- [ ] Test telemetry's metadata and lifecycle contracts
  (`src/shared/telemetry{,_disabled}.rs`, `src/commands/configure.rs`,
  `src/main.rs`). Use a fake/local sink to cover aliases/default subcommands,
  external-command redaction, module-name deduplication, failures, cancellation,
  and opt-out without flushing queued events first. Assert that URLs, arbitrary
  arguments, config values, and secrets never enter serialized events.

- [ ] Document public error/output contracts (`src/shared.rs`, `src/commands/`,
  public library APIs). Keep strict Clippy, rustdoc, and supported
  reduced-feature builds warning-free as commands evolve.

- [ ] Repair generated command examples and reference snippets
  (`.config/readmer/README.md.liquid`, `Rakefile`, `etc/readmer/`, `Makefile`).
  Examples still use the absent top-level `asimov import` alias and describe
  module management as a separate installation. Rake's help capture is commented
  out, leaving reference snippets with only command prompts. Generate real,
  deterministic help for supported commands, track snippet dependencies, and
  regenerate the README from its template.
