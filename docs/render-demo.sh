#!/bin/sh
# Renders docs/pervitin.gif, the demo in the README, from docs/pervitin.tape.
# It records this checkout's own build in a dry run, so nothing is changed
# and no sudo is asked for. Needs: brew install vhs gifsicle
set -eu
cd "$(dirname "$0")/.."

for tool in cargo vhs gifsicle; do
  command -v "$tool" >/dev/null || { echo "render-demo: $tool is not installed" >&2; exit 1; }
done

cargo build --release --locked
PATH="$PWD/target/release:$PATH" vhs docs/pervitin.tape
gifsicle -O3 --lossy=60 --colors 96 pervitin-raw.gif -o docs/pervitin.gif
rm pervitin-raw.gif
ls -l docs/pervitin.gif
