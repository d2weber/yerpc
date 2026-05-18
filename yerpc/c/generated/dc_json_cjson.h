#pragma once
#include <cjson/cJSON.h>
#include <string.h>
#include <stdlib.h>
#include <stdbool.h>
#include <stdint.h>

#include "dc_json_decl.h"

struct dc_json { cJSON* node; };
struct dc_json_array_iter{ cJSON* node; };
struct dc_json_map_iter{ cJSON* node; };
struct dc_json_doc { cJSON* node; };

// Document lifecycle
static inline dc_json_doc_t dc_json_doc_parse(const char* s) { return (dc_json_doc_t){cJSON_Parse(s)}; }
static inline dc_json_doc_t dc_json_doc_new(void) { return (dc_json_doc_t){cJSON_CreateObject()}; }
static inline void dc_json_doc_free(dc_json_doc_t doc) { cJSON_Delete(doc.node); }
static inline dc_json_t dc_json_root(dc_json_doc_t doc) { return (dc_json_t){doc.node}; }

// Read
static inline char* dc_json_print(dc_json_t j) { return cJSON_PrintUnformatted(j.node); }
static inline bool dc_json_is_valid(dc_json_t j) { return j.node != NULL; }
static inline dc_json_t dc_json_get(dc_json_t j, const char* key) { return (dc_json_t){cJSON_GetObjectItem(j.node, key)}; }
static inline int dc_json_len(dc_json_t j) { return cJSON_GetArraySize(j.node); }
static inline bool dc_json_is_object(dc_json_t j) { return cJSON_IsObject(j.node); }
static inline bool dc_json_is_array(dc_json_t j) { return cJSON_IsArray(j.node); }
static inline bool dc_json_is_string(dc_json_t j) { return cJSON_IsString(j.node); }
static inline bool dc_json_is_number(dc_json_t j) { return cJSON_IsNumber(j.node); }
static inline bool dc_json_is_bool(dc_json_t j) { return cJSON_IsBool(j.node); }
static inline bool dc_json_is_null(dc_json_t j) { return cJSON_IsNull(j.node); }
static inline double dc_json_get_double(dc_json_t j) { return j.node->valuedouble; }
static inline int64_t dc_json_get_int(dc_json_t j) { return (int64_t)j.node->valuedouble; }
static inline bool dc_json_get_bool(dc_json_t j) { return cJSON_IsTrue(j.node); }
static inline bool dc_json_string_eq(dc_json_t j, const char* s) { return strcmp(j.node->valuestring, s) == 0; }
static inline char* dc_json_copy_string(dc_json_t j) { return strdup(j.node->valuestring); }

static inline dc_json_array_iter_t dc_json_array_iter_new(dc_json_t j) { return (dc_json_array_iter_t){j.node->child}; }
static inline dc_json_array_iter_t dc_json_array_iter_next(dc_json_array_iter_t it) { return (dc_json_array_iter_t){it.node->next}; }
static inline bool dc_json_array_iter_is_valid(dc_json_array_iter_t it) { return it.node != NULL; }
static inline dc_json_t dc_json_array_iter_value(dc_json_array_iter_t it) { return (dc_json_t){it.node}; }

static inline dc_json_map_iter_t dc_json_map_iter_new(dc_json_t j) { return (dc_json_map_iter_t){j.node->child}; }
static inline dc_json_map_iter_t dc_json_map_iter_next(dc_json_map_iter_t it) { return (dc_json_map_iter_t){it.node->next}; }
static inline bool dc_json_map_iter_is_valid(dc_json_map_iter_t it) { return it.node != NULL; }
static inline dc_json_t dc_json_map_iter_value(dc_json_map_iter_t it) { return (dc_json_t){it.node}; }
static inline char* dc_json_map_iter_copy_key(dc_json_map_iter_t it) { return it.node->string ? strdup(it.node->string) : NULL; }

// Write
static inline dc_json_t dc_json_new_object(void) { return (dc_json_t){cJSON_CreateObject()}; }
static inline dc_json_t dc_json_new_array(void) { return (dc_json_t){cJSON_CreateArray()}; }
static inline dc_json_t dc_json_new_string(const char* s) { return (dc_json_t){cJSON_CreateString(s)}; }
static inline dc_json_t dc_json_new_number(double v) { return (dc_json_t){cJSON_CreateNumber(v)}; }
static inline dc_json_t dc_json_new_bool(bool v) { return (dc_json_t){cJSON_CreateBool(v)}; }
static inline dc_json_t dc_json_new_null(void) { return (dc_json_t){cJSON_CreateNull()}; }

static inline dc_json_t dc_json_add_to_object(dc_json_t obj, const char* key, dc_json_t val) {
    cJSON_AddItemToObject(obj.node, key, val.node);
    return obj;
}
static inline dc_json_t dc_json_add_to_array(dc_json_t arr, dc_json_t val) {
    cJSON_AddItemToArray(arr.node, val.node);
    return arr;
}
