-- MVP Roguelike main system for MGE

function get_player_xy()
	local player = get_player_eid()
	local pos = get_component(player, "Position")
	return get_square_xy(pos)
end

function is_adjacent(eid1, eid2)
	local x1, y1 = get_square_xy(get_component(eid1, "Position"))
	local x2, y2 = get_square_xy(get_component(eid2, "Position"))
	return math.abs(x1 - x2) + math.abs(y1 - y2) == 1
end

function get_monster_eids()
	local monsters = {}
	for _, eid in ipairs(get_entities()) do
		if get_component(eid, "Monster") then
			table.insert(monsters, eid)
		end
	end
	return monsters
end

function make_square_pos(x, y, z)
	return { pos = { Square = { x = x, y = y, z = z or 0 } } }
end

function get_square_xy(pos)
	if pos and pos.pos and pos.pos.Square then
		return pos.pos.Square.x, pos.pos.Square.y
	end
	return nil, nil
end

function monster_turn(player, map)
	local px, py = get_square_xy(get_component(player, "Position"))
	for _, mid in ipairs(get_monster_eids()) do
		if is_entity_alive(mid) then
			local mpos = get_component(mid, "Position")
			local mx, my = get_square_xy(mpos)
			if mx and my then
				local dx = px - mx
				local dy = py - my
				if math.abs(dx) + math.abs(dy) == 1 then
					print("The monster attacks you!")
					local php = get_component(player, "Health")
					php.current = math.max(0, php.current - 1)
					set_component(player, "Health", php)
				else
					-- Move towards player (simple AI)
					local step_x = dx ~= 0 and (dx > 0 and 1 or -1) or 0
					local step_y = (dx == 0 and dy ~= 0) and (dy > 0 and 1 or -1) or 0
					try_move(mid, step_x, step_y, map)
				end
			end
		end
	end
end

function spawn_entities_from_map(map)
	for y, row in ipairs(map.tiles) do
		local new_row = row
		for x = 1, #row do
			local ch = row:sub(x, x)
			local pos = make_square_pos(x - 1, y - 1)
			if ch == "@" then
				local eid = spawn_entity()
				set_component(eid, "Player", { name = "Hero" })
				set_component(eid, "Renderable", { glyph = "@", color = "yellow" })
				set_component(eid, "Position", pos)
				set_component(eid, "Health", { current = 10, max = 10 })
				new_row = new_row:sub(1, x - 1) .. "." .. new_row:sub(x + 1)
			elseif ch == "M" then
				local eid = spawn_entity()
				set_component(eid, "Monster", { name = "Goblin", ai = "basic" })
				set_component(eid, "Renderable", { glyph = "M", color = "red" })
				set_component(eid, "Position", pos)
				set_component(eid, "Health", { current = 5, max = 5 })
				new_row = new_row:sub(1, x - 1) .. "." .. new_row:sub(x + 1)
			elseif ch == "!" then
				local eid = spawn_entity()
				set_component(eid, "Item", { id = "potion1", name = "Potion", effect = "heal" })
				set_component(eid, "Renderable", { glyph = "!", color = "green" })
				set_component(eid, "Position", pos)
				new_row = new_row:sub(1, x - 1) .. "." .. new_row:sub(x + 1)
			end
		end
		map.tiles[y] = new_row
	end
end

function render(map)
	local glyph_map = {}
	for y = 1, map.height do
		glyph_map[y] = {}
		for x = 1, map.width do
			local ch = map.tiles[y]:sub(x, x)
			glyph_map[y][x] = ch
		end
	end
	for _, eid in ipairs(get_entities()) do
		local rend = get_component(eid, "Renderable")
		local pos = get_component(eid, "Position")
		local px, py = get_square_xy(pos)
		if px and py and rend then
			-- Only render alive monsters/players, always render items
			if get_component(eid, "Monster") or get_component(eid, "Player") then
				if is_entity_alive(eid) then
					glyph_map[py + 1][px + 1] = rend.glyph
				end
			else
				glyph_map[py + 1][px + 1] = rend.glyph
			end
		end
	end
	for y = 1, map.height do
		local row = ""
		for x = 1, map.width do
			row = row .. glyph_map[y][x]
		end
		print(row)
	end
