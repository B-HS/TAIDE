#!/bin/bash
set -u
ROOT=/Users/hyunseokbyun/development/TAIDE
OUT=${1:?log directory}
TARGET=(--locked --offline --target-dir "$ROOT/experiments/native-shell-spike/target")
mkdir -p "$OUT"
cargo build --quiet --manifest-path "$ROOT/native/taide-native-app/Cargo.toml" --example native-lsp-mock "${TARGET[@]}"
echo "lsp-mock $?"
cargo test --manifest-path "$ROOT/native/taide-native-app/Cargo.toml" "${TARGET[@]}" --no-fail-fast -- --test-threads=4 >"$OUT/app.log" 2>&1
echo "app $?"
cargo test --manifest-path "$ROOT/native/taide-native-ui/Cargo.toml" --features native-host,inspection "${TARGET[@]}" --no-fail-fast >"$OUT/ui.log" 2>&1
echo "ui $?"
for crate in editor syntax terminal; do
    cargo test --manifest-path "$ROOT/native/taide-native-$crate/Cargo.toml" "${TARGET[@]}" --no-fail-fast >"$OUT/$crate.log" 2>&1
    echo "$crate $?"
done
cargo check --quiet --manifest-path "$ROOT/native/taide-remote-web/Cargo.toml" --target wasm32-unknown-unknown --lib "${TARGET[@]}" >"$OUT/remote-web.log" 2>&1
echo "remote-web $?"
for crate in app ui editor syntax terminal; do
    cargo fmt --manifest-path "$ROOT/native/taide-native-$crate/Cargo.toml" -- --check >/dev/null 2>&1
    echo "fmt-$crate $?"
done
for log in "$OUT"/*.log; do
    echo "== $(basename "$log") ok=$(grep -c '^test result: ok' "$log")"
    grep -E ' \.\.\. FAILED$|^error(\[|:)' "$log" | sort -u
done
