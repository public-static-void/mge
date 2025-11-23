#include "engine_plugin_abi.h"
#include <jansson.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

const char *worldgen_name(void) { return "simple_hex"; }

static const int NEIGHBOR_OFFSETS[6][2] = {{1, 0},  {1, -1}, {0, -1},
                                           {-1, 0}, {-1, 1}, {0, 1}};

int generate_world(const char *params_json, char **out_result_json) {
  json_error_t error;
  json_t *params = json_loads(params_json, 0, &error);
  if (!params)
    return 1;

  int width = json_integer_value(json_object_get(params, "width"));
  int height = json_integer_value(json_object_get(params, "height"));
  int z_levels = json_integer_value(json_object_get(params, "z_levels"));
  int chunk_q = json_integer_value(json_object_get(params, "chunk_q"));
  int chunk_r = json_integer_value(json_object_get(params, "chunk_r"));

  json_t *biomes = json_object_get(params, "biomes");
  int use_biomes =
      biomes && json_is_array(biomes) && json_array_size(biomes) > 0;

  json_t *cells = json_array();
  srand((unsigned)time(NULL));

  for (int q = 0; q < width; ++q) {
    for (int r = 0; r < height; ++r) {
      for (int z = 0; z < z_levels; ++z) {
        int gq = chunk_q + q;
        int gr = chunk_r + r;

        json_t *cell = json_object();
        json_object_set_new(cell, "q", json_integer(gq));
        json_object_set_new(cell, "r", json_integer(gr));
        json_object_set_new(cell, "z", json_integer(z));

        // Add neighbors
        json_t *neighbors = json_array();
        for (int i = 0; i < 6; ++i) {
          int nq = gq + NEIGHBOR_OFFSETS[i][0];
          int nr = gr + NEIGHBOR_OFFSETS[i][1];
          // Simple bounds check for neighbor inclusion
          if (nq >= chunk_q && nq < chunk_q + width && nr >= chunk_r &&
              nr < chunk_r + height) {
            json_t *n = json_object();
            json_object_set_new(n, "q", json_integer(nq));
            json_object_set_new(n, "r", json_integer(nr));
            json_object_set_new(n, "z", json_integer(z));
            json_array_append_new(neighbors, n);
          }
        }
        json_object_set_new(cell, "neighbors", neighbors);

        // Biome and Terrain
        if (use_biomes) {
          size_t biome_count = json_array_size(biomes);
          size_t biome_idx = (gq + gr + z) % biome_count;
          json_t *biome = json_array_get(biomes, biome_idx);
          const char *biome_name =
              json_string_value(json_object_get(biome, "name"));
          json_object_set_new(cell, "biome",
                              json_string(biome_name ? biome_name : "Unknown"));

          json_t *tiles = json_object_get(biome, "tiles");
          if (tiles && json_is_array(tiles) && json_array_size(tiles) > 0) {
            size_t tile_count = json_array_size(tiles);
            size_t tile_idx = (gq * 17 + gr * 31 + z * 13) % tile_count;
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
  json_object_set_new(root, "topology", json_string("hex"));
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

int plugin_init(struct EngineApi *api, void *world) {
  (void)api;
  (void)world;
  return 0;
}

struct PluginVTable vtable;

__attribute__((constructor)) void init_vtable() {
  vtable.init = plugin_init;
  vtable.shutdown = stub_void;
  vtable.update = stub_update;
  vtable.worldgen_name = worldgen_name;
  vtable.generate_world = generate_world;
  vtable.free_result_json = free_result_json;
}

__attribute__((visibility("default"))) struct PluginVTable *PLUGIN_VTABLE =
    &vtable;
