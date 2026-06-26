-- MGE Roguelike Demo — MVP
--
-- Cell coordinate formats used:
--   entities_in_cell(cell)    -> { Square = { x, y, z } }  (table with Square wrapper)
--   add_neighbor(from, to)    -> { x, y, z }               (flat table)
--   find_path(start, goal)    -> { .path = { { x, y, z }, ... }, .total_cost }
--   set_cell_metadata(cell,_) -> { x, y, z }               (flat table)
--   get_cell_metadata(cell)   -> { x, y, z }               (flat table)

-- SECTION 1: Config
COLOR_YELLOW = { 255, 255, 0 }
COLOR_RED = { 255, 0, 0 }
COLOR_GREEN = { 0, 255, 0 }
COLOR_WHITE = { 200, 200, 200 }
COLOR_DGRAY = { 80, 80, 80 }
COLOR_CYAN = { 0, 200, 200 }
COLOR_ORANGE = { 255, 165, 0 }
COLOR_BROWN = { 139, 69, 19 }
MAP_W = 40
MAP_H = 25
VIEW_W = 20
VIEW_H = 12

-- SECTION 2: Coordinate Helpers
function sq_cell(x, y)
	return { Square = { x = x, y = y, z = 0 } }
end

function flat_cell(x, y)
	return { x = x, y = y, z = 0 }
end

function make_pos(x, y, z)
	return { pos = { Square = { x = x, y = y, z = z or 0 } } }
end

function get_xy(eid)
	local pos = get_component(eid, "Position")
	if pos and pos.pos and pos.pos.Square then
		return pos.pos.Square.x, pos.pos.Square.y
	end
	return nil, nil
end

-- SECTION 3: Map Generation
local wall_data = {}

function build_grid_map(w, h)
	for x = 0, w - 1 do
		for y = 0, h - 1 do
			add_cell(x, y, 0)
		end
	end
	for x = 0, w - 1 do
		for y = 0, h - 1 do
			for _, d in ipairs({ { 1, 0 }, { 0, 1 }, { -1, 0 }, { 0, -1 } }) do
				local nx, ny = x + d[1], y + d[2]
				if nx >= 0 and nx < w and ny >= 0 and ny < h then
					add_neighbor(flat_cell(x, y), flat_cell(nx, ny))
				end
			end
		end
	end
end

function set_wall(x, y)
	wall_data[wall_key(x, y)] = true
end

function wall_key(x, y)
	return x .. "," .. y
end

function is_wall(x, y)
	return wall_data[wall_key(x, y)] == true
end

function carve_room(x1, y1, x2, y2, doors)
	for x = x1, x2 do
		set_wall(x, y1)
		set_wall(x, y2)
	end
	for y = y1 + 1, y2 - 1 do
		set_wall(x1, y)
		set_wall(x2, y)
	end
	for _, d in ipairs(doors) do
		wall_data[wall_key(d[1], d[2])] = nil
	end
end

function register_walls_with_pathfinder()
	for key, _ in pairs(wall_data) do
		local comma = key:find(",")
		if comma then
			local x = tonumber(key:sub(1, comma - 1))
			local y = tonumber(key:sub(comma + 1))
			if x and y then
				set_cell_metadata(flat_cell(x, y), { walkable = false })
			end
		end
	end
end

local function create_map()
	world:apply_generated_map({ topology = "square", cells = { { x = 0, y = 0, z = 0 } } })
	build_grid_map(MAP_W, MAP_H)

	for x = 0, MAP_W - 1 do
		set_wall(x, 0)
		set_wall(x, MAP_H - 1)
	end
	for y = 0, MAP_H - 1 do
		set_wall(0, y)
		set_wall(MAP_W - 1, y)
	end

	carve_room(2, 2, 10, 7, { { 10, 5 }, { 6, 7 } })
	carve_room(14, 2, 24, 7, { { 14, 5 }, { 24, 5 }, { 19, 7 } })
	carve_room(28, 2, 37, 9, { { 28, 5 }, { 32, 9 } })
	carve_room(2, 12, 10, 21, { { 6, 12 }, { 10, 16 } })
	carve_room(14, 12, 24, 21, { { 19, 12 }, { 14, 16 }, { 24, 16 } })
	carve_room(28, 12, 37, 21, { { 32, 12 }, { 28, 16 } })

	for x = 11, 13 do
		wall_data[wall_key(x, 5)] = nil
	end
	for x = 25, 27 do
		wall_data[wall_key(x, 5)] = nil
	end
	for y = 8, 11 do
		wall_data[wall_key(6, y)] = nil
	end
	for y = 8, 11 do
		wall_data[wall_key(19, y)] = nil
	end
	for y = 10, 11 do
		wall_data[wall_key(32, y)] = nil
	end
	for x = 11, 13 do
		wall_data[wall_key(x, 16)] = nil
	end
	for x = 25, 27 do
		wall_data[wall_key(x, 16)] = nil
	end

	register_walls_with_pathfinder()
