#!/bin/bash
# Times all 125 fences. Run from the repo root after `cargo build --release`.
HU=./target/release/humanuml
PU=~/.config/backbone/bin/plantuml
F=(tests/fences/*.puml)
t() { local s=$(python3 -c 'import time;print(time.time())'); "$@" >/dev/null 2>&1; local e=$(python3 -c 'import time;print(time.time())'); python3 -c "print(f'{($e-$s)*1000:.0f} ms')"; }
best() { local b=999999; for i in 1 2 3 4 5; do local s=$(python3 -c 'import time;print(time.time())'); "$@" >/dev/null 2>&1; local e=$(python3 -c 'import time;print(time.time())'); b=$(python3 -c "print(min($b,($e-$s)*1000))"); done; python3 -c "print(f'{$b:.0f} ms')"; }
echo "humanuml check   (1 process, 125 files): $(best $HU check ${F[@]})"
echo "humanuml render  (1 process, 125 files): $(best $HU render ${F[@]} -o out/bench)"
perjson() { for f in "${F[@]}"; do $HU json $f >/dev/null; done; }
perrender() { for f in "${F[@]}"; do $HU render $f -o out/bench1.svg; done; }
percheck() { for f in "${F[@]}"; do $HU check $f >/dev/null; done; }
echo "humanuml check   (125 processes):        $(best bash -c "$(declare -f percheck); F=(${F[@]}); HU=$HU; percheck")"
echo "humanuml json    (125 processes):        $(best bash -c "$(declare -f perjson); F=(${F[@]}); HU=$HU; perjson")"
echo "humanuml render  (125 processes):        $(best bash -c "$(declare -f perrender); F=(${F[@]}); HU=$HU; perrender")"
echo "plantuml -checkonly (1 process):         $(best $PU -checkonly ${F[@]})"
echo "plantuml -tsvg      (1 process):         $(best $PU -tsvg -o ../../out/bench-pu ${F[@]})"
