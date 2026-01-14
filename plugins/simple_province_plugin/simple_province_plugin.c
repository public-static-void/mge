#include "engine_plugin_abi.h"
#include <jansson.h>
#include <stdlib.h>
#include <string.h>

const char *worldgen_name(void) { return "simple_province"; }

int generate_world(const char *params_json, char **out_result_json) {
  json_error_t error;
  json_t *params = json_loads(params_json, 0, &error);
  if (!params)
    return 1;

  // For now no param usage, could parse here if needed

  json_t *cells = json_array();

  // Define fixed provinces and neighbors
  json_t *provA = json_pack("{s:s, s:[s,s]}", "id", "A", "neighbors", "B", "C");
  json_t *provB = json_pack("{s:s, s:[s]}", "id", "B", "neighbors", "A");
  json_t *provC = json_pack("{s:s, s:[s]}", "id", "C", "neighbors", "A");

  json_array_append_new(cells, provA);
  json_array_append_new(cells, provB);
  json_array_append_new(cells, provC);

  json_t *root = json_object();
  json_object_set_new(root, "topology", json_string("province"));
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
