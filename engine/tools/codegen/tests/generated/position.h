// AUTO-GENERATED FILE: DO NOT EDIT!
// Schema: PositionComponent

#ifndef POSITION_COMPONENT_H
#define POSITION_COMPONENT_H

#include <stdint.h>

typedef enum {
  POSITION_KIND_SQUARE,
  POSITION_KIND_HEX,
  POSITION_KIND_REGION
} PositionKind;

typedef struct {
  PositionKind kind;
  union {
    struct {
      int32_t x, y, z;
    } Square;
    struct {
      int32_t q, r, z;
    } Hex;
    struct {
      const char *id;
    } Region;
  };
} Position;

typedef struct {
  Position pos;
} PositionComponent;

#endif // POSITION_COMPONENT_H
