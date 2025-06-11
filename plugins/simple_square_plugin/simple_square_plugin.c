#include "engine_plugin_abi.h"
#include <jansson.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

// Plugin functions
const char *worldgen_name(void) { return "simple_square"; }

static int min(int a, int b) { return a < b ? a : b; }
static int max(int a, int b) { return a > b ? a : b; }

int generate_world(const char *params_json, char **out_result_json) {
  json_error_t error;
  json_t *params = json_loads(params_json, 0, &error);
  if (!params)
    return 1;

  int width = json_integer_value(json_object_get(params, "width"));
  int height = json_integer_value(json_object_get(params, "height"));
  int z_levels = json_integer_value(json_object_get(params, "z_levels"));
  int chunk_x = json_integer_value(json_object_get(params, "chunk_x"));
  int chunk_y = json_integer_value(json_object_get(params, "chunk_y"));

  fprintf(
      stderr,
      "simple_square: chunk_x=%d chunk_y=%d width=%d height=%d z_levels=%d\n",
      chunk_x, chunk_y, width, height, z_levels);

  // Biome/terrain support
  json_t *biomes = json_object_get(params, "biomes");
  int use_biomes =
      biomes && json_is_array(biomes) && json_array_size(biomes) > 0;

  // Prepare output JSON
  json_t *cells = json_array();
  srand((unsigned)time(NULL)); // For random biome/terrain selection

  for (int x = 0; x < width; ++x) {
    for (int y = 0; y < height; ++y) {
      for (int z = 0; z < z_levels; ++z) {
        int gx = chunk_x + x;
        int gy = chunk_y + y;

        json_t *cell = json_object();
        char idbuf[64];
        snprintf(idbuf, sizeof(idbuf), "%d,%d,%d", gx, gy, z);
        json_object_set_new(cell, "id", json_string(idbuf));
        json_object_set_new(cell, "x", json_integer(gx));
        json_object_set_new(cell, "y", json_integer(gy));
        json_object_set_new(cell, "z", json_integer(z));

        // --- Neighbors (4-way) ---
        json_t *neighbors = json_array();
        if (x > 0) {
          json_t *n = json_object();
          json_object_set_new(n, "x", json_integer(gx - 1));
          json_object_set_new(n, "y", json_integer(gy));
          json_object_set_new(n, "z", json_integer(z));
          json_array_append_new(neighbors, n);
        }
        if (x < width - 1) {
          json_t *n = json_object();
          json_object_set_new(n, "x", json_integer(gx + 1));
          json_object_set_new(n, "y", json_integer(gy));
          json_object_set_new(n, "z", json_integer(z));
          json_array_append_new(neighbors, n);
        }
        if (y > 0) {
          json_t *n = json_object();
          json_object_set_new(n, "x", json_integer(gx));
          json_object_set_new(n, "y", json_integer(gy - 1));
          json_object_set_new(n, "z", json_integer(z));
          json_array_append_new(neighbors, n);
        }
        if (y < height - 1) {
          json_t *n = json_object();
          json_object_set_new(n, "x", json_integer(gx));
          json_object_set_new(n, "y", json_integer(gy + 1));
          json_object_set_new(n, "z", json_integer(z));
          json_array_append_new(neighbors, n);
        }
        json_object_set_new(cell, "neighbors", neighbors);

        // --- Biome and Terrain ---
        if (use_biomes) {
          size_t biome_count = json_array_size(biomes);
          size_t biome_idx =
              (gx + gy + z) %
              biome_count; // deterministic, or use rand() % biome_count
          json_t *biome = json_array_get(biomes, biome_idx);
          const char *biome_name =
              json_string_value(json_object_get(biome, "name"));
          json_object_set_new(cell, "biome",
                              json_string(biome_name ? biome_name : "Unknown"));

          json_t *tiles = json_object_get(biome, "tiles");
          if (tiles && json_is_array(tiles) && json_array_size(tiles) > 0) {
            size_t tile_count = json_array_size(tiles);
            size_t tile_idx =
                (gx * 17 + gy * 31 + z * 13) %
                tile_count; // deterministic, or use rand() % tile_count
            const char *terrain =
                json_string_value(json_array_get(tiles, tile_idx));
            json_object_set_new(cell, "terrain",
                                json_string(terrain ? terrain : "unknown"));
          } else {
            json_object_set_new(cell, "terrain", json_string("unknown"));
          }
        }

        json_array_append_new(cells, cell);
      }
    }
  }

  json_t *root = json_object();
  json_object_set_new(root, "topology", json_string("square"));
  json_object_set_new(root, "cells", cells);

  char *dumped = json_dumps(root, 0);
  *out_result_json = dumped ? strdup(dumped) : NULL;

  json_decref(root);
  json_decref(params);

  return *out_result_json ? 0 : 1;
}

void free_result_json(char *result_json) { free(result_json); }

void stub_void(void) {}
void stub_update(float x) { (void)x; }

// Plugin init function
int plugin_init(struct EngineApi *api, void *world) {
  (void)api;
  (void)world;
  return 0;
}

// Declare vtable as non-const global
struct PluginVTable vtable;

// Runtime initialization of vtable after relocation
__attribute__((constructor)) void init_vtable() {
  vtable.init = plugin_init;
  vtable.shutdown = stub_void;
  vtable.update = stub_update;
  vtable.worldgen_name = worldgen_name;
  vtable.generate_world = generate_world;
  vtable.free_result_json = free_result_json;
}

// Export vtable pointer with default visibility
__attribute__((visibility("default"))) struct PluginVTable *PLUGIN_VTABLE =
    &vtable;
