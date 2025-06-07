#include "engine_plugin_abi.h"
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

// Plugin functions
const char *worldgen_name(void) { return "simple_square"; }

int generate_world(const char *params_json, char **out_result_json) {
  (void)params_json;
  const char *json = "{\"cells\": [{\"id\": \"0,0\", \"x\": 0, \"y\": 0}]}";
  *out_result_json = strdup(json);
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
