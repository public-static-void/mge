#include "engine_plugin_abi.h"
#include <stdint.h>
#include <stdio.h>

// Forward declarations
static int init(struct EngineApi *api, void *world);
static void shutdown(void);
static void update(float dt);

// Global vtable struct
static struct PluginVTable vtable;

void hello_system(WorldPtr world, float delta_time) {
  (void)world;
  (void)delta_time;
  printf("[PLUGIN] Hello from system!\n");
}

static SystemPlugin system_plugins[] = {{"hello_system", hello_system}};

// Register systems function
int register_systems(struct EngineApi *api, void *world, SystemPlugin **systems,
                     int *count) {
  (void)api;
  (void)world;
  *systems = system_plugins;
  *count = 1;
  return 0;
}

// Runtime initialization of vtable after relocation
__attribute__((constructor)) void init_vtable() {
  vtable.init = init;
  vtable.shutdown = shutdown;
  vtable.update = update;
  vtable.worldgen_name = NULL;
  vtable.generate_world = NULL;
  vtable.free_result_json = NULL;
  vtable.register_systems = register_systems;
  vtable.free_systems = NULL;
}

// Export vtable pointer with default visibility
__attribute__((visibility("default"))) struct PluginVTable *PLUGIN_VTABLE =
    &vtable;

// Plugin function implementations
static int init(struct EngineApi *api, void *world) {
  uint32_t entity = api->spawn_entity(world);
  const char *position_json = "{\"x\": 1.0, \"y\": 2.0}";
  int result = api->set_component(world, entity, "Position", position_json);
  printf("Plugin initialized: spawned entity %u with Position\n", entity);
  return result;
}

static void shutdown(void) { printf("Plugin shutdown\n"); }

static void update(float dt) {
  printf("Plugin update called with dt=%f\n", dt);
}
