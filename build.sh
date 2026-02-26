#/bin/sh

cargo fmt --all -- --check
cargo clippy --all-targets --all-features || echo "Clippy warnings"

cd example

echo "Building and publishing a module using SpacetimeDB..."
spacetime publish --server local --delete-data=always spacetime-bindings-macro-input --yes || echo "Publish failed"

echo "Showing logs..."
spacetime logs --server local spacetime-bindings-macro-input --yes || echo "Logs failed"

echo "Cleaning up module..."
spacetime delete --server local spacetime-bindings-macro-input --yes || echo "Delete failed"
