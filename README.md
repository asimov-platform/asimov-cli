# ASIMOV Command-Line Interface (CLI)

[![License](https://img.shields.io/badge/license-Public%20Domain-blue.svg)](https://unlicense.org)
[![Compatibility](https://img.shields.io/badge/rust-1.85%2B-blue)](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/)
[![Package](https://img.shields.io/crates/v/asimov-cli)](https://crates.io/crates/asimov-cli)

🚧 _We are building in public. This is presently under heavy construction._

## ✨ Features

- 100% free and unencumbered public domain software.

## 🛠️ Prerequisites

- [Rust](https://rust-lang.org) 1.85+

## ⬇️ Installation

### Installation from Source Code

#### Installation via Cargo

```bash
cargo install asimov-cli --version 25.0.0-dev.8
```

### Installation using Package Manager

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
<summary>Nix flakes</summary>

#### [Nix flakes](https://nixos.wiki/wiki/Flakes)

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

When running commands you can add one or more `-v` flags to increase verbosity level.

## Fetch data

```bash
# Fetch data from a URL, automatically choosing from installed modules
asimov fetch https://example.com/

# To fetch with a specific module use `-m` or `--module`
asimov fetch -m http https://example.com/

# Fetch multiple URLs
asimov fetch https://asimov.sh/ https://asimov.blog/
```

### Import data as RDF from a URL

If you have the [ASIMOV Bright Data module](https://github.com/asimov-modules/asimov-brightdata-module) installed and configured, you should be able to fetch various social platform resources:

```bash
# Import data from a URL, automatically choosing from installed modules
asimov import https://x.com/asimov_platform

# Import using the specific module
asimov import -m brightdata https://x.com/asimov_platform
```

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

## 📚 Reference

TBD

## 👨‍💻 Development

```bash
git clone https://github.com/asimov-platform/asimov-cli.git
```

---

[![Share on X](https://img.shields.io/badge/share%20on-x-03A9F4?logo=x)](https://x.com/intent/post?url=https://github.com/asimov-platform/asimov-cli&text=ASIMOV%20Command-Line%20Interface%20%28CLI%29)
[![Share on Reddit](https://img.shields.io/badge/share%20on-reddit-red?logo=reddit)](https://reddit.com/submit?url=https://github.com/asimov-platform/asimov-cli&title=ASIMOV%20Command-Line%20Interface%20%28CLI%29)
[![Share on Hacker News](https://img.shields.io/badge/share%20on-hn-orange?logo=ycombinator)](https://news.ycombinator.com/submitlink?u=https://github.com/asimov-platform/asimov-cli&t=ASIMOV%20Command-Line%20Interface%20%28CLI%29)
[![Share on Facebook](https://img.shields.io/badge/share%20on-fb-1976D2?logo=facebook)](https://www.facebook.com/sharer/sharer.php?u=https://github.com/asimov-platform/asimov-cli)
[![Share on LinkedIn](https://img.shields.io/badge/share%20on-linkedin-3949AB?logo=linkedin)](https://www.linkedin.com/sharing/share-offsite/?url=https://github.com/asimov-platform/asimov-cli)

[ASIMOV Module CLI]: https://github.com/asimov-platform/asimov-module-cli
