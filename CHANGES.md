# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased
### Fixed
- Forward external command arguments without requiring UTF-8
- Resolve external help commands after leading global options
- Limit proxy request bodies to 16 MiB by default (`--max-body-bytes` overrides)
- Format reported IPv6 proxy URLs with address brackets
- Initialize module and snapshot storage only for commands that use it
- Run external help discovery only when root long help is requested
- Forward nested external help requests to the intended subcommand
- Expand and advertise aliases only for commands enabled in the build
- Stop module inspection on configuration read errors before emitting a report
- Report unreadable configuration values instead of displaying them as unset
- Preserve whitespace in effective configuration values from every source
- Report unclassified command errors and return specific I/O failure codes
- Allow help, version, and license output without initializing local identity
- Log proxy request URLs only in verbose mode
- Report proxy bind and server failures without panicking
- Validate proxy API credentials at startup without panics or secret disclosure
- Decode percent-encoded HTTP and SOCKS proxy credentials
- Reject unsafe and colliding variable names in config, inspect, and install
- Open only valid HTTP or HTTPS module links in the browser
- Replace patched application settings atomically while preserving permissions
- Write module configuration values atomically with private permissions
- Create proxy body logs with private permissions on Unix
- Reject invalid Jev input JSON before uploading a batch
- Rank module links by actual URL hosts and path segments
- Resolve IPv6 upstream proxy addresses without URL brackets
- Generate correct, distinct Windows `set` and `setx` commands
- Reject unsupported proxy configuration formats instead of producing no output
- Identify proxy model output as a static example and reject invalid formats
- Require URLs for source fetch, list, and read operations
- Require module names for install, uninstall, enable, and disable
- Report registry state errors before attempting module installation
- Preserve value whitespace in `module config get --stored`
- Report unsupported experimental proxy targets instead of panicking
- Reject unsupported module output formats during argument parsing
- Install and upgrade to the exact module version resolved by the CLI
- Handle incomplete writes when generating Jev requests
- Preserve unreadable or invalid application configs during proxy installation
- Escape special characters in module JSONL output
- Redact upstream proxy credentials from diagnostics
- Hide secret defaults in JSON module inspection output
- Preserve all jq results and report later filter errors in source commands
- Reject invalid Jev answers before selecting source records

## 25.6.1 - 2026-10-02
### Added
- Optional telemetry (`asimov configure telemetry {enable,disable}`)
### Changed
- Double the default `--jev` batch size
- Bump the SDK and dependencies

## 25.6.0 - 2026-09-21
### Added
- `asimov list --jev=NOUL` (requires `TYPESAFE_API_TOKEN`)

## 25.5.0 - 2026-09-10
### Added
- Add source caching options to `asimov fetch` and `asimov list`
### Changed
- Rename cataloger terminology and program lookup to lister
### Fixed
- Support jq filters that produce no output

## 25.4.3 - 2026-09-09
### Added
- `asimov {fetch,list} --jq=EXPR`

## 25.4.2 - 2026-09-08
### Added
- Resume building for macOS on x86-64

## 25.4.1 - 2026-09-07
### Added
- Generate and store a secret key in the system keyring
- Store the public key in `~/.asimov/keyring/$USER`

## 25.4.0 - 2026-09-05
### Changed
- Reorganize the command hierarchy in prep for the 26.0 release
- Add support for command aliases to preserve backwards compability
- Disable all incomplete/unstable features and commands

## 25.3.0 - 2026-08-12
### Added
- Add `asimov module doc`, `inspect`, and `search` subcommands
- Add `asimov package init`, `check`, and `tree` subcommands behind the `unstable` feature flag
### Changed
- Overhaul the `asimov module config` subcommand
- Update ASIMOV SDK dependencies
- Bump the MSRV to Rust 1.97.1
### Fixed
- Fix `asimov module list` help text

## 25.2.0 - 2026-07-30
### Added
- Add `asimov module new` behind the `unstable` feature flag
### Changed
- Add binary installation support through `cargo-binstall`
### Fixed
- Fix `cargo-binstall` URL templates for Linux targets

## 25.1.1 - 2026-03-16
### Fixed
- Fix a runtime tokio panic

## 25.1.0 - 2026-03-07
### Added
- `asimov module` now bundled in the default build (by @artob)
- `asimov snapshot` now bundled in the default build (by @artob)
### Changed
- Bump the MSRV to 1.93 to match ASIMOV.rs (by @artob)

## 25.0.3 - 2026-01-22
### Fixed
- Make `ask` output always have a newline

## 25.0.2 - 2025-11-25
### Fixed
- Remove panics on executor failure (#93 by @SamuelSarle)
- Fix handling of `help` flag and command (#92 by @SamuelSarle)

## 25.0.1 - 2025-11-12
### Added
- Add missing help messages
### Changed
- Update ASIMOV SDK dependencies

## 25.0.0 - 2025-11-05
### Added
- General availability

## 25.0.0-dev.13 - 2025-10-22
### Added
- Implement `asimov read` (#82 by @SamuelSarle)

## 25.0.0-dev.12 - 2025-08-21
### Added
- Implement `asimov ask` (#71 by @SamuelSarle)
### Changed
- Update ASIMOV SDK dependencies

## 25.0.0-dev.11 - 2025-08-21
### Added
- Implement `asimov snap` (#48 by @SamuelSarle)
### Changed
- Remove the `serde_yml` dependency (#41 by @imunproductive)
- Add hint when no modules are installed (#53 by @SamuelSarle)

## 25.0.0-dev.10 - 2025-08-01
### Added
- Utilise asimov_installer (#42 by @SamuelSarle)
### Fixed
- Remove the serde_yml dependency (#41 by @imunproductive)

## 25.0.0-dev.9 - 2025-07-29
### Added
- Remove `asimov import` in favor of `asimov fetch` (by @artob)
- Define aliases for built-in commands (by @artob)
### Changed
- Remove the OpenSSL dependency (by @imunproductive)

## 25.0.0-dev.8 - 2025-07-15
### Added
- Stabilize `asimov list` (by @artob)
### Changed
- Pass through `--limit` and `--output` flags (by @artob)

## 25.0.0-dev.7 - 2025-07-02
### Added
- `asimov fetch` (#30 by @SamuelSarle)
- `asimov import` (#30 by @SamuelSarle)
### Changed
- Normalize URLs before resolution (#31 by @SamuelSarle)
### Fixed
- Fix OpenSSL builds (@imunproductive)

## 25.0.0-dev.6 - 2025-06-27
### Changed
- Bump the MSRV to 1.85 (2024 edition)

## 25.0.0-dev.5 - 2025-06-27
### Added
- Implement `asimov fetch` (#29)
- Implement `asimov import` (#29)
### Changed
- Enhance `asimov help` (#28)

## 25.0.0-dev.4 - 2025-04-03

## 25.0.0-dev.3 - 2025-03-13

## 25.0.0-dev.2 - 2025-02-22

## 25.0.0-dev.1 - 2025-02-19

## 25.0.0-dev.0 - 2025-02-13
