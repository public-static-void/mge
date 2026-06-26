-- loot.lua: Weighted loot table module
--
-- Provides define_table, roll, spawn_for_death, and register_spawner.
-- Module-scoped state; no global namespace pollution.

local tables = {}       -- { [name] = { entries = {...}, total_weight = n } }
local spawners = {}     -- { [item_id] = function(pos) -> entity_id }

-- Default rendering metadata for known items.
-- Fallback: "?" glyph, white color, derived name.
local ITEM_METADATA = {
	health_potion = { glyph = "!", color = { 0, 255, 0 }, name = "Health Potion" },
	rusty_sword = { glyph = "/", color = { 200, 200, 200 }, name = "Rusty Sword" },
}

-- Capitalize first letter of a word
local function capitalize(word)
	return word:sub(1, 1):upper() .. word:sub(2)
end

-- Derive a display name from item_id (e.g., "health_potion" -> "Health Potion")
local function derive_name(item_id)
	local words = {}
	for word in item_id:gmatch("[^_]+") do
		table.insert(words, capitalize(word))
	end
	return table.concat(words, " ")
end

-- Look up rendering metadata for an item_id
local function lookup_metadata(item_id)
	local meta = ITEM_METADATA[item_id]
	if meta then
		return meta
	end
	return { glyph = "?", color = { 200, 200, 200 }, name = derive_name(item_id) }
end

-- Validate a single loot entry
local function validate_entry(entry, idx)
	if type(entry) ~= "table" then
		error("entry " .. idx .. " must be a table, got " .. type(entry))
	end
	if type(entry.item_id) ~= "string" then
		error("entry " .. idx .. ": item_id must be a string, got " .. type(entry.item_id))
	end
	if type(entry.weight) ~= "number" or entry.weight < 1 or entry.weight ~= math.floor(entry.weight) then
		error("entry " .. idx .. ": weight must be an integer >= 1, got " .. tostring(entry.weight))
	end
	local min_count = entry.min_count or 1
	local max_count = entry.max_count or 1
	if type(min_count) ~= "number" or min_count < 1 or min_count ~= math.floor(min_count) then
		error("entry " .. idx .. ": min_count must be an integer >= 1, got " .. tostring(min_count))
	end
	if type(max_count) ~= "number" or max_count < 1 or max_count ~= math.floor(max_count) then
		error("entry " .. idx .. ": max_count must be an integer >= 1, got " .. tostring(max_count))
	end
	if min_count > max_count then
		error("entry " .. idx .. ": min_count (" .. min_count .. ") > max_count (" .. max_count .. ")")
	end
	if entry.condition ~= nil and type(entry.condition) ~= "function" then
		error("entry " .. idx .. ": condition must be a function or nil, got " .. type(entry.condition))
	end
end

--- Register a function that spawns a specific item.
-- @param item_id string  Key matching loot entry item_id
-- @param fn     function  Function receiving position {x, y, z} -> entity_id
local function register_spawner(item_id, fn)
	spawners[item_id] = fn
end

--- Define a named loot table.
-- @param name    string   Unique table identifier
-- @param entries table[]  Array of entry tables
function define_table(name, entries)
	if type(name) ~= "string" then
		error("table name must be a string, got " .. type(name))
	end
	if type(entries) ~= "table" or #entries == 0 then
		error("entries must be a non-empty table")
	end
	for i, entry in ipairs(entries) do
		validate_entry(entry, i)
	end
	local total_weight = 0
	for _, entry in ipairs(entries) do
		total_weight = total_weight + entry.weight
	end
	tables[name] = {
		entries = entries,
		total_weight = total_weight,
	}
end

