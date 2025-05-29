-- Roguelike MVP Demo (interactive)
-- Controls: w/a/s/d = move, e = attack, q = quit

-- Setup player
local player = spawn_entity()
set_component(player, "Type", { kind = "player" })
set_component(player, "Position", { pos = { Square = { x = 0, y = 0, z = 0 } } })
set_component(player, "Health", { current = 10, max = 10 })

-- Setup enemies
local enemies = {}
for i = 1, 3 do
	local enemy = spawn_entity()
	set_component(enemy, "Type", { kind = "enemy" })
	set_component(enemy, "Position", { pos = { Square = { x = i * 2, y = 1, z = 0 } } })
	set_component(enemy, "Health", { current = 3, max = 3 })
	table.insert(enemies, enemy)
end

local directions = { w = { 0, -1 }, a = { -1, 0 }, s = { 0, 1 }, d = { 1, 0 } }
local turn = 1

function get_xy(entity)
	ocal pos = get_component(entity, "Position")
	return pos.pos.Square.x, pos.pos.Square.y
end

function print_state()
	print("\n--- Turn " .. turn .. " ---")
	local px, py = get_xy(player)
	local health = get_component(player, "Health")
	print("Player at (" .. px .. "," .. py .. ") HP: " .. health.current .. "/" .. health.max)
	for _, e in ipairs(enemies) do
		if is_entity_alive(e) then
			local ex, ey = get_xy(e)
			local health = get_component(e, "Health")
			print("Enemy " .. e .. " at (" .. ex .. "," .. ey .. ") HP: " .. health.current .. "/" .. health.max)
		end
	end
end

function adjacent(a, b)
	local ax, ay = get_xy(a)
	local bx, by = get_xy(b)
	return math.abs(ax - bx) + math.abs(ay - by) == 1
end

while true do
	print_state()

	-- Player turn
	local acted = false
	while not acted do
		local cmd = get_user_input("Your move (w/a/s/d, e=attack, q=quit): ")
		if directions[cmd] then
			move_entity(player, directions[cmd][1], directions[cmd][2])
			acted = true
		elseif cmd == "e" then
			-- Attack first adjacent enemy
			local attacked = false
			for _, e in ipairs(enemies) do
				if is_entity_alive(e) and adjacent(player, e) then
					print("You attack enemy " .. e .. "!")
					damage_entity(e, 1)
					attacked = true
					break
				end
			end
			if not attacked then
				print("No enemy adjacent to attack.")
			else
				acted = true
			end
		elseif cmd == "q" then
			print("Quitting game. Goodbye!")
			return
		else
			print("Unknown command. Use w/a/s/d to move, e to attack, q to quit.")
		end
	end

	-- Enemy turn
	local px, py = get_xy(player)
	for _, e in ipairs(enemies) do
		if is_entity_alive(e) then
			local ex, ey = get_xy(e)
			local dx = px - ex
			local dy = py - ey
			if math.abs(dx) + math.abs(dy) == 1 then
				print("Enemy " .. e .. " attacks you!")
				damage_entity(player, 1)
			else
				-- Move towards player (simple AI)
				local step_x = dx ~= 0 and (dx > 0 and 1 or -1) or 0
				local step_y = (dx == 0 and dy ~= 0) and (dy > 0 and 1 or -1) or 0
				move_entity(e, step_x, step_y)
			end
		end
	end

	-- Check win/lose
	if not is_entity_alive(player) then
		print("You have died. Game over!")
		break
	end
	local alive_enemies = 0
	for _, e in ipairs(enemies) do
		if is_entity_alive(e) then
			alive_enemies = alive_enemies + 1
		end
	end
	if alive_enemies == 0 then
		print("You defeated all enemies! You win!")
		break
	end

	turn = turn + 1
end
