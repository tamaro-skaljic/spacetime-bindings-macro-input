@echo off

rustup target add x86_64-unknown-linux-gnu x86_64-unknown-linux-musl

cargo fmt --all -- --check
cargo clippy --all-targets --all-features || echo "Clippy warnings"

cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-musl
