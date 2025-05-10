#include "engine_plugin_abi.h"
#include <stdint.h>
#include <stdio.h>

static int init(EngineApi *api, void *world);
static void shutdown(void);
static void update(float dt);

__attribute__((visibility("default"))) PluginVTable PLUGIN_VTABLE = {
    .init = init,
    .shutdown = shutdown,
    .update = update,
};

static int init(EngineApi *api, void *world) {
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
