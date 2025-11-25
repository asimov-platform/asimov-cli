# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
