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
- `cargo test --locked`: 44 tests pass; one doctest is ignored.
- `cargo +1.97.1 check --locked`: pass on the declared minimum toolchain.
- `cargo check --locked --all-features`: pass.
- `cargo check --locked --no-default-features`: pass with warnings. Individual
  `module`, `proxy`, `source`, `telemetry`, and `source-snap` feature checks
  also pass; several report unused imports/functions.
- `cargo test --locked --no-default-features`: 13 module integration tests fail.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: fails
  with 35 diagnostics, including two default-deny `unused_io_amount` findings.
- `cargo doc --locked --no-deps`: succeeds with 14 `rustdoc::bare_urls`
  warnings.

## P1: Data integrity and confidentiality

- [ ] Constrain config reads/writes to the intended directory
  (`src/commands/module/config.rs::set_permissions` and
  `src/commands/module/config/{get,set,setup}.rs`). Permission repair skips
  symlinks, while reads follow them and symlinked config directories can redirect
  writes outside the config tree.
  Define and enforce symlink handling for directories and files, including
  replacement races, and test that external targets remain untouched.

## P2: CLI behavior and process lifecycle

- [ ] Collect external help only when requested (`src/main.rs:128-132`,
  `after_long_help`). Constructing the parser eagerly runs `Help.execute()`;
  even `asimov --version` invokes every discovered external command with
  `--help` (reproduced). Make ordinary dispatch and short informational paths
  avoid those subprocesses; test startup with a side-effecting fake command.

- [ ] Bound and correctly reap help subprocesses
  (`src/commands/{help,help_cmd}.rs`). Collection waits for exit before draining
  pipes, uses one deadline for the entire parallel batch, busy-polls, and kills
  without waiting; explicit external help has no timeout. Drain stdout/stderr
  concurrently, bound output, apply per-child deadlines, and always reap
  children. Exercise large help output, hangs, failed spawns, and termination.

- [ ] Defer identity and filesystem initialization to the commands needing it
  (`src/main.rs:108-117,292-311`). Help/version/license require a working OS
  keyring, while unrelated commands create module and snapshot directories.
  Snapshot-directory failure can even prevent telemetry opt-out. Preserve
  identity initialization for actual operations, but test informational commands
  with an unavailable keyring and independent commands with unwritable storage.

- [ ] Make external help and aliases respect the parsed command position
  (`src/main.rs:178-211`, `src/commands/help_cmd.rs`, `src/aliases.rs`).
  `asimov --color never help probe` fails although `asimov help probe` succeeds
  (reproduced); nested external help prepends `--help` before subcommands.
  Cover leading global options and nested help. Also feature-gate alias
  expansion/help entries so reduced builds preserve external-command dispatch.

- [ ] Preserve external command arguments and exit statuses
  (`src/main.rs::Command::External`, `src/commands/external.rs`). An external
  exit status of 42 becomes `EX_SOFTWARE` (reproduced), and signal statuses are
  similarly forced into `SysexitsError`. Carry raw process exit status across
  this boundary and retain `OsString` arguments for non-UTF-8 paths. Test normal
  failures, Unix signals, and byte-preserving argument forwarding.

- [ ] Report raw errors with context and handle broken pipes deliberately
  (`src/main.rs::sysexits`, command output paths). Non-`SysexitsError` values
  are silently reduced to `EX_SOFTWARE`; several config/filesystem errors reach
  this path without diagnostics. Meanwhile, `println!` on a closed pipe panics
  with exit 101 (reproduced). Introduce fallible output/error handling with
  appropriate I/O/config exit codes, preserving existing explicit sysexits.

- [ ] Apply standard color and verbosity options to handler output
  (`src/commands/module/`, `src/commands/source/snap/`, and the proxy). Listing
  still emits ANSI escapes with `--color never` (reproduced), and proxy request
  logging is unconditional. Share color-aware rendering across stdout/stderr,
  honor quiet/verbose modes, and test redirected output as well as help output.

## P2: Module configuration and source execution

- [ ] Preserve whitespace on effective config retrieval
  (`src/commands/module/config/get.rs` and SDK resolution). Define whitespace
  handling consistently across environment, stored values, and defaults, and
  cover precedence and exact output for `get` without `--stored`.

- [ ] Distinguish unset configuration from unreadable configuration
  (`src/commands/module/config.rs::Module::source`, config show, and inspect).
  `try_exists(...).unwrap_or(false)` and `.variable(...).ok()` suppress I/O and
  decoding errors; inspection reduces read failures to an unset status. Resolve
  value and provenance together, preserve errors, and test directories in place
  of values, invalid UTF-8, and permissions failures with defaults present.

- [ ] Test module installation/upgrade against changing release metadata using
  an injectable installer (`src/commands/module/{install,upgrade}.rs`).

- [ ] Bound source subprocess concurrency and make cancellation explicit
  (`src/commands/source/{fetch,list}.rs`). All URL tasks are spawned at once and
  drained in input order; early jq, HTTP, or stdout errors drop remaining join
  handles. Use a bounded, cancellation-aware runner group and verify SDK child
  cleanup on early return. Add fake-runner tests for delayed output, post-output
  failures, ordering, debug/cache/deadline forwarding, and inherited reader I/O.

