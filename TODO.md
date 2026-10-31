# Enhancement backlog

Review baseline: 2026-10-04. Scope: the Rust library and binary, all stable and
experimental handlers, build configuration, dependencies, tests, proxy
templates, and local CI/release workflows. P1 = data integrity/confidentiality;
P2 = runtime correctness/reliability; P3 = maintenance, portability, and
additional coverage.
Items marked "reproduced" were checked with isolated synthetic fixtures.

## Verification baseline

- Rust/Cargo 1.99.1 on aarch64 macOS; no test environment variables needed.
- `cargo fmt --all -- --check` and `cargo check --locked`: pass.
- `cargo test --locked` (2026-10-30): 126 tests pass; 145 with
  `--all-features`, 34 with `--no-default-features`.
- `cargo check --locked --all-features`: pass.
- `cargo check --locked --no-default-features`: passes without warnings.
- `cargo test --locked --no-default-features`: 32 tests pass.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: passes.
- `cargo doc --locked --no-deps`: passes without warnings, including with
  `--all-features`.

## P2: CLI behavior and process lifecycle

- [ ] Make argument-file expansion aware of literal option values.
  `clientele::args_os()` expands all `@...` arguments before Clap, so
  `--json '@id,name'` tries to read an argument file. The documented equals form
  (`--json=@id,name`) works. Coordinate an option-aware upstream API, preserving
  wildcard expansion, recursive-include detection, and external arguments.

- [ ] Bound explicit external help (`src/commands/help_cmd.rs`). `asimov help
  <cmd>` waits indefinitely and buffers all output. Decide whether a timeout
  fits there, or stream the child's help through instead.

- [ ] Handle broken pipes in remaining command output paths. `println!` on a
  closed pipe panics with exit 101 (reproduced). Convert remaining stdout
  writes to `shared::outln!` (done: `module`, `resource`, `remote list`,
  `proxy`, `--version`, `--license`; remaining: `unstable` commands). Add operation/path context to filesystem errors
  at their origin.

- [ ] Apply standard color and verbosity options to remaining module and
  snapshot handlers (`src/commands/module/`, `src/commands/resource/snap/`).
  Reuse shared color-aware rendering across stdout/stderr and test redirected
  output as well as help output.

## P2: Module configuration and source execution

- [ ] Complete `--json` relay rollout. The server must accept the `options.json`
  name array and apply Jev, property selection, then jq; relay output is already
  filtered and is printed directly. Coordinate this with the SDK's relay backlog.

- [ ] Test module installation/upgrade against changing release metadata using
  an injectable installer (`src/commands/module/{install,upgrade}.rs`).

- [ ] Add fake-runner tests for debug/cache/deadline option forwarding in
  `resource {fetch,list}` and for inherited reader I/O in `resource read`.

- [ ] Improve Jev transport errors (`src/shared.rs::post_typesafe`). HTTP
  failures surface as raw reqwest errors with one exit code. Using the
  private `post_json()` against a local endpoint, first test 401, 429, and
  500 responses, malformed JSON, body-generation failure, and cancellation;
  then report each distinctly (401/403: rejected `TYPESAFE_API_TOKEN`,
  `EX_NOPERM` or `EX_CONFIG`; 429/5xx: `EX_UNAVAILABLE`; malformed:
  `EX_PROTOCOL`), never including the token or request body.

- [ ] Fix `module install all`, which installs nothing: the upstream `all`
  manifest (asimov-modules `all/.asimov/module.yaml`) declares
  `requires.modules: []`. Populate it or enumerate modules another way.

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

- [ ] Replace the SDK git pin (`[patch.crates-io]` in `Cargo.toml`, at
  64c3861 for symlinked config directories and
  `OriginProxy::client_builder()`) with the next SDK release before
  publishing the CLI; published crates ignore patches.

- [ ] Exercise default, all-features, and supported reduced-feature builds in
  CI, including the feature-gated module and resource integration suites.

- [ ] Include stderr in remaining CLI fixture status assertions (e.g.,
  `tests/module/install.rs`). Unix fixtures are isolated (cwd, `HOME`,
  file keyring, `DO_NOT_TRACK`); Windows fixtures still use `USERPROFILE`
  and the native keyring.

- [ ] Finish constraining delegated workflow execution. `ci.yaml` now pins
  `ci-rust.yaml` and passes only the review job's app secrets, but that
  pinned revision still calls `ci-rust-job.yaml@master` and
  `set-env-vars@master`, which build and test fork PR heads under
  `pull_request_target` after exporting the `ENV` secret. Fix that in
  asimov-platform/actions (deferred): pass `ENV` to `ci-rust-job.yaml` only
  for pushes and same-repository PRs; pin `ci-rust-job.yaml`,
  `set-env-vars`, `dtolnay/rust-toolchain`, and `actions/checkout` to SHAs;
  set `persist-credentials: false` on PR-head checkouts. Then move the
  `ci.yaml` pin to the fixed revision.
  `release.yaml` is likewise pinned with explicit secrets. Add automated
  updates for pinned Actions dependencies.

## P3: Portability, maintenance, and further coverage

- [ ] Implement tracked-resource search in `src/commands/resource/search.rs`;
  it currently returns `EX_UNAVAILABLE` and is gated behind `unstable`.

- [ ] Add Iroh remote URL support when the corresponding SDK transport is
  available.

- [ ] Reconcile the feature/no_std baseline (`Cargo.toml`, `src/lib.rs`).
  Defaults are `["all"]`, no `std` feature exists, and the library and its
  dependencies rely on std. Design a real std boundary for portable library
  APIs versus process/network/filesystem commands, adopt
  `default = ["all", "std"]`, and verify the portable subset on a no_std target.

- [ ] Check the MSRV and current stable Rust against the locked dependency graph
  in CI.

- [ ] Reduce unused and unnecessarily unconditional dependencies
  (`Cargo.toml`, `build.rs`). Audit `cc`, `iroh-base`, `secrecy`,
  optional `mime`, and generated shadow metadata, which have no corresponding
  source consumers. Gate experimental SDK dependencies and narrow broad
  Tokio/default features where practical. Measure clean-build time and binary
  size, checking feature unification before removing dependencies.

- [ ] Establish readiness contracts before wiring experimental commands
  (`src/commands/unstable.rs`, `src/commands/unstable/`). The root enum is
  empty; several handlers return success without doing anything, and checks
  report missing required files but still succeed. Before exposing each group,
  replace stubs with explicit failures and add dispatch/exit-status tests.
  Protocol follow-ups include validating tickets without panics, honoring
  ignored `--ticket` values, observing subscriber task errors, bounded peer
  waits, and guaranteed node termination on errors.

- [ ] Add deterministic snapshot lifecycle coverage
  (`src/commands/resource/snap/`, `src/timestamps.rs`). Exercise save/list/log/
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
