use crate::c::{
    type_def::{TypeDef, TypeDefInner},
    Method, TypeExpr,
};
use convert_case::{Case, Casing};
use std::{io::Write, iter};

impl TypeExpr {
    pub fn cpp_type(&self, ns: &str) -> String {
        match self {
            Self::Optional(_)
            | Self::Array(_)
            | Self::Map(_)
            | Self::Struct(_)
            | Self::TaggedEnum(_)
            | Self::Tuple(_) => self.type_slug().to_case(Case::Pascal),
            _ => self.c_type(ns),
        }
    }

    pub fn cpp_view_type(&self, ns: &str) -> String {
        match self {
            Self::String => self.c_type(ns),
            Self::Optional(_) if !self.needs_heap_free() => self.cpp_type(ns),
            Self::Optional(_)
            | Self::Array(_)
            | Self::Map(_)
            | Self::Struct(_)
            | Self::TaggedEnum(_)
            | Self::Tuple(_) => format!("{}View", self.cpp_type(ns)),
            _ => self.cpp_type(ns),
        }
    }

    fn release_or_copy_string(&self, name: &str, ns: &str) -> String {
        match self {
            TypeExpr::String => format!("{ns}string_new({name})"),
            TypeExpr::Optional(inner) if inner.is_scalar() => format!("{name}._c"),
            t if t.needs_heap_free() => format!("{name}._c.release()"),
            _ => format!("{name}"),
        }
    }
}

// fn emit_map_view(out: &mut impl Write, val: &TypeExpr, ns: &str) -> std::io::Result<()> {
//     let name = val.cpp_type(ns);
//     let vals_type = cpp_items_ptr(val, ns);
//     let ret = val.cpp_view_type(ns);
//     let elem = cpp_view_elem(val, "_values[i]");
//     let not_found = cpp_view_default(val);
//     writeln!(out, "struct {name} {{")?;
//     writeln!(out, "    char** _keys;")?;
//     writeln!(out, "    {vals_type} _values;")?;
//     writeln!(out, "    size_t _len;")?;
//     writeln!(out, "    size_t size() const {{ return _len; }}")?;
//     writeln!(
//         out,
//         "    const char* key(size_t i) const {{ return _keys[i]; }}"
//     )?;
//     writeln!(out, "    {ret} value(size_t i) const {{ return {elem}; }}")?;
//     writeln!(out, "    {ret} find(const char* k) const {{")?;
//     writeln!(
//         out,
//         "        for (size_t i = 0; i < _len; i++) if (strcmp(_keys[i], k) == 0) return {elem};"
//     )?;
//     writeln!(out, "        return {not_found};")?;
//     writeln!(out, "    }}")?;
//     writeln!(out, "}};\n")
// }

