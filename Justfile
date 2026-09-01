set shell := ["bash", "-cu"]

bin := "asp-rust"
features := "cli"

default:
    @just --list

build-cli:
    cargo build --features {{features}} --bin {{bin}}

# ASP owns artifact publication; this package owns its development build.
install:
    cargo build --offline --release --features {{features}} --bin {{bin}}

install-bin-macos prefix="/opt/homebrew":
    CARGO_INSTALL_ROOT="{{prefix}}" cargo install --path . --features {{features}} --bin {{bin}} --force

install-bin-linux prefix="/usr/local":
    CARGO_INSTALL_ROOT="{{prefix}}" cargo install --path . --features {{features}} --bin {{bin}} --force

install-bin:
    #!/usr/bin/env bash
    set -euo pipefail
    case "$(uname -s)" in
      Darwin) just install-bin-macos ;;
      Linux) just install-bin-linux ;;
      *) echo "unsupported OS: $(uname -s)" >&2; exit 1 ;;
    esac