end

function get_player_eid()
	for _, eid in ipairs(get_entities()) do
		if get_component(eid, "Player") then
			return eid
		end
	end
	return nil
end

function try_move(eid, dx, dy, map)
	local pos = get_component(eid, "Position")
	local px, py = get_square_xy(pos)
	if px == nil or py == nil then
		return
	end
	local nx, ny = px + dx, py + dy
	local tile = map.tiles[ny + 1]:sub(nx + 1, nx + 1)
	if tile == "#" then
		return
	end
	-- First, check for monsters (player attacks, monsters can't attack monsters)
	for _, other in ipairs(get_entities()) do
		if other ~= eid and is_entity_alive(other) then
			local op = get_component(other, "Position")
			local ox, oy = get_square_xy(op)
			if ox == nx and oy == ny then
				if get_component(other, "Monster") and get_component(eid, "Player") then
					local hp = get_component(other, "Health")
					hp.current = math.max(0, hp.current - 3)
					set_component(other, "Health", hp)
					print("You hit the monster!")
					if hp.current <= 0 then
						despawn_entity(other)
						print("Monster dies!")
					end
					return
				end
			end
		end
	end
	-- Only player can pick up potions
	if get_component(eid, "Player") then
		for _, other in ipairs(get_entities()) do
			if other ~= eid and get_component(other, "Item") then
				local op = get_component(other, "Position")
				local ox, oy = get_square_xy(op)
				if ox == nx and oy == ny then
					print("You pick up a potion and heal 5 HP!")
					local php = get_component(eid, "Health")
					php.current = math.min(php.current + 5, php.max)
					set_component(eid, "Health", php)
					despawn_entity(other)
					return
				end
			end
		end
	end
	set_component(eid, "Position", make_square_pos(nx, ny))
end

function all_monsters_dead()
	for _, eid in ipairs(get_monster_eids()) do
		if is_entity_alive(eid) then
			return false
		end
	end
	return true
end

function attack_adjacent_monster(player)
	for _, mid in ipairs(get_monster_eids()) do
		if is_entity_alive(mid) and is_adjacent(player, mid) then
			local hp = get_component(mid, "Health")
			hp.current = math.max(0, hp.current - 3)
			set_component(mid, "Health", hp)
			print("You attack the monster!")
			if hp.current <= 0 then
				despawn_entity(mid)
				print("Monster dies!")
			end
			return true
		end
	end
	print("No adjacent monster to attack.")
	return false
end

function main()
	local map = require_json("mods/mvp_roguelike/assets/map1.json")
	spawn_entities_from_map(map)
	local player = get_player_eid()
	local pos = get_component(player, "Position")
	local px, py = get_square_xy(pos)
	if px and py then
		set_camera(px, py)
	else
		error("Player position is missing x/y")
	end

	while true do
		player = get_player_eid()
		if not player or not is_entity_alive(player) then
			print("You have died. Game over!")
			break
		end

		render(map)
		local hp = get_component(player, "Health")
		print("HP: " .. hp.current .. "/" .. hp.max)
		print("Move with WASD, e to attack, . to wait, q to quit.")
		local cmd = get_user_input(">")
		if cmd == "q" then
			print("Game over!")
			break
		end
		local acted = false
		local dx, dy = 0, 0
		if cmd == "w" then
			dy = -1
			acted = true
		elseif cmd == "s" then
			dy = 1
			acted = true
		elseif cmd == "a" then
			dx = -1
			acted = true
		elseif cmd == "d" then
			dx = 1
			acted = true
		elseif cmd == "e" then
			acted = attack_adjacent_monster(player)
		elseif cmd == "." then
			acted = true -- skip turn
		end
		if acted then
			if dx ~= 0 or dy ~= 0 then
				try_move(player, dx, dy, map)
				local pos = get_component(player, "Position")
				local px, py = get_square_xy(pos)
				if px and py then
					set_camera(px, py)
				else
					error("Player position is missing x/y")
				end
			end
			monster_turn(player, map)
		end

		if not is_entity_alive(player) then
			print("You have died. Game over!")
			break
		end
		if all_monsters_dead() then
			print("You defeated all monsters! You win!")
			break
		end
	end
end

main()