end

-- SECTION 4: Entity Factories
function spawn_player(x, y)
	local eid = spawn_entity()
	set_component(eid, "Type", { kind = "player" })
	set_component(eid, "Position", make_pos(x, y, 0))
	set_component(eid, "Health", { current = 10, max = 10 })
	set_component(eid, "Renderable", { glyph = "@", color = COLOR_YELLOW })
	return eid
end

function spawn_enemy(x, y)
	local eid = spawn_entity()
	set_component(eid, "Type", { kind = "enemy" })
	set_component(eid, "Position", make_pos(x, y, 0))
	set_component(eid, "Health", { current = 3, max = 3 })
	set_component(eid, "Renderable", { glyph = "g", color = COLOR_RED })
	return eid
end

function spawn_item(item_id, item_name, glyph, color, x, y)
	local eid = spawn_entity()
	set_component(eid, "Type", { kind = "item" })
	set_component(eid, "Position", make_pos(x, y, 0))
	set_component(eid, "Renderable", { glyph = glyph, color = color })
	set_component(eid, "Item", { id = item_id, name = item_name, slot = "none" })
	return eid
end

function spawn_health_potion_entity(x, y)
	local eid = spawn_entity()
	set_component(eid, "Type", { kind = "item" })
	set_component(eid, "Position", make_pos(x, y, 0))
	set_component(eid, "Renderable", { glyph = "!", color = COLOR_GREEN })
	set_component(eid, "Item", { id = "health_potion", name = "Health Potion", slot = "none" })
	return eid
end

-- SECTION 5: Game State
player = nil
enemies = {}
items = {}
message_log = {}
game_state = "play"
save_slots = {}

function add_message(msg)
	table.insert(message_log, msg)
	if #message_log > 6 then
		table.remove(message_log, 1)
	end
end

function find_entity_by_kind(kind)
	for _, eid in ipairs(get_entities_with_component("Type")) do
		local t = get_component(eid, "Type")
		if t and t.kind == kind and is_entity_alive(eid) then
			return eid
		end
	end
	return nil
end

function collect_entities_by_kind(kind)
	local result = {}
	for _, eid in ipairs(get_entities_with_component("Type")) do
		local t = get_component(eid, "Type")
		if t and t.kind == kind and is_entity_alive(eid) then
			table.insert(result, eid)
		end
	end
	return result
end

-- SECTION 6: Rendering
function get_tile_glyph(mx, my)
	local cell = sq_cell(mx, my)
	local occupants = entities_in_cell(cell)
	local corpse_glyph = nil
	local item_glyph = nil
	local item_color = nil
	local alive_glyph = nil
	local alive_color = nil
	for _, eid in ipairs(occupants) do
		local rend = get_component(eid, "Renderable")
		if rend and is_entity_alive(eid) then
			alive_glyph = rend.glyph
			alive_color = rend.color
		end
		local item = get_component(eid, "Item")
		if item then
			if item.id == "health_potion" or item.id == "potion" then
				item_glyph = "!"
				item_color = COLOR_GREEN
			else
				item_glyph = "?"
				item_color = COLOR_WHITE
			end
		end
		local corpse = get_component(eid, "Corpse")
		if corpse then
			corpse_glyph = "%"
		end
	end
	if alive_glyph then
		return alive_glyph, alive_color
	end
	if item_glyph then
		return item_glyph, item_color
	end
	if corpse_glyph then
		return "%", COLOR_DGRAY
	end
	if is_wall(mx, my) then
		return "#", COLOR_DGRAY
	end
	return ".", COLOR_WHITE
end

