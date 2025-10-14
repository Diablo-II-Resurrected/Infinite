# Quick Start Guide

This guide will help you get started with infinite CLI quickly.

## Installation

### Building from Source

```bash
cd infinite
cargo build --release
```

The compiled binary will be at `target/release/infinite` (or `infinite.exe` on Windows).

## Basic Usage

### 1. List Available Mods

```bash
infinite list --mods-path "./mods"
```

Output:
```
📦 Available Mods
═════════════════════════════════════

1. Stack Size Changer v1.0.0
   Change the stack size for stackable items
   By: infinite Team
   ⚙️ 2 configuration option(s)

2. Simple Text Modifier v1.0.0
   A simple example mod
   By: infinite Team
   ⚙️ 1 configuration option(s)

═════════════════════════════════════
Total: 2 mod(s)
```

### 2. Validate a Mod

Before installing, validate your mod configuration:

```bash
infinite validate --mod-path "./mods/MyMod"
```

### 3. Install Mods (Dry Run)

Test without actually writing files:

```bash
infinite install \
    --game-path "C:/Program Files (x86)/Diablo II Resurrected" \
    --mods-path "./mods" \
    --output-path "./output" \
    --dry-run
```

### 4. Install Mods

When ready, install for real:

```bash
infinite install \
    --game-path "C:/Program Files (x86)/Diablo II Resurrected" \
    --mods-path "./mods" \
    --output-path "./output"
```

Output:
```
🎮 infinite CLI - Installing Mods
═════════════════════════════════════
  Game:  C:/Program Files (x86)/Diablo II Resurrected
  Mods:  ./mods
  Output: ./output
═════════════════════════════════════

📦 Found 2 mod(s)

⚙️ 1/2 - Stack Size Changer v1.0.0
   ✅ Installed in 0.12s

⚙️ 2/2 - Simple Text Modifier v1.0.0
   ✅ Installed in 0.05s

═════════════════════════════════════
📊 File Operations Summary:
   Total files tracked: 3
   Files extracted: 1
   Files modified: 2

═════════════════════════════════════
🎉 All mods processed in 0.18s
```

## Creating Your First Mod

### Step 1: Create Mod Directory

```bash
mkdir -p mods/MyFirstMod
cd mods/MyFirstMod
```

### Step 2: Create mod.json

```json
{
  "name": "My First Mod",
  "description": "My first infinite mod",
  "author": "Your Name",
  "version": "1.0.0",
  "config": [
    {
      "type": "checkbox",
      "id": "enabled",
      "name": "Enable Mod",
      "default": true
    }
  ]
}
```

### Step 3: Create mod.lua

```lua
console.log("Installing My First Mod...")

if config.enabled then
    -- Read a game file
    local data = infinite.readJson("global\\excel\\misc.json")
    
    -- Modify it
    for i, item in ipairs(data) do
        if item.name == "Gold" then
            console.log("Found Gold item!")
        end
    end
    
    -- Write it back
    infinite.writeJson("global\\excel\\misc.json", data)
    
    console.log("Mod installed successfully!")
else
    console.log("Mod is disabled")
end
```

### Step 4: Test Your Mod

```bash
# Validate
infinite validate --mod-path mods/MyFirstMod

# Test install (dry run)
infinite install \
    --game-path "C:/Program Files (x86)/Diablo II Resurrected" \
    --mods-path mods \
    --output-path output \
    --dry-run
```

## Common Operations

### Enable Verbose Logging

```bash
infinite install --verbose ...
```

### Working with Configuration

User configurations are accessed via the `config` global variable:

```lua
-- In mod.json
{
  "config": [
    {
      "type": "number",
      "id": "stackSize",
      "name": "Stack Size",
      "default": 100
    }
  ]
}

-- In mod.lua
local size = config.stackSize  -- 100 (or user's value)
```

### Reading and Writing Files

```lua
-- JSON
local data = infinite.readJson("path/to/file.json")
infinite.writeJson("path/to/file.json", data)

-- TSV (Tab-Separated Values)
local rows = infinite.readTsv("path/to/file.txt")
infinite.writeTsv("path/to/file.txt", rows)

-- Plain Text
local text = infinite.readTxt("path/to/file.txt")
infinite.writeTxt("path/to/file.txt", "new content")

-- Copy files from mod
infinite.copyFile("myfile.png", "data/hd/ui/myfile.png")
```

## Tips

1. **Always test with --dry-run first**
2. **Use console.log() for debugging**
3. **Validate your mod before installing**
4. **Check file paths carefully** (use backslashes for game files)
5. **Keep mods small and focused**

## Next Steps

- Read the [full API documentation](../README.md#api-reference)
- Check out the [example mods](../examples/)
- Join the community for help and sharing

Happy modding! 🎮
