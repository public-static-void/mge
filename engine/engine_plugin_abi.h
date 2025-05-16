#ifndef ENGINE_PLUGIN_ABI_H
#define ENGINE_PLUGIN_ABI_H

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
  int (*init)(struct EngineApi *api, void *world);
  void (*shutdown)();
  void (*update)(float delta_time);
  const char *(*worldgen_name)();
  int (*generate_world)(const char *params_json, char **out_result_json);
  void (*free_result_json)(char *result_json);
  int (*register_systems)(struct EngineApi *api, void *world,
                          SystemPlugin **systems, int *count);
} PluginVTable;

extern PluginVTable *PLUGIN_VTABLE;

const char *worldgen_name(void);
int generate_world(const char *params_json, char **out_result_json);
void free_result_json(char *result_json);

#ifdef __cplusplus
}
#endif

#endif // ENGINE_PLUGIN_ABI_H
