@echo off

cargo build --release --target x86_64-pc-windows-msvc
cargo publish --allow-dirty