fn emit_defined_declarations(
    out: &mut impl Write,
    d: &TypeDef,
    ns: &str,
) -> Result<(), std::io::Error> {
    let name = d.as_ctype().cpp_type(ns);
    let id = d.as_ctype().id(ns);
    Ok(match &d.def {
        TypeDefInner::StringEnum { .. } => {}
        TypeDefInner::Struct { fields } => {
            let imp = fields
                .iter()
                .map(|(fname, field_t)| {
                    format!("    {} {fname}() const;", field_t.cpp_view_type(ns))
                })
                .reduce(|a, b| a + "\n" + &b)
                .unwrap_or_default();
            let ctor = format!(
                "    {name}({}) : _c({id}_new({})) {{}}",
                argument_list(fields, ns),
                argument_list_release(fields, ns)
            );
            emit_cpp_structs(out, &d.as_ctype(), &imp, &ctor, ns)?;
        }
        TypeDefInner::TaggedEnum { def } => {
            let t: &TypeExpr = &d.as_ctype();
            let imp_owned_only: &str = &format!("    {name}() : _c({id}_new()) {{}};");
            let ct = t.c_type(ns);
            let id = t.id(ns);
            let name = t.cpp_type(ns);
            let view_name = t.cpp_view_type(ns);
            let ptr = pointed_to_type(t, ns);
            for v in &def.variants {
                let vname = v.tag_value.to_case(Case::Snake);
                let id_u = id.to_case(Case::ScreamingSnake);
                let name_u = v.tag_value.to_case(Case::ScreamingSnake);
                writeln!(
                    out,
                    "struct {name}Variant{}View {{",
                    v.tag_value.to_case(Case::UpperCamel)
                )?;
                writeln!(
                    out,
                    "    operator bool() {{ return _c->kind == {id_u}_{name_u}; }}"
                )?;
                for (fname, t) in &v.fields {
                    writeln!(
                        out,
                        "    {0} {fname}() {{ if (!*this) return ({0}){{{1}}}; return ({0}){{_c->data.{vname}.{fname}}}; }}",
                        t.cpp_view_type(ns),
                        t.zero_val(ns)
                    )?;
                }
                writeln!(out, "    {ct} _c;")?;
                writeln!(out, "}};")?;
            }
            writeln!(out, "struct {view_name} {{")?;
            emit_tagged_enum_methods(out, ns, def, &id, &name, "")?;
            writeln!(out, "    operator bool() {{ return bool(_c); }}")?;
            writeln!(out, "    {ct} _c;")?;
            writeln!(out, "}};")?;
            writeln!(out, "struct {name} {{")?;
            emit_tagged_enum_methods(out, ns, def, &id, &name, ".get()")?;
            writeln!(out, "{}", imp_owned_only)?;
            writeln!(out, "    operator bool() {{ return bool(_c); }}")?;
            writeln!(out, "    {view_name} view() const;")?;
            writeln!(out, "    {name}({ct} owned_ptr) : _c(owned_ptr) {{}}")?;
            writeln!(
                out,
                "    struct Deleter {{ void operator()({ct} p) {{ {id}_unref(p); }} }};"
            )?;
            writeln!(out, "    std::unique_ptr<{ptr}, Deleter> _c;")?;
            writeln!(out, "}};")?;
            writeln!(out)?;
        }
    })
}

fn emit_tagged_enum_methods(
    out: &mut impl Write,
    ns: &str,
    def: &crate::c::type_def::EnumDef,
    id: &String,
    name: &String,
    ptr_accessor: &str,
) -> Result<(), std::io::Error> {
    Ok(for v in &def.variants {
        let vname = v.tag_value.to_case(Case::Snake);
        let vname_c = v.tag_value.to_case(Case::UpperCamel);
        let args = argument_list(&v.fields, ns);
        let id_u = id.to_case(Case::ScreamingSnake);
        let name_u = v.tag_value.to_case(Case::ScreamingSnake);
        let args_a = argument_list_release(&v.fields, ns);
        writeln!(out, "    void set_variant_{vname}({args}) {{")?;
        writeln!(out, "        _c->kind = {id_u}_{name_u};")?;
        if !v.fields.is_empty() {
            writeln!(
                out,
                "        _c->data.{vname} = decltype(_c->data.{vname}){{{args_a}}};"
            )?;
        };
        writeln!(out, "    }}")?;
        writeln!(
            out,
            "    {name}Variant{vname_c}View as_variant_{vname}() {{ return {name}Variant{vname_c}View{{_c{ptr_accessor}}}; }}"
        )?;
    })
}

fn emit_cpp_structs(
    out: &mut impl Write,
    t: &TypeExpr,
    imp: &str,
    imp_owned_only: &str,
    ns: &str,
) -> Result<(), std::io::Error> {
    let ct = t.c_type(ns);
    let id = t.id(ns);
    let name = t.cpp_type(ns);
    let view_name = t.cpp_view_type(ns);
    let ptr = pointed_to_type(t, ns);
    writeln!(out, "struct {view_name} {{")?;
    writeln!(out, "{}", imp)?;
    writeln!(out, "    operator bool() {{ return bool(_c); }}")?;
    writeln!(out, "    {ct} _c;")?;
    writeln!(out, "}};")?;
    writeln!(out, "struct {name} {{")?;
    writeln!(out, "{}", imp_owned_only)?;
    writeln!(out, "{}", imp)?;
    writeln!(out, "    operator bool() {{ return bool(_c); }}")?;
    writeln!(out, "    {view_name} view() const;")?;
    writeln!(out, "    {name}({ct} owned_ptr) : _c(owned_ptr) {{}}")?;
    writeln!(out, "    {name}() : _c{{}} {{}}")?;
    writeln!(
        out,
        "    struct Deleter {{ void operator()({ct} p) {{ {id}_unref(p); }} }};"
    )?;
    writeln!(out, "    std::unique_ptr<{ptr}, Deleter> _c;")?;
    writeln!(out, "}};")?;
    writeln!(out)?;
    Ok(())
}