- [ ] Bound shared HTTP requests and improve Jev transport errors
  (`src/shared.rs::{http_client,post_typesafe,filter_jev_batch}`). The shared
  client configures no connect/read/overall timeout, so Jev filtering can stall
  indefinitely despite a lister deadline. Expose useful, credential-safe
  status/body-generation errors. Test slow responses, HTTP failures, and upload
  cancellation against an injectable local endpoint.

## P2: Proxy reliability and interoperability

- [ ] Replace proxy startup/request panics with validated state and errors
  (`src/commands/proxy/serve.rs`). Missing API keys, invalid authorization
  header values, bind failures, and serve errors use
  `expect`/`unwrap`. Validate credentials once, keep the header in shared state,
  and return contextual sysexits. Add graceful shutdown so in-flight responses,
  logs, and command telemetry can finish on termination.

- [ ] Share endpoint configuration across serving, reporting, and templates
  (`src/commands/proxy/` and its `config/` templates).
  IPv6 reporting emits invalid `http://::1:1920/v1` (reproduced); generated and
  installed configs hardcode port 1920 despite environment overrides. Centralize
  validation and URL rendering, reject malformed environment values, and test
  custom ports, IPv4/IPv6, and wildcard-bind versus client-address semantics.

- [ ] Bound proxy request buffering and network waits
  (`src/commands/proxy/serve.rs:138-142` and its `ProxyConnector`). Bodies are
  collected without a size limit, and connection, CONNECT, SOCKS, TLS, and
  upstream-header waits have no explicit deadlines. Add configurable limits,
  appropriate 413/504 responses, and bounded concurrency while preserving
  streaming responses. Test slow uploads/upstreams and client disconnection.

- [ ] Strip hop-by-hop headers in both proxy directions
  (`src/commands/proxy/serve.rs::proxy_handler`). Forwarding currently removes
  only request Host/Content-Length and passes upstream response headers through.
  Remove Connection-nominated fields and standard hop-by-hop/proxy-auth headers,
  reconcile framing, and test keep-alive, chunked responses, and streamed SSE
  using a local upstream fixture.

- [ ] Cover conventional upstream-proxy addressing and authentication
  (`src/commands/proxy/serve/{proxy_config,proxy_connector}.rs`).
  URL credentials lack percent-decoding, and local SOCKS DNS uses only the
  first address. Add tests for encoded credentials, IPv6, multiple addresses,
  and port-aware NO_PROXY rules.
  Test CONNECT framing/status handling and both SOCKS DNS modes against local
  servers rather than relying solely on parser tests.

- [ ] Move body logging off the response polling path
  (`src/commands/proxy/serve/body_logger.rs`). Each frame locks a shared mutex
  and performs synchronous disk writes; write failures are discarded, concurrent
  exchanges lack correlation IDs. Use a bounded writer queue with explicit
  failure/backpressure behavior, request IDs, and shutdown flushing. Test
  slow/full sinks without losing response-stream correctness.

- [ ] Execute generated proxy shell templates on Windows to validate quoting,
  current-session `set` behavior, and persistent `setx` assignments
  (`src/commands/proxy/config.rs`).

## P2: Verification and delivery

- [ ] Exercise default, all-features, and supported reduced-feature builds in
  CI, including the feature-gated module and source integration suites.

- [ ] Strengthen and isolate CLI fixtures (`tests/shared.rs`, external-command
  tests, and `tests/module/`). Commented presence/success
  assertions let several tests pass when every lookup/execution fails. Assert
  expected success and failure paths, include stderr in failures, and verify
  setup commands' statuses. Prefer per-child PATH/environment configuration over
  unsafe global mutation; isolate cwd, dotenv, telemetry, and keyring state.

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
  optional `asimov-proxy`/`mime`, and generated shadow metadata, which have no
  corresponding source consumers. Gate experimental SDK dependencies and narrow
  broad Tokio/default features where practical. Measure clean-build time and
  binary size, checking feature unification before removing dependencies.

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

- [ ] Add deterministic snapshot lifecycle and timestamp coverage
  (`src/commands/source/snap/`, `src/timestamps.rs`). Exercise save/list/log/
  compact with temporary storage, normalized URLs, corruption, and failures.
  Replace wall-clock-dependent timestamp tests and global tracing initialization
  with fixed instants; include month ends, leap years, DST, and future times.
  Avoid panics from relative-time formatting and consolidate the unused parallel
  `create` implementation with the dispatched `save` path.

- [ ] Test telemetry's metadata and lifecycle contracts
  (`src/shared/telemetry{,_disabled}.rs`, `src/commands/configure.rs`,
  `src/main.rs`). Use a fake/local sink to cover aliases/default subcommands,
  external-command redaction, module-name deduplication, failures, cancellation,
  and opt-out without flushing queued events first. Assert that URLs, arbitrary
  arguments, config values, and secrets never enter serialized events.

- [ ] Establish warning-clean lint and rustdoc baselines
  (`src/shared.rs`, `src/commands/`, public library APIs). Address the remaining
  strict Clippy findings and feature-specific dead imports/functions. Document
  public error/output contracts.

- [ ] Repair generated command examples and reference snippets
  (`.config/readmer/README.md.liquid`, `Rakefile`, `etc/readmer/`, `Makefile`).
  Examples still use the absent top-level `asimov import` alias and describe
  module management as a separate installation. Rake's help capture is commented
  out, leaving reference snippets with only command prompts. Generate real,
  deterministic help for supported commands, track snippet dependencies, and
  regenerate the README from its template.