function render_viewport()
	local cam = get_camera()
	local sx = math.max(0, math.min(cam.x - math.floor(VIEW_W / 2), MAP_W - VIEW_W))
	local sy = math.max(0, math.min(cam.y - math.floor(VIEW_H / 2), MAP_H - VIEW_H))
	for vy = 0, VIEW_H - 1 do
		local row = ""
		for vx = 0, VIEW_W - 1 do
			local glyph, _ = get_tile_glyph(sx + vx, sy + vy)
			row = row .. glyph
		end
		print(row)
	end
end

function render_hud()
	if not player then
		return
	end
	local hp = get_component(player, "Health")
	if not hp then
		return
	end
	local inv = get_inventory(player)
	local inv_count = inv and inv.slots and #inv.slots or 0
	local max_slots = inv and inv.max_slots or 10
	print("---")
	print("Turn: " .. get_turn() .. "  HP: " .. math.floor(hp.current) .. "/" .. math.floor(hp.max) .. "  Items: " .. inv_count .. "/" .. max_slots .. "  Loot: " .. loot_count)
end

function render_log()
	if #message_log == 0 then
		return
	end
	print("-- Log --")
	for i = math.max(1, #message_log - 5), #message_log do
		print(message_log[i])
	end
end

-- SECTION 7: Player Actions
function is_walkable(x, y)
	if x < 0 or x >= MAP_W or y < 0 or y >= MAP_H then
		return false
	end
	return not is_wall(x, y)
end

function is_occupied(x, y)
	local cell = sq_cell(x, y)
	for _, eid in ipairs(entities_in_cell(cell)) do
		local t = get_component(eid, "Type")
		if t and is_entity_alive(eid) then
			return true, eid
		end
	end
	return false, nil
end

function find_adjacent_free_cell(x, y)
	local dirs = { { 1, 0 }, { -1, 0 }, { 0, 1 }, { 0, -1 } }
	for _, d in ipairs(dirs) do
		local nx, ny = x + d[1], y + d[2]
		if nx >= 0 and nx < MAP_W and ny >= 0 and ny < MAP_H and is_walkable(nx, ny) then
			local occ = is_occupied(nx, ny)
			if not occ then
				return nx, ny
			end
		end
	end
	return nil, nil
end

function move_player(dx, dy)
	local px, py = get_xy(player)
	if not px then
		return false
	end
	local nx, ny = px + dx, py + dy
	if not is_walkable(nx, ny) then
		add_message("You bump into a wall.")
		return false
	end
	local occ, occ_eid = is_occupied(nx, ny)
	if occ then
		if occ_eid then
			local t = get_component(occ_eid, "Type")
			if t and t.kind == "enemy" then
				attack_entity(player, occ_eid, 2)
				return true
			end
		end
		add_message("Something blocks your path.")
		return false
	end
	set_component(player, "Position", make_pos(nx, ny, 0))
	return true
end

function pickup_item()
	local px, py = get_xy(player)
	if not px then
		return true
	end
	local cell = sq_cell(px, py)
	local occupants = entities_in_cell(cell)
	for _, eid in ipairs(occupants) do
		local item = get_component(eid, "Item")
		if item then
			local inv = get_inventory(player)
			if inv and inv.max_slots and #inv.slots >= inv.max_slots then
				add_message("Inventory full.")
				return true
			end
			add_item_to_inventory(player, item.id)
			despawn_entity(eid)
			for i, ieid in ipairs(items) do
				if ieid == eid then
					table.remove(items, i)
					break
				end
			end
			add_message("You pick up " .. item.name .. ".")
			return true
		end
	end
	add_message("Nothing to pick up here.")
	return true
end

function use_item()
	local inv = get_inventory(player)
	if not inv or #inv.slots == 0 then
		add_message("No items to use.")
		return true
	end
	for idx, item_id in ipairs(inv.slots) do
		if item_id == "health_potion" or item_id == "potion" then
			local hp = get_component(player, "Health")
			hp.current = math.min(hp.current + 5, hp.max)
			set_component(player, "Health", hp)
			remove_item_from_inventory(player, idx - 1)
			add_message("You drink a potion and heal 5 HP!")
			return true
		end
	end
	add_message("No usable items.")
	return true
end

function drop_item()
	local inv = get_inventory(player)
	if not inv or #inv.slots == 0 then
		add_message("Nothing to drop.")
		return true
	end
	local px, py = get_xy(player)
	if not px then
		return true
	end
	local drop_x, drop_y = find_adjacent_free_cell(px, py)
	if not drop_x then
		add_message("No space to drop items.")
		return true
	end
	local last_idx = #inv.slots
	local item_id = inv.slots[last_idx]
	remove_item_from_inventory(player, last_idx - 1)
	local eid = spawn_item(item_id, item_id, "?", COLOR_WHITE, drop_x, drop_y)
	table.insert(items, eid)
	add_message("You drop " .. item_id .. ".")
	return true
end

function handle_player_action(cmd)
	-- Movement (return true = advance turn)
	if cmd == "h" or cmd == "left" or cmd == "a" then
		return move_player(-1, 0)
	elseif cmd == "j" or cmd == "down" or cmd == "s" then
		return move_player(0, 1)
	elseif cmd == "k" or cmd == "up" or cmd == "w" then
		return move_player(0, -1)
	elseif cmd == "l" or cmd == "right" or cmd == "d" then
		return move_player(1, 0)
	elseif cmd == "." then
		-- Wait: advance turn without action
		return true
	elseif cmd == "g" or cmd == "e" then
		return pickup_item()
	elseif cmd == "u" or cmd == "q" then
		return use_item()
	elseif cmd == "d" then
		return drop_item()
	end
	return false
end

-- SECTION 8: Enemy AI
function process_enemy_turn()
	local px, py = get_xy(player)
	if not px then
		return
	end
	for _, eid in ipairs(enemies) do
		if is_entity_alive(eid) then
			local ex, ey = get_xy(eid)
			if ex then
				local dist = math.abs(px - ex) + math.abs(py - ey)
				if dist == 1 then
					attack_entity(eid, player, 1)
				elseif dist > 1 and dist <= 5 then
					local result = find_path(flat_cell(ex, ey), flat_cell(px, py))
					if result and result.path and #result.path >= 2 then
						local next_step = result.path[2]
						if next_step then
							local nx, ny = next_step.Square.x, next_step.Square.y
							if nx == px and ny == py then
								attack_entity(eid, player, 1)
							else
								local cell = sq_cell(nx, ny)
								local occupants = entities_in_cell(cell)
								local blocked = false
								for _, occ in ipairs(occupants) do
									if occ ~= eid and occ ~= player and is_entity_alive(occ) then
										blocked = true
										break
									end
								end
								if not blocked then
									set_component(eid, "Position", make_pos(nx, ny, 0))
								end
							end
						end
					end
				else
					local dirs = { {0,-1}, {0,1}, {-1,0}, {1,0} }
					local r = math.random(1, 4)
					local nx, ny = ex + dirs[r][1], ey + dirs[r][2]
					if is_walkable(nx, ny) then
						local cell = sq_cell(nx, ny)
						local occupants = entities_in_cell(cell)
						local blocked = false
						for _, occ in ipairs(occupants) do
							if occ ~= eid and is_entity_alive(occ) then
								blocked = true
								break
							end
						end
						if not blocked then
							set_component(eid, "Position", make_pos(nx, ny, 0))
						end
					end
				end
			end
		end
	end
end

-- SECTION 9: Combat System
function attack_entity(attacker, target, damage)
	local hp = get_component(target, "Health")
	local prev = hp.current
	hp.current = math.max(0, hp.current - damage)
	set_component(target, "Health", hp)
	local dealt = prev - hp.current
	if dealt <= 0 then
		return
	end
	local a_type = get_component(attacker, "Type")
	local t_type = get_component(target, "Type")
	local a_name = (a_type and a_type.kind == "player") and "Player" or "Goblin"
	local t_name = (t_type and t_type.kind == "player") and "Player" or "Goblin"
	send_event("combat", '{"attacker_id":' .. attacker .. ',"target_id":' .. target .. ',"damage":' .. dealt .. ',"message":"' .. a_name .. " hits " .. t_name .. " for " .. dealt .. ' damage!"}')
	if hp.current <= 0 then
		send_event("death", '{"entity_id":' .. target .. ',"message":"' .. t_name .. ' dies!"}')
	end
end

function check_win_lose()
	if not is_entity_alive(player) then
		add_message("You have died. Game over!")
		render_viewport()
		render_hud()
		render_log()
		print("You have died. Game over!")
		return true
	end
	for _, eid in ipairs(enemies) do
		if is_entity_alive(eid) then
			return false
		end
	end
	add_message("You defeated all enemies! You win!")
	render_viewport()
	render_hud()
	render_log()
	print("You defeated all enemies! You win!")
	return true
end

-- SECTION 10: Inventory Screen
function show_inventory_screen()
	local selected = 1
	while true do
		print("\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n")
		print("=== INVENTORY ===\n")
		local inv = get_inventory(player)
		if not inv or #inv.slots == 0 then
			print("  (empty)")
		else
			if selected > #inv.slots then
				selected = #inv.slots
			end
			for i, item_id in ipairs(inv.slots) do
				local marker = (i == selected) and " >" or "  "
				print(marker .. " " .. i .. ". " .. item_id)
			end
		end
		print("")
		print("[u] use item  [d] drop item  [i] close")
		local cmd = get_user_input("> ")
		if cmd == "i" then
			return
		elseif cmd == "u" and inv and #inv.slots > 0 then
			local item_id = inv.slots[selected]
			if item_id == "health_potion" or item_id == "potion" then
				local hp = get_component(player, "Health")
				hp.current = math.min(hp.current + 5, hp.max)
				set_component(player, "Health", hp)
				remove_item_from_inventory(player, selected - 1)
				add_message("You drink a potion and heal 5 HP!")
				inv = get_inventory(player)
				if not inv or #inv.slots == 0 then
					return
				end
			else
				add_message("Cannot use that item.")
			end
		elseif cmd == "d" and inv and #inv.slots > 0 then
			local item_id = inv.slots[selected]
			local px, py = get_xy(player)
			if px then
				local drop_x, drop_y = find_adjacent_free_cell(px, py)
				if drop_x then
					remove_item_from_inventory(player, selected - 1)
					local eid = spawn_item(item_id, item_id, "?", COLOR_WHITE, drop_x, drop_y)
					table.insert(items, eid)
					add_message("You drop " .. item_id .. ".")
					inv = get_inventory(player)
					if not inv or #inv.slots == 0 then
						return
					end
				else
					add_message("No space to drop items.")
				end
			end
		elseif cmd >= "1" and cmd <= "9" then
			local n = tonumber(cmd)
			if n and inv and n <= #inv.slots then
				selected = n
			end
		end
	end
end

-- SECTION 11: Save/Load Slots
function list_save_slots()
	local lines = {}
	for i = 1, 4 do
		local slot = save_slots[i]
		if slot then
			table.insert(lines, tostring(i) .. ". Slot " .. i .. " — Turn " .. slot.turn .. ", HP " .. slot.hp)
		else
			table.insert(lines, tostring(i) .. ". Slot " .. i .. " — (empty)")
		end
	end
	return lines
end

function show_save_menu()
	print("\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n")
	print("=== SAVE GAME ===\n")
	local lines = list_save_slots()
	for _, line in ipairs(lines) do
		print(line)
	end
	print("q — Cancel")
	local cmd = get_user_input("Select slot (1-4): ")
	if cmd == "q" or cmd == "Q" then
		add_message("Save cancelled.")
		return
	end
	local n = tonumber(cmd)
	if n and n >= 1 and n <= 4 then
		local filename = "mge_demo_save_" .. n .. ".json"
		local ok, err = pcall(function()
			save_to_file(filename)
		end)
		if ok then
			local hp = get_component(player, "Health")
			local hp_str = "?"
			if hp then
				hp_str = math.floor(hp.current) .. "/" .. math.floor(hp.max)
			end
			save_slots[n] = { turn = get_turn(), hp = hp_str }
			add_message("Game saved to slot " .. n .. "!")
		else
			add_message("Failed to save: " .. tostring(err))
		end
	else
		add_message("Invalid slot.")
	end
end

function show_load_menu()
	print("\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n")
	print("=== LOAD GAME ===\n")
	local lines = list_save_slots()
	for _, line in ipairs(lines) do
		print(line)
	end
	print("q — Cancel")
	local cmd = get_user_input("Select slot (1-4): ")
	if cmd == "q" or cmd == "Q" then
		add_message("Load cancelled.")
		return
	end
	local n = tonumber(cmd)
	if n and n >= 1 and n <= 4 then
		if not save_slots[n] then
			add_message("Slot " .. n .. " is empty.")
			return
		end
		local confirm = get_user_input("Load slot " .. n .. "? (y/n): ")
		if confirm == "y" or confirm == "Y" then
			local filename = "mge_demo_save_" .. n .. ".json"
			local ok, err = pcall(function()
				load_from_file(filename)
			end)
			if ok then
				player = find_entity_by_kind("player")
				enemies = collect_entities_by_kind("enemy")
				items = {}
				for _, eid in ipairs(get_entities_with_component("Item")) do
					table.insert(items, eid)
				end
				if player then
					local px, py = get_xy(player)
					if px then
						set_camera(px, py)
					end
					add_message("Game loaded from slot " .. n .. "!")
				else
					add_message("Save corrupted: no player entity.")
				end
			else
				add_message("Failed to load: " .. tostring(err))
			end
		else
			add_message("Load cancelled.")
		end
	else
		add_message("Invalid slot.")
	end
end

-- SECTION 12: Loot Tables (Rust-native API)
-- Item metadata for loot drops: maps item_id to rendering info
local ITEM_LOOKUP = {
	health_potion = { glyph = "!", color = COLOR_GREEN, name = "Health Potion" },
	rusty_sword = { glyph = "/", color = COLOR_WHITE, name = "Rusty Sword" },
}

local function item_meta(item_id)
	local m = ITEM_LOOKUP[item_id]
	if m then return m end
	return { glyph = "?", color = COLOR_WHITE, name = item_id }
end

-- Spawn items from loot table results at a dead entity's position
local function spawn_loot_at(death_eid)
	local hp = get_component(death_eid, "Health")
	if hp and hp.current and hp.current > 0 then return end
	local pos = get_component(death_eid, "Position")
	if not pos or not pos.pos or not pos.pos.Square then return end
	local results = roll_loot_table("enemy")
	if not results or #results == 0 then return end
	for _, drop in ipairs(results) do
		local meta = item_meta(drop.item_id)
		for _ = 1, drop.count do
			local eid = spawn_item(drop.item_id, meta.name, meta.glyph, meta.color,
				pos.pos.Square.x, pos.pos.Square.y)
			if eid then
				table.insert(items, eid)
				loot_count = loot_count + 1
			end
		end
	end
end

loot_count = 0

-- Goblin death drops: health potion (weight 80), rusty sword (weight 20)
define_loot_table("enemy", {
	{ item_id = "health_potion", weight = 80, min_count = 1, max_count = 1 },
	{ item_id = "rusty_sword", weight = 20, min_count = 1, max_count = 1 },
})

-- SECTION 13: Game Loop
function main()
	create_map()

	player = spawn_player(5, 5)

	local enemy_positions = {
		{ 16, 4 },
		{ 35, 4 },
		{ 5, 16 },
		{ 30, 16 },
		{ 22, 16 },
	}
	for _, pos in ipairs(enemy_positions) do
		table.insert(enemies, spawn_enemy(pos[1], pos[2]))
	end

	table.insert(items, spawn_item("health_potion", "Health Potion", "!", COLOR_GREEN, 3, 5))
	table.insert(items, spawn_item("health_potion", "Health Potion", "!", COLOR_GREEN, 30, 4))
	table.insert(items, spawn_item("health_potion", "Health Potion", "!", COLOR_GREEN, 8, 14))

	set_camera(5, 5)
	set_inventory(player, {
		slots = setmetatable({}, { __is_array = true }),
		max_slots = 10,
		weight = 0.0,
		volume = 0.0,
	})

	add_message("Welcome to the MGE Roguelike!")
	add_message("Explore the dungeon and defeat all enemies.")

	game_state = "play"

	while true do
		if game_state == "play" then
			render_viewport()
			render_hud()
			render_log()
			print("[WASD/hjkl=move .=wait e/g=get q/u=use d=drop i=inv S=save L=load Q=quit]")

			local cmd = get_user_input("> ")
			if cmd == "Q" then
				add_message("Goodbye!")
				break
			elseif cmd == "S" then
				show_save_menu()
			elseif cmd == "L" then
				show_load_menu()
			elseif cmd == "i" then
				show_inventory_screen()
			else
				local acted = handle_player_action(cmd)

				if acted then
					tick()
					process_enemy_turn()

					for _, ev in ipairs(poll_event("combat")) do
						if ev.message then
							add_message(ev.message)
						end
					end
					for _, ev in ipairs(poll_event("death")) do
						if ev.message then
							add_message(ev.message)
						end
						-- Spawn loot at dead entity's position
						if ev.entity_id then
							spawn_loot_at(ev.entity_id)
						end
					end

					process_deaths()
					process_decay()

					local px, py = get_xy(player)
					if px then
						set_camera(px, py)
					end

					if check_win_lose() then
						break
					end
				end
			end
		end
	end
end

main()