fn emit_defined_definitions(
    out: &mut impl Write,
    d: &TypeDef,
    ns: &str,
) -> Result<(), std::io::Error> {
    Ok(match &d.def {
        TypeDefInner::StringEnum { .. } => {}
        TypeDefInner::Struct { fields } => {
            for (fname, field_t) in fields {
                for namespace in [&d.as_ctype().cpp_type(ns), &d.as_ctype().cpp_view_type(ns)] {
                    writeln!(
                        out,
                        "inline {0} {namespace}::{fname}() const {{ return ({0}){{_c->{fname}}}; }}",
                        field_t.cpp_view_type(ns)
                    )?;
                }
            }
            emit_view_fn_declaration(out, &d.as_ctype(), ns)?;
            writeln!(out)?;
        }
        TypeDefInner::TaggedEnum { .. } => {}
    })
}

fn emit_view_fn_declaration(out: &mut impl Write, t: &TypeExpr, ns: &str) -> std::io::Result<()> {
    let name = t.cpp_type(ns);
    let view_name = t.cpp_view_type(ns);
    writeln!(
        out,
        "inline {view_name} {name}::view() const {{ return ({view_name}){{_c.get()}}; }}"
    )
}

fn emit_rpc_methods(out: &mut impl Write, methods: &[Method], ns: &str) -> std::io::Result<()> {
    for m in methods {
        let t = m.output.as_ref().unwrap_or(&TypeExpr::Void);
        if m.is_notification {
            writeln!(
                out,
                "    void {}({}) {{",
                m.c_name,
                argument_list(&m.args, ns)
            )?;
            writeln!(
                out,
                "        notify({ns}build_{}({}));",
                m.c_name,
                argument_list_accessors(&m.args)
            )?;
            writeln!(out, "    }}\n")?;
        } else {
            let ret = match t {
                TypeExpr::String => "std::string".to_owned(),
                _ => t.cpp_type(ns),
            };
            let method_name = &m.c_name;
            let is_void = matches!(t, TypeExpr::Void);
            writeln!(
                out,
                "    {ret} {method_name}({}) {{",
                argument_list(&m.args, ns)
            )?;
            writeln!(
                out,
                "        Result<{ret}> r = {method_name}_request({}).get();",
                argument_list_move(&m.args)
            )?;
            writeln!(
                out,
                r#"        if (r.error_code) {{ line_logger_("Error " + std::to_string(r.error_code) + " in `{method_name}`: " + r.error_message); }}"#
            )?;
            if is_void {
                writeln!(out, "        return;")?;
            } else {
                writeln!(out, "        return std::move(r.result);")?;
            }
            writeln!(out, "    }}")?;
            writeln!(
                out,
                "    std::future<Result<{ret}>> {method_name}_request({}) {{",
                argument_list(&m.args, ns)
            )?;
            let c_args = argument_list_accessors(iter::chain(
                iter::once(&("_id".to_owned(), TypeExpr::U32)),
                &m.args,
            ));
            writeln!(out, "        uint32_t _id = next_id();")?;
            writeln!(
                out,
                "        auto* raw_prom = new std::promise<Result<{ret}>>();"
            )?;
            writeln!(out, "        auto fut = raw_prom->get_future();")?;
            writeln!(out, "        dispatch(_id, {ns}build_{0}({c_args}), [raw_prom]({ns}result_t* raw_res) {{", m.c_name)?;
            writeln!(
                out,
                "            std::unique_ptr<std::promise<Result<{ret}>>> prom{{raw_prom}};"
            )?;
            if is_void {
                writeln!(out, "            if (raw_res->error_code != 0) {{")?;
                writeln!(out, "                prom->set_value({{raw_res->error_message, raw_res->error_code}});")?;
                writeln!(out, "                return;")?;
                writeln!(out, "            }}")?;
                writeln!(out, "            prom->set_value({{{{}}, 0}});")?;
            } else {
                writeln!(out, "            if (raw_res->error_code != 0) {{")?;
                writeln!(out, "                prom->set_value({{{{}}, raw_res->error_message, raw_res->error_code}});")?;
                writeln!(out, "                return;")?;
                writeln!(out, "            }}")?;
                writeln!(out, "            {} res;", t.c_type(ns))?;
                writeln!(
                    out,
                    "            if ({}_from_json(raw_res->result, &res) != 0) {{",
                    t.id(ns)
                )?;
                writeln!(
                    out,
                    "                prom->set_value({{{{}}, \"parse error\", -32700}});"
                )?;
                writeln!(out, "                return;")?;
                writeln!(out, "            }}")?;
                writeln!(out, "            prom->set_value({{res, {{}}, 0}});")?;
                if matches!(t, TypeExpr::String) {
                    writeln!(out, "            {ns}string_unref(res);")?;
                }
            }
            writeln!(out, "        }});")?;
            writeln!(out, "        return fut;")?;
            writeln!(out, "    }}\n")?;
        }
    }
    Ok(())
}

