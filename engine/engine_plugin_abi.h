#ifndef ENGINE_PLUGIN_ABI_H
#define ENGINE_PLUGIN_ABI_H

// Current ABI version. Increment on any breaking change to PluginVTable layout.
#define PLUGIN_ABI_VERSION 1

#ifdef __cplusplus
extern "C" {
#endif

typedef void *WorldPtr;

typedef void (*SystemRunFn)(WorldPtr, float delta_time);

typedef struct SystemPlugin {
  const char *name;
  SystemRunFn run;
  // Optional: add metadata fields here (e.g., required components)
} SystemPlugin;

typedef struct EngineApi {
  unsigned int (*spawn_entity)(WorldPtr);
  int (*set_component)(WorldPtr, unsigned int, const char *name,
                       const char *json_value);
} EngineApi;

typedef struct PluginVTable {
  unsigned int abi_version;  // MUST equal PLUGIN_ABI_VERSION
  int (*init)(struct EngineApi *api, void *world);
  void (*shutdown)();
  void (*update)(float delta_time);
  const char *(*worldgen_name)();
  int (*generate_world)(const char *params_json, char **out_result_json);
  void (*free_result_json)(char *result_json);
  int (*register_systems)(struct EngineApi *api, void *world,
                          SystemPlugin **systems, int *count);
  void (*free_systems)(SystemPlugin *systems, int count);
  void *(*hot_reload)(void *old_state);
} PluginVTable;

extern PluginVTable *PLUGIN_VTABLE;

const char *worldgen_name(void);
int generate_world(const char *params_json, char **out_result_json);
void free_result_json(char *result_json);

#ifdef __cplusplus
}
#endif

#endif // ENGINE_PLUGIN_ABI_H
