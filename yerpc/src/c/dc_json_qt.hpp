#pragma once
#include <QJsonDocument>
#include <QJsonValue>
#include <QJsonObject>
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wdeprecated-copy-with-user-provided-copy"
#include <QJsonArray>
#pragma GCC diagnostic pop
#include <cstdlib>
#include <cstring>

#include "dc_json_decl.h"
#include "qglobal.h"
#include "qjsonarray.h"
#include "qjsondocument.h"

struct dc_json { QJsonValue val; };
struct dc_json_array_iter{ QJsonArray::const_iterator iter; QJsonArray::const_iterator end;};
struct dc_json_map_iter{ QJsonObject::const_iterator iter; QJsonObject::const_iterator end;};
struct dc_json_doc { QJsonDocument* qdoc; };

// Document lifecycle
static inline dc_json_doc_t dc_json_doc_parse(const char* s) {
    return {new QJsonDocument(QJsonDocument::fromJson(QByteArray(s)))};
}
static inline dc_json_doc_t dc_json_doc_new() { return dc_json_doc_t{new QJsonDocument()}; }
static inline void dc_json_doc_free(dc_json_doc_t doc) { delete doc.qdoc; }
static inline dc_json_t dc_json_root(dc_json_doc_t doc) {
    if (doc.qdoc->isArray()) return {QJsonValue(doc.qdoc->array())};
    return {QJsonValue(doc.qdoc->object())};
}

// Read
static inline char* dc_json_print(dc_json_t j) {
    QJsonDocument doc;
    if (j.val.isObject()) doc.setObject(j.val.toObject());
    else if (j.val.isArray()) doc.setArray(j.val.toArray());
    else return nullptr;
    QByteArray ba = doc.toJson(QJsonDocument::Compact);
    return strdup(ba.constData());
}
static inline bool dc_json_is_valid(dc_json_t j) { return !j.val.isUndefined(); }
static inline dc_json_t dc_json_get(dc_json_t j, const char* key) { return {j.val.toObject().value(key)}; }
static inline int dc_json_len(dc_json_t j) {
    if (j.val.isArray()) return j.val.toArray().size();
    if (j.val.isObject()) return j.val.toObject().size();
    return 0;
}
static inline bool dc_json_is_object(dc_json_t j) { return j.val.isObject(); }
static inline bool dc_json_is_array(dc_json_t j) { return j.val.isArray(); }
static inline bool dc_json_is_string(dc_json_t j) { return j.val.isString(); }
static inline bool dc_json_is_number(dc_json_t j) { return j.val.isDouble(); }
static inline bool dc_json_is_bool(dc_json_t j) { return j.val.isBool(); }
static inline bool dc_json_is_null(dc_json_t j) { return j.val.isNull(); }
static inline double dc_json_get_double(dc_json_t j) { return j.val.toDouble(); }
static inline int64_t dc_json_get_int(dc_json_t j) { return (int64_t)j.val.toDouble(); }
static inline bool dc_json_get_bool(dc_json_t j) { return j.val.toBool(); }
static inline bool dc_json_string_eq(dc_json_t j, const char* s) {
    return j.val.toString() == QString::fromUtf8(s);
}
static inline char* dc_json_copy_string(dc_json_t j) {
    return j.val.isString() ? strdup(qUtf8Printable(j.val.toString())) : nullptr;
}

static inline dc_json_array_iter_t dc_json_array_iter_new(dc_json_t j) { QJsonArray a = j.val.toArray(); return {a.begin(), a.end()}; }
static inline dc_json_array_iter_t dc_json_array_iter_next(dc_json_array_iter_t it) { return {++it.iter, it.end}; }
static inline bool dc_json_array_iter_is_valid(dc_json_array_iter_t it) { return it.iter != it.end; }
static inline dc_json_t dc_json_array_iter_value(dc_json_array_iter_t it) { return {*it.iter}; }

static inline dc_json_map_iter_t dc_json_map_iter_new(dc_json_t j) { QJsonObject o = j.val.toObject(); return {o.begin(), o.end()}; }
static inline dc_json_map_iter_t dc_json_map_iter_next(dc_json_map_iter_t it) { return {++it.iter, it.end}; }
static inline bool dc_json_map_iter_is_valid(dc_json_map_iter_t it) { return it.iter != it.end; }
static inline dc_json_t dc_json_map_iter_value(dc_json_map_iter_t it) { return {*it.iter}; }
static inline char* dc_json_map_iter_copy_key(dc_json_map_iter_t it) { return strdup(qUtf8Printable(it.iter.key())); }

// Write
static inline dc_json_t dc_json_new_object() { return {QJsonValue(QJsonObject())}; }
static inline dc_json_t dc_json_new_array() { return {QJsonValue(QJsonArray())}; }
static inline dc_json_t dc_json_new_string(const char* s) { return {QJsonValue(QString::fromUtf8(s))}; }
static inline dc_json_t dc_json_new_number(double v) { return {QJsonValue(v)}; }
static inline dc_json_t dc_json_new_bool(bool v) { return {QJsonValue(v)}; }
static inline dc_json_t dc_json_new_null() { return {QJsonValue()}; }

static inline dc_json_t dc_json_add_to_object(dc_json_t obj, const char* key, dc_json_t val) {
    QJsonObject o = obj.val.toObject();
    o.insert(QString::fromUtf8(key), val.val);
    return {QJsonValue(o)};
}
static inline dc_json_t dc_json_add_to_array(dc_json_t arr, dc_json_t val) {
    QJsonArray a = arr.val.toArray();
    a.append(val.val);
    return {QJsonValue(a)};
}


