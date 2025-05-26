def test_pathfinding(make_world):
    world = make_world()
    for x in range(3):
        for y in range(3):
            world.add_cell(x, y, 0)
    for x in range(3):
        for y in range(3):
            for dx, dy in [(1,0),(0,1),(-1,0),(0,-1)]:
                nx, ny = x+dx, y+dy
                if 0<=nx<=2 and 0<=ny<=2:
                    world.add_neighbor((x,y,0), (nx,ny,0))
    world.set_cell_metadata({"Square": {"x":1,"y":1,"z":0}}, {"walkable": False})
    result = world.find_path({"Square": {"x":0,"y":0,"z":0}}, {"Square": {"x":2,"y":2,"z":0}})
    assert result is not None
    path = result["path"]
    for cell in path:
        if "Square" in cell:
            assert not (cell["Square"]["x"]==1 and cell["Square"]["y"]==1)
    assert len(path) == 5
