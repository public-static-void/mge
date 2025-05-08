-- Spawn player
local player = spawn_entity()
set_component(player, "Type", { kind = "player" })
set_component(player, "Position", { x = 0, y = 0 })
set_component(player, "Health", { current = 10, max = 10 })

-- Spawn enemies
local enemies = {}
for i = 1, 3 do
	local enemy = spawn_entity()
	set_component(enemy, "Type", { kind = "enemy" })
	set_component(enemy, "Position", { x = i * 2, y = 1 })
	set_component(enemy, "Health", { current = 3, max = 3 })
	table.insert(enemies, enemy)
end

local turn = 1
while true do
	print("Turn " .. turn)

	-- Player turn: attack adjacent enemy if any
	if is_entity_alive(player) then
		local player_pos = get_component(player, "Position")
		for _, enemy in ipairs(enemies) do
			if is_entity_alive(enemy) then
				local epos = get_component(enemy, "Position")
				if math.abs(player_pos.x - epos.x) + math.abs(player_pos.y - epos.y) == 1 then
					print("Player attacks enemy " .. enemy)
					damage_entity(enemy, 1)
					break
				end
			end
		end
	end

	-- Enemy turns: move toward player or attack if adjacent
	local player_pos = get_component(player, "Position")
	for _, enemy in ipairs(enemies) do
		if is_entity_alive(enemy) then
			local epos = get_component(enemy, "Position")
			local dx = player_pos.x - epos.x
			local dy = player_pos.y - epos.y
			if math.abs(dx) + math.abs(dy) == 1 then
				print("Enemy " .. enemy .. " attacks player!")
				damage_entity(player, 1)
			else
				local step_x = dx ~= 0 and (dx > 0 and 1 or -1) or 0
				local step_y = (dx == 0 and dy ~= 0) and (dy > 0 and 1 or -1) or 0
				move_entity(enemy, step_x, step_y)
			end
		end
	end

	process_deaths()
	process_decay()

	-- Print positions and healths
	print_positions()
	print_healths()

	-- Check win/lose conditions
	if not is_entity_alive(player) then
		print("You lose!")
		break
	end
	local alive_enemies = 0
	for _, enemy in ipairs(enemies) do
		if is_entity_alive(enemy) then
			alive_enemies = alive_enemies + 1
		end
	end
	if alive_enemies == 0 then
		print("You win!")
		break
	end

	turn = turn + 1
end
