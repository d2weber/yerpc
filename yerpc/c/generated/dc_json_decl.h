#pragma once
#include <stdbool.h>
#include <stdint.h>

typedef struct dc_json dc_json_t;
typedef struct dc_json_array_iter dc_json_array_iter_t;
typedef struct dc_json_map_iter dc_json_map_iter_t;
typedef struct dc_json_doc dc_json_doc_t;

// Document lifecycle
static inline dc_json_doc_t dc_json_doc_parse(const char* s);
static inline dc_json_doc_t dc_json_doc_new(void);
static inline void dc_json_doc_free(dc_json_doc_t doc);
static inline dc_json_t dc_json_root(dc_json_doc_t doc);

// Read
static inline char* dc_json_print(dc_json_t j);
static inline bool dc_json_is_valid(dc_json_t j);
static inline dc_json_t dc_json_get(dc_json_t j, const char* key);
static inline int dc_json_len(dc_json_t j);
static inline bool dc_json_is_object(dc_json_t j);
static inline bool dc_json_is_array(dc_json_t j);
static inline bool dc_json_is_string(dc_json_t j);
static inline bool dc_json_is_number(dc_json_t j);
static inline bool dc_json_is_bool(dc_json_t j);
static inline bool dc_json_is_null(dc_json_t j);
static inline double dc_json_get_double(dc_json_t j);
static inline int64_t dc_json_get_int(dc_json_t j);
static inline bool dc_json_get_bool(dc_json_t j);
static inline bool dc_json_string_eq(dc_json_t j, const char* s);
static inline char* dc_json_copy_string(dc_json_t j);
static inline dc_json_array_iter_t dc_json_array_iter_new(dc_json_t j);
static inline dc_json_array_iter_t dc_json_array_iter_next(dc_json_array_iter_t it);
static inline bool dc_json_array_iter_is_valid(dc_json_array_iter_t it);
static inline dc_json_t dc_json_array_iter_value(dc_json_array_iter_t it);
static inline dc_json_map_iter_t dc_json_map_iter_new(dc_json_t j);
static inline dc_json_map_iter_t dc_json_map_iter_next(dc_json_map_iter_t it);
static inline bool dc_json_map_iter_is_valid(dc_json_map_iter_t it);
static inline dc_json_t dc_json_map_iter_value(dc_json_map_iter_t it);
static inline char* dc_json_map_iter_copy_key(dc_json_map_iter_t it);

// Write
static inline dc_json_t dc_json_new_object(void);
static inline dc_json_t dc_json_new_array(void);
static inline dc_json_t dc_json_new_string(const char* s);
static inline dc_json_t dc_json_new_number(double v);
static inline dc_json_t dc_json_new_bool(bool v);
static inline dc_json_t dc_json_new_null(void);
static inline dc_json_t dc_json_add_to_object(dc_json_t obj, const char* key, dc_json_t val);
static inline dc_json_t dc_json_add_to_array(dc_json_t arr, dc_json_t val);
