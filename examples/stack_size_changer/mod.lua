-- Stack Size Changer Mod for infinite
-- This mod changes the stack size for stackable items

-- Check infinite version
if infinite.getVersion() < 1.5 then
    infinite.error("This mod requires infinite version 1.5 or higher!")
end

-- Check if mod is enabled
if not config.enabled then
    console.log("Stack Size Changer is disabled")
    return
end

console.log("Installing Stack Size Changer mod...")

-- Get user configuration
local stackSize = config.stackSize or 500

console.log("Stack size: " .. stackSize)

-- Read misc.json (contains item definitions)
local misc = infinite.readJson("global\\excel\\misc.json")

-- Counter for modified items
local modifiedCount = 0

-- Iterate through all items
for i, item in ipairs(misc) do
    if item.maxstack and item.maxstack > 0 then
        -- This item is stackable, update its stack size
        item.maxstack = stackSize
        modifiedCount = modifiedCount + 1

        console.debug("Updated " .. (item.name or "unknown") .. " stack size to " .. stackSize)
    end
end

-- Write back the modified file
infinite.writeJson("global\\excel\\misc.json", misc)

console.log("Modified " .. modifiedCount .. " stackable items")
console.log("Stack Size Changer mod installed successfully!")
