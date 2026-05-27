// test_abi_mismatch.c — TEST ONLY plugin with wrong ABI version (999)
//
// WARNING: This plugin is for testing ABI version mismatch detection only.
// It is NOT intended for production use. Do NOT include in production builds.
//
// This plugin sets abi_version to 999 (not PLUGIN_ABI_VERSION) to verify
// that the engine's plugin loader correctly rejects mismatched versions.

#include "engine_plugin_abi.h"
#include <stddef.h>
#include <stdio.h>

// Forward declarations
static int init(struct EngineApi *api, void *world);
static void shutdown(void);
static void update(float dt);

// Global vtable struct
static struct PluginVTable vtable;

// Stub functions
static int init(struct EngineApi *api, void *world) {
  (void)api;
  (void)world;
  printf("[TEST ABI MISMATCH] init called (should never happen)\n");
  return 0;
}

static void shutdown(void) {}

static void update(float dt) { (void)dt; }

// Runtime initialization — uses WRONG version 999
__attribute__((constructor)) void init_vtable() {
  vtable.abi_version = 999; // Deliberately wrong — NOT PLUGIN_ABI_VERSION
  vtable.init = init;
  vtable.shutdown = shutdown;
  vtable.update = update;
  vtable.worldgen_name = NULL;
  vtable.generate_world = NULL;
  vtable.free_result_json = NULL;
  vtable.register_systems = NULL;
  vtable.free_systems = NULL;
  vtable.hot_reload = NULL;
}

// Export vtable pointer
__attribute__((visibility("default"))) struct PluginVTable *PLUGIN_VTABLE =
    &vtable;
