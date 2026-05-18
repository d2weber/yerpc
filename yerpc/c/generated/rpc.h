#pragma once

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "dc_json_cjson.h"
/* dc_json backend must be included before this header */

typedef struct {
    const char* json;   /* serialized JSON-RPC string (owned) */
} request_t;

static inline const char* string_new(const char* s) { return strdup(s); }
static inline void string_unref(const char* s) { free((char*)s); }

static inline void request_unref(request_t* r) {
    if (r) {
        free((void*)r->json);
        free(r);
    }
}

typedef struct {
    uint32_t      id;
    int32_t       error_code;     /* 0 = success, -32700 = parse fail, else JSON-RPC error */
    dc_json_t     result;         /* node in doc_, valid when error_code == 0 */
    const char*   error_message;  /* owned string or literal, valid when error_code != 0 */
    dc_json_doc_t doc_;        /* owns the parsed tree */
} result_t;

static inline result_t* parse_response(const char* json) {
    result_t* r = (result_t*)calloc(1, sizeof(result_t));
    if (!r) return NULL;

    r->doc_ = dc_json_doc_parse(json);
    dc_json_t root = dc_json_root(r->doc_);
    if (!dc_json_is_valid(root)) {
        r->error_code = -32700;
        r->error_message = strdup("Failed to parse JSON");
        return r;
    }

    dc_json_t id = dc_json_get(root, "id");
    if (!dc_json_is_number(id)) {
        r->error_code = -32700;
        r->error_message = strdup("Repsonse does not have a valid `id`");
        return r;
    }
    r->id = (uint32_t)dc_json_get_double(id);

    dc_json_t error = dc_json_get(root, "error");
    if (dc_json_is_valid(error) && dc_json_is_object(error)) {
        dc_json_t code = dc_json_get(error, "code");
        dc_json_t message = dc_json_get(error, "message");
        r->error_code = dc_json_is_number(code) ? (int32_t)dc_json_get_double(code) : -32700;
        r->error_message = dc_json_is_string(message) ? dc_json_copy_string(message) : strdup("Unknown error");
        return r;
    }

    dc_json_t result = dc_json_get(root, "result");
    if (!dc_json_is_valid(result)) {
        r->error_code = -32700;
        r->error_message = strdup("Response has neither 'result' nor 'error'");
        return r;
    }

    r->result = result;
    return r;
}

static inline void result_unref(result_t* r) {
    if (r) {
        dc_json_doc_free(r->doc_);
        free((char*)r->error_message);
        free(r);
    }
}
typedef struct string string_t;
static inline dc_json_t string_to_json(const char* o);
static inline int string_from_json(dc_json_t v, const char** r);
static inline dc_json_t f32_to_json(float o);
static inline int f32_from_json(dc_json_t v, float* r);
static inline int string_from_json(dc_json_t v, const char** r) {
    if (!dc_json_is_string(v)) return -1;
    *r = dc_json_copy_string(v);
    return 0;
}
static inline dc_json_t string_to_json(const char* o) {
    return dc_json_new_string(o);
}
static inline int f32_from_json(dc_json_t v, float* r) {
    if (!dc_json_is_number(v)) return -1;
    *r = (float)dc_json_get_double(v);
    return 0;
}
static inline dc_json_t f32_to_json(float o) {
    return dc_json_new_number((double)o);
}
static inline request_t* build_shout(uint32_t _rpc_id, const char* msg) {
    dc_json_doc_t doc = dc_json_doc_new();
    dc_json_t req = dc_json_root(doc);
    req = dc_json_add_to_object(req, "jsonrpc", dc_json_new_string("2.0"));
    req = dc_json_add_to_object(req, "id", dc_json_new_number((double)_rpc_id));
    req = dc_json_add_to_object(req, "method", dc_json_new_string("shout"));
    dc_json_t params = dc_json_new_array();
    dc_json_add_to_array(params, string_to_json(msg));
    req = dc_json_add_to_object(req, "params", params);
    request_t* r = (request_t*)calloc(1, sizeof(request_t));
    r->json = dc_json_print(req);
    dc_json_doc_free(doc);
    return r;
}
static inline const char* parse_shout(result_t* r) {
    if (!r || r->error_code != 0) return "";
    const char* o;
    if (string_from_json(r->result, &o)) {
        r->error_code = -32700;
        r->error_message = strdup("parse error");
        return "";
    }
    return o;
}
static inline request_t* build_add(uint32_t _rpc_id, float a, float b) {
    dc_json_doc_t doc = dc_json_doc_new();
    dc_json_t req = dc_json_root(doc);
    req = dc_json_add_to_object(req, "jsonrpc", dc_json_new_string("2.0"));
    req = dc_json_add_to_object(req, "id", dc_json_new_number((double)_rpc_id));
    req = dc_json_add_to_object(req, "method", dc_json_new_string("add"));
    dc_json_t params = dc_json_new_array();
    dc_json_add_to_array(params, f32_to_json(a));
    dc_json_add_to_array(params, f32_to_json(b));
    req = dc_json_add_to_object(req, "params", params);
    request_t* r = (request_t*)calloc(1, sizeof(request_t));
    r->json = dc_json_print(req);
    dc_json_doc_free(doc);
    return r;
}
static inline float parse_add(result_t* r) {
    if (!r || r->error_code != 0) return 0;
    float o;
    if (f32_from_json(r->result, &o)) {
        r->error_code = -32700;
        r->error_message = strdup("parse error");
        return 0;
    }
    return o;
}