--- Roll on a named loot table at a position.
-- @param table_name string     Name of previously defined table
-- @param position   table      {x, y} or {x, y, z} coordinates
-- @param entity_id  number|nil Optional entity ID for condition evaluation
-- @return number[]             Array of spawned entity IDs (empty if no drops)
function roll(table_name, position, entity_id)
	if type(table_name) ~= "string" then
		error("table_name must be a string, got " .. type(table_name))
	end
	if not position or position.x == nil or position.y == nil then
		error("position must have x and y keys")
	end
	local table_def = tables[table_name]
	if not table_def or table_def.total_weight <= 0 then
		return {}
	end
	local z = position.z or 0
	-- Filter entries by condition (if entity_id provided)
	local eligible = {}
	for _, entry in ipairs(table_def.entries) do
		if entity_id ~= nil and entry.condition ~= nil then
			local ok = entry.condition(entity_id)
			if ok then
				table.insert(eligible, entry)
			end
		else
			table.insert(eligible, entry)
		end
	end
	if #eligible == 0 then
		return {}
	end
	-- Compute total weight of eligible entries
	local total_weight = 0
	for _, entry in ipairs(eligible) do
		total_weight = total_weight + entry.weight
	end
	if total_weight <= 0 then
		return {}
	end
	-- Weighted random selection
	local r = math.random(1, total_weight)
	local cumulative = 0
	local selected = nil
	for _, entry in ipairs(eligible) do
		cumulative = cumulative + entry.weight
		if r <= cumulative then
			selected = entry
			break
		end
	end
	if not selected then
		return {}
	end
	-- Determine quantity
	local min_count = selected.min_count or 1
	local max_count = selected.max_count or 1
	local count = math.random(min_count, max_count)
	-- Spawn items
	local results = {}
	local spawner = spawners[selected.item_id]
	for _ = 1, count do
		local eid
		if spawner then
			eid = spawner({ x = position.x, y = position.y, z = z })
		else
			local meta = lookup_metadata(selected.item_id)
			eid = spawn_entity()
			if eid then
				set_component(eid, "Type", { kind = "item" })
				set_component(eid, "Position", { pos = { Square = { x = position.x, y = position.y, z = z } } })
				set_component(eid, "Renderable", { glyph = meta.glyph, color = meta.color })
				set_component(eid, "Item", { id = selected.item_id, name = meta.name, slot = "none" })
			end
		end
		if eid then
			table.insert(results, eid)
		end
	end
	return results
end

--- Convenience: roll loot for a dying entity.
-- Reads components to determine table name and position.
-- @param entity_id number  Entity ID of dying entity
-- @return number[]         Array of spawned entity IDs (empty if no drops)
function spawn_for_death(entity_id)
	if entity_id == nil or type(entity_id) ~= "number" then
		error("entity_id must be a number, got " .. type(entity_id))
	end
	-- Check if alive
	local hp = get_component(entity_id, "Health")
	if hp and hp.current ~= nil and hp.current > 0 then
		return {}
	end
	-- Read position
	local pos = get_component(entity_id, "Position")
	if not pos or not pos.pos or not pos.pos.Square then
		return {}
	end
	local px = pos.pos.Square.x
	local py = pos.pos.Square.y
	local pz = pos.pos.Square.z or 0
	-- Read type and map to table name
	local typ = get_component(entity_id, "Type")
	local kind = (typ and typ.kind) or "enemy"
	local kind_to_table = {
		enemy = "enemy",
		player = "player",
	}
	local table_name = kind_to_table[kind]
	if not table_name then
		return {}
	end
	return roll(table_name, { x = px, y = py, z = pz }, entity_id)
end

-- Register default item spawners

register_spawner("health_potion", function(pos)
	local eid = spawn_entity()
	if not eid then
		return nil
	end
	set_component(eid, "Type", { kind = "item" })
	set_component(eid, "Position", { pos = { Square = { x = pos.x, y = pos.y, z = pos.z or 0 } } })
	set_component(eid, "Renderable", { glyph = "!", color = { 0, 255, 0 } })
	set_component(eid, "Item", { id = "health_potion", name = "Health Potion", slot = "none" })
	return eid
end)

register_spawner("rusty_sword", function(pos)
	local eid = spawn_entity()
	if not eid then
		return nil
	end
	set_component(eid, "Type", { kind = "item" })
	set_component(eid, "Position", { pos = { Square = { x = pos.x, y = pos.y, z = pos.z or 0 } } })
	set_component(eid, "Renderable", { glyph = "/", color = { 200, 200, 200 } })
	set_component(eid, "Item", { id = "rusty_sword", name = "Rusty Sword", slot = "weapon" })
	return eid
end)

return {
	define_table = define_table,
	roll = roll,
	spawn_for_death = spawn_for_death,
	register_spawner = register_spawner,
}
