# infinite Example Mods

This directory contains example mods to demonstrate how to create mods for infinite CLI.

## Examples

### 1. Simple Example (`simple_example/`)
A minimal example that demonstrates:
- Basic mod structure
- Accessing configuration
- Writing text files
- Using console logging
- Getting infinite version

### 2. Stack Size Changer (`stack_size_changer/`)
A practical example that demonstrates:
- Reading JSON game files
- Modifying game data
- Writing modified JSON files
- Using numeric configuration
- Conditional logic

## Testing Examples

You can test these examples using:

```bash
# List the example mods
cargo run -- list --mods-path ./examples

# Validate a specific mod
cargo run -- validate --mod-path ./examples/simple_example

# Install mods (dry run)
cargo run -- install \
    --game-path "C:/Program Files (x86)/Diablo II Resurrected" \
    --mods-path ./examples \
    --output-path ./test_output \
    --dry-run
```

## Creating Your Own Mod

1. Create a new directory in your mods folder
2. Add `mod.json` with mod metadata
3. Add `mod.lua` with mod logic
4. Test with `infinite validate` and `infinite install --dry-run`

See the main README for API documentation.