fn argument_list(fields: &Vec<(String, TypeExpr)>, ns: &str) -> String {
    fields
        .iter()
        .map(|(fname, field_t)| format!("{} {fname}", field_t.cpp_type(ns)))
        .reduce(|a, b| a + ", " + &b)
        .unwrap_or_default()
}

fn argument_list_move<'a>(fields: impl IntoIterator<Item = &'a (String, TypeExpr)>) -> String {
    fields
        .into_iter()
        .map(|(n, t)| match t {
            TypeExpr::String => n.to_owned(),
            t if t.needs_heap_free() => format!("std::move({n})"),
            _ => n.to_owned(),
        })
        .reduce(|a, b| a + ", " + &b)
        .unwrap_or_default()
}
fn argument_list_accessors<'a>(fields: impl IntoIterator<Item = &'a (String, TypeExpr)>) -> String {
    fields
        .into_iter()
        .map(|(n, t)| {
            format!(
                "{n}{}",
                match t {
                    TypeExpr::String => "",
                    TypeExpr::Optional(inner) if inner.is_scalar() => "._c",
                    t if t.needs_heap_free() => "._c.get()",
                    _ => "",
                }
            )
        })
        .reduce(|a, b| a + ", " + &b)
        .unwrap_or_default()
}
fn argument_list_release(fields: &Vec<(String, TypeExpr)>, ns: &str) -> String {
    fields
        .iter()
        .map(|(name, t)| t.release_or_copy_string(name, ns))
        .reduce(|a, b| a + ", " + &b)
        .unwrap_or_default()
}

