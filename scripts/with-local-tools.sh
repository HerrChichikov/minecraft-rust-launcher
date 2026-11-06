#!/bin/sh
set -eu

launcher_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)

if [ -x "$launcher_root/.tools/node/bin/node" ]; then
  PATH="$launcher_root/.tools/node/bin:$PATH"
fi
if [ -x "$launcher_root/.tools/cargo/bin/cargo" ]; then
  CARGO_HOME="$launcher_root/.tools/cargo"
  RUSTUP_HOME="$launcher_root/.tools/rustup"
  PATH="$CARGO_HOME/bin:$PATH"
  export CARGO_HOME RUSTUP_HOME
fi
export PATH

exec "$@"
