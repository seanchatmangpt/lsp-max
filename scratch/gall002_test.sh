#!/bin/bash
export RUST_BACKTRACE=1
cargo test --manifest-path /Users/sac/ggen/crates/ggen-projection/Cargo.toml --test dogfood_gc002 -- --nocapture
