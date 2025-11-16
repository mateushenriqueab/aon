#include <node_api.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "aon_ffi.h"

#define NAPI_CALL(env, call)                                  \
  do {                                                        \
    napi_status status = (call);                              \
    if (status != napi_ok) {                                  \
      const napi_extended_error_info* info;                   \
      napi_get_last_error_info((env), &info);                 \
      napi_throw_error((env), NULL, info->error_message);     \
      return NULL;                                            \
    }                                                         \
  } while (0)


// Convert Rust char* → JS string and free Rust memory
napi_value make_js_string_and_free(napi_env env, char* ptr) {
    if (ptr == NULL) {
        const char* err = aon_last_error();
        if (err == NULL) err = "Unknown error";
        napi_throw_error(env, NULL, err);
        return NULL;
    }

    napi_value result;
    NAPI_CALL(env, napi_create_string_utf8(env, ptr, NAPI_AUTO_LENGTH, &result));

    aon_free_string(ptr);
    return result;
}


// --------- NODE WRAPPERS ----------

// js: aon.jsonToAon(jsonStr, rootName)
napi_value js_json_to_aon(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2];
    NAPI_CALL(env, napi_get_cb_info(env, info, &argc, args, NULL, NULL));

    // argumento 1: JSON string
    size_t len1;
    NAPI_CALL(env, napi_get_value_string_utf8(env, args[0], NULL, 0, &len1));
    char* json = malloc(len1 + 1);
    napi_get_value_string_utf8(env, args[0], json, len1 + 1, &len1);

    // argumento 2: root schema name
    size_t len2;
    NAPI_CALL(env, napi_get_value_string_utf8(env, args[1], NULL, 0, &len2));
    char* root = malloc(len2 + 1);
    napi_get_value_string_utf8(env, args[1], root, len2 + 1, &len2);

    char* aon = aon_json_to_aon(json, root);

    free(json);
    free(root);

    return make_js_string_and_free(env, aon);
}

// js: aon.aonToJson(aonText)
napi_value js_aon_to_json(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value args[1];
    NAPI_CALL(env, napi_get_cb_info(env, info, &argc, args, NULL, NULL));

    size_t len;
    NAPI_CALL(env, napi_get_value_string_utf8(env, args[0], NULL, 0, &len));
    char* aon = malloc(len + 1);
    napi_get_value_string_utf8(env, args[0], aon, len + 1, &len);

    char* json = aon_aon_to_json(aon);

    free(aon);

    return make_js_string_and_free(env, json);
}

// js: aon.lastError()
napi_value js_last_error(napi_env env, napi_callback_info info) {
    const char* err = aon_last_error();
    if (!err) err = "";
    napi_value result;
    napi_create_string_utf8(env, err, NAPI_AUTO_LENGTH, &result);
    return result;
}


// Init module
napi_value Init(napi_env env, napi_value exports) {
    napi_value fn1, fn2, fn3;

    napi_create_function(env, NULL, 0, js_json_to_aon, NULL, &fn1);
    napi_create_function(env, NULL, 0, js_aon_to_json, NULL, &fn2);
    napi_create_function(env, NULL, 0, js_last_error, NULL, &fn3);

    napi_set_named_property(env, exports, "jsonToAon", fn1);
    napi_set_named_property(env, exports, "aonToJson", fn2);
    napi_set_named_property(env, exports, "lastError", fn3);

    return exports;
}

NAPI_MODULE(NODE_GYP_MODULE_NAME, Init)
