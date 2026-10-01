# ASIMOV Command-Line Interface (CLI)

[![License](https://img.shields.io/badge/license-Public%20Domain-blue.svg)](https://unlicense.org)
[![Compatibility](https://img.shields.io/badge/rust-1.97.1%2B-blue)](https://endoflife.date/rust)
[![Package on Crates.io](https://img.shields.io/crates/v/asimov-cli)](https://crates.io/crates/asimov-cli)
[![Documentation](https://img.shields.io/docsrs/asimov-cli?label=docs.rs)](https://docs.rs/asimov-cli)

> [!TIP]
> 🚧 _We are building in public. This is presently under heavy construction._

## ✨ Features

- Cuts red tape: 100% free and unencumbered public domain software.

## 🛠️ Prerequisites

- [Rust] 1.97.1+ (2024 edition)

## ⬇️ Installation

### Installation from GitHub

#### Installation via [Cargo Binstall]

```bash
cargo binstall -y asimov-cli
```

<!--
<img width="100%" alt="Installation via cargo-binstall" src="https://github.com/asimov-platform/asimov-cli/raw/master/etc/asciinema/install.gif"/>
-->

#### Installation via [Cargo]

```bash
cargo install asimov-cli --locked
```

### Installation using a Package Manager

<details>
<summary>Homebrew</summary>

#### [Homebrew](https://brew.sh)

Firstly, register this tap in your local Homebrew installation with:

```bash
brew tap asimov-platform/tap
```

Now you can install ASIMOV CLI with:

```bash
brew install asimov-cli
```

</details>

<details>
<summary>Scoop</summary>

#### [Scoop](https://scoop.sh)

First things first, you need to add our custom Scoop bucket:

```bash
scoop bucket add asimov-platform https://github.com/asimov-platform/scoop-bucket
```

Now, installing ASIMOV CLI is as easy as running:

```bash
scoop install asimov-platform/asimov-cli
```

</details>

<details>
<summary>Nix Flakes</summary>

#### [Nix Flakes](https://nixos.wiki/wiki/Flakes)

Nix flakes is an experimental feature that has to be enabled before going any further:

```bash
mkdir -p ~/.config/nix && echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
```

Now you can register the flake using:

```bash
nix registry add asimov-cli github:asimov-platform/nix-flake
```

And then install ASIMOV CLI with:

```bash
nix profile install asimov-cli#default --no-write-lock-file
```

</details>

<details>
<summary>Flatpak</summary>

#### [Flatpak](https://flatpak.org)

First add the ASIMOV Platform Flatpak remote:

```bash
flatpak remote-add --if-not-exists --user asimov-cli --no-gpg-verify https://asimov-platform.github.io/flatpak
```

Then install ASIMOV CLI with:

```bash
flatpak install asimov-cli so.asimov.cli
```

Now you can run it like this:

```bash
flatpak run so.asimov.cli --help
```

You may want to create an alias for it:

```bash
alias asimov="flatpak run so.asimov.cli"
```

</details>

## 👉 Examples

Show help, including all available commands:

```bash
asimov help
```

> [!TIP]
> Add one or more `-v` flags to increase the verbosity level.

<details>
<summary>Fetch Data</summary>

## Fetch Data

```bash
# Fetch data from a URL, automatically choosing from installed modules
asimov fetch https://example.com/

# To fetch with a specific module use `-M` or `--module`
asimov fetch -M http https://example.com/

# Fetch multiple URLs
asimov fetch https://asimov.sh/ https://asimov.blog/
```

</details>

<details>
<summary>Import Data From a URL</summary>

### Import Data From a URL

If you have the [ASIMOV Bright Data module](https://github.com/asimov-modules/asimov-brightdata-module) installed and configured, you should be able to fetch various social platform resources:

```bash
# Import data from a URL, automatically choosing from installed modules
asimov import https://x.com/asimov_platform

# Import using the specific module
asimov import -M brightdata https://x.com/asimov_platform
```

</details>

<details>
<summary>External Commands</summary>

### External Commands

The CLI automatically discovers and runs external commands starting with `asimov-`.
If you installed using a package manager you should have access to [ASIMOV Module CLI] for managing installed [modules](https://asimov.directory/modules):

```bash
# If you have asimov-module-cli installed
asimov module [arguments]

asimov module install http

# Get help for external commands
asimov help module
```

</details>

## 📚 Reference

### Command-Line Interface

```shellsession
$ asimov
```

#### `asimov module`

```shellsession
$ asimov module --help
```

<details>
<summary><code>asimov module install</code></summary>

```shellsession
$ asimov module install --help
```

</details>

#### `asimov proxy`

```shellsession
$ asimov proxy --help
```

<details>
<summary><code>asimov proxy serve</code></summary>

```shellsession
$ asimov proxy serve --help
```

</details>

<details>
<summary><code>asimov proxy url</code></summary>

```shellsession
$ asimov proxy url --help
```

</details>

<details>
<summary><code>asimov proxy host</code></summary>

```shellsession
$ asimov proxy host --help
```

</details>

<details>
<summary><code>asimov proxy port</code></summary>

```shellsession
$ asimov proxy port --help
```

</details>

<details>
<summary><code>asimov proxy models</code></summary>

```shellsession
$ asimov proxy models --help
```

</details>

<details>
<summary><code>asimov proxy config</code></summary>

```shellsession
$ asimov proxy config --help
```

</details>

<details>
<summary><code>asimov proxy install</code></summary>

```shellsession
$ asimov proxy install --help
```

</details>

#### `asimov source`

```shellsession
$ asimov source --help
```

<details>
<summary><code>asimov source fetch</code></summary>

```shellsession
$ asimov source fetch --help
```

</details>

<details>
<summary><code>asimov source list</code></summary>

```shellsession
$ asimov source list --help
```

</details>

<details>
<summary><code>asimov source read</code></summary>

```shellsession
$ asimov source read --help
```

</details>

<details>
<summary><code>asimov source snap</code></summary>

```shellsession
$ asimov source snap --help
```

</details>

## 📡 Telemetry

The ASIMOV CLI binaries published in [GitHub releases] come with **opt-out
telemetry**. Builds from source, such as `cargo install asimov-cli`, don't
send anything.

### Opting Out

Any one of these disables telemetry:

```bash
# Permanently, for the current user:
asimov configure telemetry disable

# Per invocation or per shell, using an environment variable:
export ASIMOV_TELEMETRY=0   # or `false`

# By building the CLI yourself, without the `telemetry` feature:
cargo install asimov-cli --locked --no-default-features --features module,proxy,source
```

Disabling telemetry also discards any events that haven't been sent yet.
To turn it back on, run `asimov configure telemetry enable`.

<details>
<summary>What is collected</summary>

Each command you run records when it started and finished. Each event
contains:

- Your ASIMOV public key (`~/.asimov/keyring/$USER`), as an identifier
- The command that was run (e.g., `source fetch`), without its arguments
- The names of the modules involved
- The exit code and duration
- The CLI version, operating system, and CPU architecture
- Whether the CLI was invoked by a coding agent, and which one

URLs, arguments, file paths, command output, and error messages are never
collected.

Events are stored locally in `~/.asimov/.telemetry/` and periodically sent to
[Statsig](https://statsig.com) in the background.

The events are defined in the [`asimov-telemetry`] crate.
</details>

## 👨‍💻 Development

```bash
git clone https://github.com/asimov-platform/asimov-cli.git
```

---

[![Share on X](https://img.shields.io/badge/share%20on-x-03A9F4?logo=x)](https://x.com/intent/post?url=https%3A%2F%2Fgithub.com%2Fasimov-platform%2Fasimov-cli&text=ASIMOV%20Command-Line%20Interface%20%28CLI%29)
[![Share on Reddit](https://img.shields.io/badge/share%20on-reddit-red?logo=reddit)](https://reddit.com/submit?url=https%3A%2F%2Fgithub.com%2Fasimov-platform%2Fasimov-cli&title=ASIMOV%20Command-Line%20Interface%20%28CLI%29)
[![Share on Hacker News](https://img.shields.io/badge/share%20on-hn-orange?logo=ycombinator)](https://news.ycombinator.com/submitlink?u=https%3A%2F%2Fgithub.com%2Fasimov-platform%2Fasimov-cli&t=ASIMOV%20Command-Line%20Interface%20%28CLI%29)
[![Share on Facebook](https://img.shields.io/badge/share%20on-fb-1976D2?logo=facebook)](https://www.facebook.com/sharer/sharer.php?u=https%3A%2F%2Fgithub.com%2Fasimov-platform%2Fasimov-cli)
[![Share on LinkedIn](https://img.shields.io/badge/share%20on-linkedin-3949AB?logo=linkedin)](https://www.linkedin.com/sharing/share-offsite/?url=https%3A%2F%2Fgithub.com%2Fasimov-platform%2Fasimov-cli)

[`asimov`]: https://github.com/asimov-platform/asimov-cli#command-line-interface
[`asimov-telemetry`]: https://github.com/asimov-platform/asimov-sdk/tree/master/rust/lib/asimov-telemetry
[Crates.io]: https://crates.io/crates/asimov-cli
[GitHub releases]: https://github.com/asimov-platform/asimov-cli/releases
[feature flags]: https://docs.rs/crate/asimov-cli/latest/features
[naming conventions]: https://rust-lang.github.io/api-guidelines/naming.html
[ASIMOV Module CLI]: https://github.com/asimov-platform/asimov-module-cli
[Cargo]: https://rustup.rs
[Cargo Binstall]: https://crates.io/crates/cargo-binstall
[Rust]: https://rust-lang.org
