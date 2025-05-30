# PositionComponent type stub
from typing import Optional, TypedDict, Union

class Square(TypedDict):
    x: int
    y: int
    z: int

class Hex(TypedDict):
    q: int
    r: int
    z: int

class Region(TypedDict):
    id: str

Position = Union[Square, Hex, Region]

class PositionComponent(TypedDict):
    pos: Position