fn emit_header_preamble(out: &mut impl Write, c_header: &str) -> std::io::Result<()> {
    write!(
        out,
        r#"#pragma once

#include <unordered_map>
#include <future>
#include <mutex>
#include <thread>
#include <atomic>
#include <functional>
#include <cstring>

extern "C" {{
#include "{c_header}"
}}

namespace dc {{

template<typename T, typename... Args>
struct is_single_same : std::false_type {{}};

template<typename T, typename Arg>
struct is_single_same<T, Arg> : std::is_same<typename std::decay<Arg>::type, T> {{}};

// We need this to show better error messages when the deleted copy constructor is requested
template<typename Self, typename... Args>
using enable_if_not_same = typename std::enable_if<
!is_single_same<Self, Args...>::value, int>::type;

"#
    )
}

pub fn generate_hpp(
    out: &mut impl Write,
    methods: &[Method],
    defs: &[TypeDef],
    c_header: &str,
    ns: &str,
) -> std::io::Result<()> {
    emit_header_preamble(out, c_header)?;
    for d in defs {
        emit_defined_forward_declarations(out, d, ns)?;
    }
    let all_type_exprs = crate::c::collect_all_type_exprs(methods, defs);
    emit_type_expr_declarations(out, &all_type_exprs, ns).unwrap();
    for d in defs {
        emit_defined_declarations(out, d, ns)?;
    }
    for d in defs {
        emit_defined_definitions(out, d, ns)?;
    }
    emit_type_expr_definitions(out, &all_type_exprs, ns).unwrap();
    write!(
        out,
        r#"template<typename T>
struct Result {{
    T result;
    std::string error_message;
    int32_t error_code;
}};

template<>
struct Result<void> {{
    std::string error_message;
    int32_t error_code;
}};

template<typename Transport>
class Rpc {{
    using Handler = std::function<void({ns}result_t*)>;

    Transport transport_;
    std::mutex mu_;
    std::atomic<uint32_t> next_id_{{1}};
    std::atomic<bool> done_{{false}};
    std::unordered_map<uint32_t, Handler> pending_;
    std::thread reader_;

    std::function<void(std::string)> line_logger_;

    uint32_t next_id() {{ return next_id_++; }}

    void reader_loop() {{
        while (true) {{
            const char* line = transport_.read();
            if (!line || done_) break;
            {ns}result_t* r = {ns}parse_response(line);
            Handler h;
            {{
                std::lock_guard<std::mutex> lk(mu_);
                auto it = pending_.find(r->id);
                if (it != pending_.end()) {{ h = std::move(it->second); pending_.erase(it); }}
            }}
            if (h) h(r);
            {ns}result_unref(r);
        }}
    }}

    void notify({ns}request_t* raw) {{
        transport_.send(raw->json);
        {ns}request_unref(raw);
    }}

    void dispatch(uint32_t id, {ns}request_t* raw, Handler h) {{
        {{ std::lock_guard<std::mutex> lk(mu_); pending_[id] = std::move(h); }}
        transport_.send(raw->json);
        {ns}request_unref(raw);
    }}

public:
    Rpc(Transport t, std::function<void(std::string)> line_logger) : transport_(std::move(t)), line_logger_(line_logger) {{ reader_ = std::thread([this]{{ reader_loop(); }}); }}
    ~Rpc() {{
        done_ = true;
        transport_.close();
        if (reader_.joinable()) reader_.join();
        std::lock_guard<std::mutex> lk(mu_);
        for (auto& kv : pending_) kv.second(nullptr);
        pending_.clear();
    }}
    Rpc(const Rpc&) = delete;
    Rpc& operator=(const Rpc&) = delete;

"#
    )?;
    emit_rpc_methods(out, methods, ns)?;
    write!(
        out,
        r#"}};

}} // namespace dc
"#
    )
}

fn emit_type_expr_declarations(
    out: &mut impl Write,
    types: &[TypeExpr],
    ns: &str,
) -> std::io::Result<()> {
    crate::c::apply_recursive(types, |t| {
        if matches!(
            t,
            TypeExpr::Struct(_) | TypeExpr::TaggedEnum(_) | TypeExpr::StringEnum(_)
        ) | t.is_scalar()
        {
            return Ok(());
        };

        let name = t.cpp_type(ns);
        let ct = t.c_type(ns);
        match t {
            TypeExpr::Array(inner) => {
                emit_cpp_structs(
                    out,
                    t,
                    &format!(
                        "    size_t size() const;\n    {} operator[](size_t i) const;",
                        inner.cpp_view_type(ns)
                    ),
                    &format!("    template<class... Args, enable_if_not_same<{name}, Args...> = 0> {name}(Args&&...);",),
                    ns,
                )?;
            }
            TypeExpr::Map(val) => {
                emit_cpp_structs(
                    out,
                    t,
                    &format!(
                        "    size_t size() const;\n    {} find(const char* k) const;",
                        val.cpp_view_type(ns)
                    ),
                    &format!("    template<class... Args, enable_if_not_same<{name}, Args...> = 0> {name}(Args&&...);\n    using kv = std::pair<const char*, {}>;", val.cpp_type(ns)),
                    ns,
                )?;
            }
            TypeExpr::Tuple(elems) => {
                emit_cpp_structs(
                    out,
                    t,
                    &elems
                        .iter()
                        .enumerate()
                        .map(|(i, el)| {
                            let el_type = el.cpp_view_type(ns);
                            format!("    {el_type} field_{i}() const;")
                        })
                        .reduce(|a, b| a + "\n" + &b)
                        .unwrap_or_default(),
                    &format!("    {name}({});", {
                        elems
                            .iter()
                            .enumerate()
                            .map(|(i, field_t)| format!("{} field_{i}", field_t.cpp_type(ns)))
                            .reduce(|a, b| a + ", " + &b)
                            .unwrap_or_default()
                    }),
                    ns,
                )?;
            }
            TypeExpr::Optional(inner) if t.needs_heap_free() => {
                let ctors = match **inner {
                    TypeExpr::String => "", // Because cpp_type == c_type, the ctor already exists
                    _ => &format!("    {name}({});", inner.cpp_type(ns)),
                };
                let imp = &format!("    {} inner();", inner.cpp_view_type(ns));
                emit_cpp_structs(out, t, imp, ctors, ns)?;
            }
            TypeExpr::Optional(inner) => {
                let inner_name = inner.cpp_type(ns);
                writeln!(out, "struct {name} {{")?;
                writeln!(out, "    {name}({inner_name} val) : _c{{true, val}} {{}}")?;
                writeln!(out, "    {name}() : _c{{}} {{}}")?;
                writeln!(out, "    {name}({ct} inner) : _c{{inner}} {{}}")?;
                writeln!(
                    out,
                    "    {inner_name} inner() {{ return _c.has_value ? _c.value : {}; }};",
                    inner.zero_val(ns)
                )?;
                writeln!(out, "    operator bool() {{ return _c.has_value; }}")?;
                writeln!(out, "    {ct} _c;")?;
                writeln!(out, "}};")?;
                writeln!(out)?;
            }
            _ => {}
        }
        Ok(())
    })
}

fn pointed_to_type(t: &TypeExpr, ns: &str) -> String {
    match t {
        TypeExpr::String => "const char".to_owned(),
        TypeExpr::Optional(inner) if inner.needs_heap_free() => pointed_to_type(inner, ns),
        _ if t.needs_heap_free() => format!("{ns}{}_t", t.type_slug()),
        _ => panic!(
            "This is inteded for heap values only, got `{}`",
            t.type_slug()
        ),
    }
}

fn emit_type_expr_definitions(
    out: &mut impl Write,
    types: &[TypeExpr],
    ns: &str,
) -> std::io::Result<()> {
    crate::c::apply_recursive(types, |t| {
        if matches!(
            t,
            TypeExpr::Struct(_) | TypeExpr::TaggedEnum(_) | TypeExpr::StringEnum(_)
        ) | t.is_scalar()
        {
            return Ok(());
        };

        let id = t.id(ns);
        let name = t.cpp_type(ns);
        let view_name = t.cpp_view_type(ns);
        match t {
            TypeExpr::Array(inner) => {
                let inner_view = inner.cpp_view_type(ns);

                writeln!(
                    out,
                    "inline size_t {view_name}::size() const {{ return _c->len; }}"
                )?;
                writeln!(
                    out,
                    "inline {inner_view} {view_name}::operator[](size_t i) const {{ return ({inner_view}){{_c->items[i]}}; }}"
                )?;
                writeln!(
                    out,
                    "template<class... Args, enable_if_not_same<{name}, Args...>>"
                )?;
                writeln!(
                    out,
                    "inline {name}::{name}(Args&&... args) : _c({id}_new(sizeof...(args))) {{"
                )?;
                writeln!(out, "   size_t i = 0;")?;
                writeln!(
                    out,
                    "    int dummy[] = {{0, (_c->items[i++] = {}, 0)...}};",
                    inner.release_or_copy_string("std::forward<Args>(args)", ns)
                )?;
                writeln!(out, "    (void)dummy;")?;
                writeln!(out, "}}")?;
                emit_view_fn_declaration(out, &t, ns)?;
                writeln!(
                    out,
                    "inline size_t {name}::size() const {{ return _c ? _c->len : 0; }}"
                )?;
                writeln!(
                    out,
                    "inline {inner_view} {name}::operator[](size_t i) const {{ return ({inner_view}){{_c->items[i]}}; }}"
                )?;
                writeln!(out)?;
            }
            TypeExpr::Map(val) => {
                let val_view = val.cpp_view_type(ns);
                writeln!(
                    out,
                    "template<class... Args, enable_if_not_same<{name}, Args...>>"
                )?;
                writeln!(
                    out,
                    "{name}::{name}(Args&&... args) : _c({id}_new(sizeof...(args))) {{"
                )?;
                writeln!(out, "    size_t i = 0;")?;
                writeln!(
                    out,
                    "    int dummy[] = {{0, (_c->keys[i] = {ns}string_new(args.first), _c->values[i++] = {}, 0)...}};",
                    val.release_or_copy_string("std::forward<Args>(args).second", ns)
                )?;
                writeln!(out, "    (void)dummy;")?;
                writeln!(out, "}}")?;
                for namespace in [view_name, name] {
                    writeln!(
                        out,
                        "inline size_t {namespace}::size() const {{ return _c ? _c->len : 0; }}"
                    )?;
                    writeln!(
                        out,
                        "inline {val_view} {namespace}::find(const char* k) const {{"
                    )?;
                    writeln!(
        out,
        "    for (size_t i = 0; i < _c->len; i++) if (strcmp(_c->keys[i], k) == 0) return ({val_view}){{_c->values[i]}};"
    )?;
                    writeln!(out, "    return {{}};")?;
                    writeln!(out, "}}")?;
                }
            }
            TypeExpr::Tuple(elems) => {
                for (i, el) in elems.iter().enumerate() {
                    let el_type = el.cpp_view_type(ns);
                    writeln!(out, "inline {el_type} {view_name}::field_{i}() const {{ return ({el_type}){{_c->field_{i}}}; }}")?;
                    writeln!(
                        out,
                        "inline {el_type} {name}::field_{i}() const {{ return ({el_type}){{_c->field_{i}}}; }}"
                    )?;
                }
                let elems = vec_with_tuple_names(elems);
                writeln!(
                    out,
                    "inline {name}::{name}({}) : _c({id}_new({})) {{}}",
                    argument_list(&elems, ns),
                    argument_list_release(&elems, ns)
                )?;
                emit_view_fn_declaration(out, &t, ns)?;
                writeln!(out)?;
            }
            TypeExpr::Optional(inner) if inner.needs_heap_free() => {
                writeln!(
                    out,
                    "inline {0} {name}::inner() {{ return ({0}){{_c.get()}}; }}",
                    inner.cpp_view_type(ns)
                )?;
                match **inner {
                    TypeExpr::String => (), // Because cpp_type == c_type, the ctor already exists
                    _ => writeln!(
                        out,
                        "{name}::{name}({} inner) : _c{{inner._c.release()}} {{}}",
                        inner.cpp_type(ns)
                    )?,
                }
                writeln!(out)?;
            }
            _ => {}
        }
        Ok(())
    })
}

fn vec_with_tuple_names(elems: &Vec<TypeExpr>) -> Vec<(String, TypeExpr)> {
    elems
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, t)| (format!("field_{i}"), t))
        .collect()
}

fn emit_defined_forward_declarations(
    out: &mut impl Write,
    d: &TypeDef,
    ns: &str,
) -> std::io::Result<()> {
    match &d.def {
        TypeDefInner::Struct { .. } | TypeDefInner::TaggedEnum { .. } => {
            writeln!(out, "struct {};", d.as_ctype().cpp_view_type(ns))?;
            writeln!(out, "struct {};", d.as_ctype().cpp_type(ns))
        }
        TypeDefInner::StringEnum { .. } => Ok(()),
    }
}
