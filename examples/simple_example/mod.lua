-- Simple Example Mod for infinite
-- This is a minimal example to demonstrate mod structure

console.log("Hello from Simple Example mod!")

-- Get the version
local version = infinite.getVersion()
console.log("infinite version: " .. version)

-- Get the full version
local fullVersion = infinite.getFullVersion()
console.log("Full version: " .. fullVersion[1] .. "." .. fullVersion[2] .. "." .. fullVersion[3])

-- Access configuration
local customText = config.customText or "Default text"
console.log("Custom text: " .. customText)

-- Write a simple text file
local content = "This file was created by the Simple Example mod.\n"
content = content .. customText .. "\n"
content = content .. "Generated at mod installation time.\n"

infinite.writeTxt("mods\\simple_example_output.txt", content)

console.log("Simple Example mod completed!")
