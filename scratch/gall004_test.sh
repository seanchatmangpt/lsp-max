#!/bin/bash
export RUST_BACKTRACE=1
cargo test --manifest-path /Users/sac/ggen/crates/ggen-lsp/Cargo.toml --test dogfood_gc004 -- --nocapture
