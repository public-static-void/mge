// engine/engine_plugin_abi.h
#ifdef __cplusplus
extern "C" {
#endif

typedef void *WorldPtr;

typedef struct EngineApi {
  unsigned int (*spawn_entity)(WorldPtr);
  int (*set_component)(WorldPtr, unsigned int, const char *name,
                       const char *json_value);
} EngineApi;

typedef struct PluginVTable {
  int (*init)(EngineApi *api, WorldPtr world);
  void (*shutdown)();
  void (*update)(float delta_time);
} PluginVTable;

extern PluginVTable PLUGIN_VTABLE;

#ifdef __cplusplus
}
#endif
