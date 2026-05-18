use super::TypeExpr;
use convert_case::{
    Case::{ScreamingSnake, Snake},
    Casing,
};
use core::fmt::Write;

#[derive(Debug, Clone)]
pub struct TypeDef {
    pub(crate) name: String,
    pub(crate) def: TypeDefInner,
}

#[derive(Debug, Clone)]
pub(crate) enum TypeDefInner {
    Struct { fields: Vec<(String, TypeExpr)> },
    TaggedEnum { def: EnumDef },
    StringEnum { variants: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub tag_value: String,
    pub fields: Vec<(String, TypeExpr)>,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub tag_field: String,
    pub variants: Vec<EnumVariant>,
}

impl TypeDef {
    fn id(&self, ns: &str) -> String {
        format!("{ns}{}", self.name.to_case(Snake))
    }

    pub(crate) fn as_ctype(&self) -> TypeExpr {
        match &self.def {
            TypeDefInner::Struct { .. } => TypeExpr::Struct(self.name.clone()),
            TypeDefInner::TaggedEnum { .. } => TypeExpr::TaggedEnum(self.name.clone()),
            TypeDefInner::StringEnum { .. } => TypeExpr::StringEnum(self.name.clone()),
        }
    }

    pub(crate) fn emit_forward_decls(&self, out: &mut impl Write, ns: &str) -> core::fmt::Result {
        let id = self.id(ns);
        match &self.def {
            TypeDefInner::Struct { .. } => {
                writeln!(out, "typedef struct {id} {id}_t;")?;
                writeln!(out, "static inline void {id}_unref({id}_t* o);")?;
                writeln!(
                    out,
                    "static inline int {id}_from_json(dc_json_t o, {id}_t** out);"
                )?;
                writeln!(out, "static inline dc_json_t {id}_to_json({id}_t* o);")?;
            }
            TypeDefInner::TaggedEnum { .. } => {
                writeln!(out, "typedef struct {id} {id}_t;")?;
                writeln!(out, "static inline void {id}_unref({id}_t* o);")?;
                writeln!(
                    out,
                    "static inline int {id}_from_json(dc_json_t o, {id}_t** out);"
                )?;
                writeln!(out, "static inline dc_json_t {id}_to_json({id}_t* o);")?;
            }
            TypeDefInner::StringEnum { variants } => {
                string_enum_def(out, &self.id(ns), variants)?;
                writeln!(out)?;
                string_enum_to_json(out, &self.id(ns), variants)?;
                writeln!(out)?;
                string_enum_from_json(out, &self.id(ns), variants)?;
                writeln!(out)?;
            }
        }
        Ok(())
    }

    pub(crate) fn emit_late_decls(&self, out: &mut impl Write, ns: &str) -> core::fmt::Result {
        let id = self.id(ns);
        match &self.def {
            TypeDefInner::Struct { fields } => {
                writeln!(
                    out,
                    "static inline {id}_t* {id}_new({});",
                    argument_list(fields, ns)
                )?;
            }
            TypeDefInner::TaggedEnum { def } => {
                writeln!(out, "static inline {id}_t* {id}_new();")?;
            }
            TypeDefInner::StringEnum { .. } => {}
        }
        Ok(())
    }

    pub(crate) fn emit_definitions(&self, out: &mut impl Write, ns: &str) -> core::fmt::Result {
        match &self.def {
            TypeDefInner::Struct { fields } => {
                struct_def(out, &self.id(ns), ns, &fields)?;
                struct_new(out, &self.id(ns), ns, &fields)?;
                struct_unref(out, &self.id(ns), ns, &fields)?;
                struct_to_json(out, &self.id(ns), ns, &fields)?;
                struct_from_json(out, &self.id(ns), ns, &fields)?;
            }
            TypeDefInner::TaggedEnum { def } => {
                tagged_enum_def(out, &self.id(ns), ns, &def)?;
                tagged_enum_new(out, &self.id(ns))?;
                tagged_enum_unref(out, &self.id(ns), ns, &def)?;
                tagged_enum_to_json(out, &self.id(ns), ns, &def)?;
                tagged_enum_from_json(out, &self.id(ns), ns, &def)?;
            }
            TypeDefInner::StringEnum { .. } => (), // already emitted early
        }
        Ok(())
    }

    pub fn field_ctypes(&self) -> Vec<TypeExpr> {
        match &self.def {
            TypeDefInner::Struct { fields } => fields.iter().map(|(_, ct)| ct.clone()).collect(),
            TypeDefInner::TaggedEnum { def } => def
                .variants
                .iter()
                .flat_map(|v| v.fields.iter().map(|(_, ct)| ct.clone()))
                .collect(),
            TypeDefInner::StringEnum { .. } => vec![],
        }
    }
}

fn string_enum_def(
    out: &mut impl core::fmt::Write,
    id: &str,
    variants: &[String],
) -> core::fmt::Result {
    let idu = id.to_uppercase();
    writeln!(out, "typedef enum {{")?;
    writeln!(out, "    {idu}_UNPARSED = 0,")?;
    for (v, i) in variants.iter().zip(1..) {
        writeln!(out, "    {idu}_{} = {i},", v.to_case(ScreamingSnake))?;
    }
    writeln!(out, "}} {id}_t;")?;
    Ok(())
}

fn string_enum_from_json(out: &mut impl Write, id: &str, variants: &[String]) -> core::fmt::Result {
    let idu = id.to_uppercase();
    writeln!(
        out,
        "static inline int {id}_from_json(dc_json_t j, {id}_t* out) {{"
    )?;
    writeln!(
        out,
        "    if (!dc_json_is_string(j)) {{ *out = {idu}_UNPARSED; return -1; }}"
    )?;
    for v in variants.iter() {
        writeln!(
            out,
            r#"    if (dc_json_string_eq(j, "{v}")) {{ *out = {idu}_{}; return 0; }}"#,
            v.to_case(ScreamingSnake)
        )?;
    }
    writeln!(out, "    *out = {idu}_UNPARSED;")?;
    writeln!(out, "    return 0;")?;
    writeln!(out, "}}")?;
    Ok(())
}

fn string_enum_to_json(
    out: &mut impl Write,
    id: &str,
    variants: &[String],
) -> Result<(), core::fmt::Error> {
    let idu = id.to_uppercase();
    writeln!(out, "static inline dc_json_t {id}_to_json({id}_t v) {{")?;
    writeln!(out, "    switch (v) {{")?;
    for v in variants.iter() {
        writeln!(
            out,
            "    case {idu}_{}: return dc_json_new_string(\"{v}\");",
            v.to_case(ScreamingSnake)
        )?;
    }
    writeln!(out, "    default: break;")?;
    writeln!(out, "    }}")?;
    writeln!(out, "    return dc_json_new_null();")?;
    writeln!(out, "}}")?;
    Ok(())
}

fn struct_def(
    out: &mut impl Write,
    id: &str,
    ns: &str,
    fields: &[(String, TypeExpr)],
) -> core::fmt::Result {
    writeln!(out, "typedef struct {id} {{")?;
    for (fname, ct) in fields {
        writeln!(out, "    {} {fname};", ct.c_type(ns))?;
    }
    writeln!(out, "}} {id}_t;\n")?;
    Ok(())
}

fn struct_new(
    out: &mut impl Write,
    id: &str,
    ns: &str,
    fields: &[(String, TypeExpr)],
) -> core::fmt::Result {
    writeln!(
        out,
        "static inline {id}_t* {id}_new({}) {{",
        argument_list(fields, ns)
    )
    .unwrap();
    writeln!(
        out,
        "    {id}_t* __result = ({id}_t*)calloc(1, sizeof({id}_t));"
    )
    .unwrap();
    writeln!(out, "    if (!__result) return __result;").unwrap();
    for (fname, _) in fields {
        writeln!(out, "    __result->{fname} = {fname};")?;
    }
    writeln!(out, "    return __result;").unwrap();
    writeln!(out, "}}\n").unwrap();
    Ok(())
}

fn argument_list(fields: &[(String, TypeExpr)], ns: &str) -> String {
    let args = fields
        .into_iter()
        .map(|(fname, ct)| format!("{} {fname}", ct.c_type(ns)))
        .reduce(|a, b| a + ", " + &b)
        .unwrap_or_default();
    args
}

fn struct_unref(
    out: &mut impl Write,
    id: &str,
    ns: &str,
    fields: &[(String, TypeExpr)],
) -> core::fmt::Result {
    writeln!(out, "static inline void {id}_unref({id}_t* o) {{").unwrap();
    writeln!(out, "    if (!o) return;").unwrap();
    for (fname, ct) in fields {
        if ct.needs_heap_free() {
            writeln!(out, "    {}_unref(o->{fname});", ct.id(ns))?;
        }
    }
    writeln!(out, "    free(o);").unwrap();
    writeln!(out, "}}\n").unwrap();
    Ok(())
}

fn struct_from_json(
    out: &mut impl Write,
    id: &str,
    ns: &str,
    fields: &[(String, TypeExpr)],
) -> core::fmt::Result {
    writeln!(
        out,
        "static inline int {id}_from_json(dc_json_t json, {id}_t** out) {{"
    )
    .unwrap();
    writeln!(out, "    if (!dc_json_is_valid(json)) return -1;").unwrap();
    writeln!(out, "    *out = ({id}_t*)calloc(1, sizeof({id}_t));").unwrap();
    for (fname, ct) in fields {
        writeln!(
            out,
            r#"    if ({}_from_json(dc_json_get(json, "{fname}"), &(*out)->{fname})) {{ {id}_unref(*out); return -1; }};"#,
            ct.id(ns),
        )?;
    }
    writeln!(out, "    return 0;").unwrap();
    writeln!(out, "}}\n").unwrap();
    Ok(())
}

fn struct_to_json(
    out: &mut impl Write,
    id: &str,
    ns: &str,
    fields: &[(String, TypeExpr)],
) -> core::fmt::Result {
    let mut body = String::new();
    writeln!(out, "static inline dc_json_t {id}_to_json({id}_t* o) {{")?;
    if fields.is_empty() {
        writeln!(out, "    (void)o;")?;
    }
    writeln!(out, "    dc_json_t obj = dc_json_new_object();")?;
    for (fname, ct) in fields {
        writeln!(
            out,
            r#"    obj = dc_json_add_to_object(obj, "{fname}", {}_to_json(o->{fname}));"#,
            ct.id(ns)
        )?;
    }
    writeln!(body, "    return obj;")?;
    write!(out, "{body}}}")?;
    writeln!(out, "\n")?;
    Ok(())
}

fn tagged_enum_def(out: &mut impl Write, id: &str, ns: &str, e: &EnumDef) -> core::fmt::Result {
    let id_u = id.to_uppercase();
    writeln!(out, "typedef enum {{").unwrap();
    for v in &e.variants {
        writeln!(out, "    {id_u}_{},", v.tag_value.to_case(ScreamingSnake)).unwrap();
    }
    writeln!(out, "}} {id}_kind_t;\n").unwrap();
    let has_data = e.variants.iter().any(|v| !v.fields.is_empty());
    writeln!(out, "typedef struct {id} {{").unwrap();
    writeln!(out, "    {id}_kind_t {};", e.tag_field).unwrap();
    if has_data {
        writeln!(out, "    union {{").unwrap();
        for v in &e.variants {
            if v.fields.is_empty() {
                continue;
            }
            let vname = v.tag_value.to_case(Snake);
            writeln!(out, "        struct {{").unwrap();
            for (fname, ct) in &v.fields {
                writeln!(out, "            {} {fname};", ct.c_type(ns)).unwrap();
            }
            writeln!(out, "        }} {vname};").unwrap();
        }
        writeln!(out, "    }} data;").unwrap();
    }
    writeln!(out, "}} {id}_t;\n").unwrap();
    Ok(())
}

fn tagged_enum_new(out: &mut impl Write, id: &str) -> core::fmt::Result {
    writeln!(out, "static inline {id}_t* {id}_new() {{")?;
    writeln!(
        out,
        "    {id}_t* __result = ({id}_t*)calloc(1, sizeof({id}_t));"
    )?;
    writeln!(out, "    return __result;")?;
    writeln!(out, "}}")?;
    Ok(())
}

fn tagged_enum_unref(out: &mut impl Write, id: &str, ns: &str, e: &EnumDef) -> core::fmt::Result {
    let id_u = id.to_uppercase();
    writeln!(out, "static inline void {id}_unref({id}_t* o) {{").unwrap();
    writeln!(out, "    if (!o) return;").unwrap();
    let mut variants_to_free = e
        .variants
        .iter()
        .filter(|v| v.fields.iter().any(|(_, ct)| ct.needs_heap_free()))
        .peekable();
    if variants_to_free.peek().is_some() {
        writeln!(out, "    switch (o->{}) {{", e.tag_field).unwrap();
        for v in variants_to_free {
            let vname = v.tag_value.to_case(Snake);
            let vtag = v.tag_value.to_case(ScreamingSnake);
            writeln!(out, "    case {id_u}_{vtag}:").unwrap();
            for (fname, ct) in &v.fields {
                if ct.needs_heap_free() {
                    writeln!(out, "        {}_unref(o->data.{vname}.{fname});", ct.id(ns))?;
                }
            }
            writeln!(out, "        break;").unwrap();
        }
        writeln!(out, "    default: break;").unwrap();
        writeln!(out, "    }}").unwrap();
    }
    writeln!(out, "    free(o);").unwrap();
    writeln!(out, "}}\n").unwrap();
    Ok(())
}

fn tagged_enum_from_json(
    out: &mut impl Write,
    id: &str,
    ns: &str,
    e: &EnumDef,
) -> core::fmt::Result {
    let id_u = id.to_uppercase();
    writeln!(
        out,
        "static inline int {id}_from_json(dc_json_t json, {id}_t** out) {{"
    )
    .unwrap();
    writeln!(out, "    if (!dc_json_is_valid(json)) return -1;").unwrap();
    writeln!(out, "    *out = ({id}_t*)calloc(1, sizeof({id}_t));").unwrap();
    writeln!(
        out,
        "    dc_json_t _tag = dc_json_get(json, \"{}\");",
        e.tag_field
    )
    .unwrap();
    writeln!(
        out,
        "    if (!dc_json_is_string(_tag)) {{ {id}_unref(*out); return -1; }}"
    )
    .unwrap();
    for (i, v) in e.variants.iter().enumerate() {
        let vtag = v.tag_value.to_case(ScreamingSnake);
        let vname = v.tag_value.to_case(Snake);
        let prefix = if i == 0 { "if" } else { "} else if" };
        writeln!(
            out,
            "    {prefix} (dc_json_string_eq(_tag, \"{}\")) {{",
            v.tag_value
        )
        .unwrap();
        writeln!(out, "        (*out)->{} = {id_u}_{vtag};", e.tag_field).unwrap();
        for (fname, ct) in &v.fields {
            writeln!(
                out,
                r#"    if ({}_from_json(dc_json_get(json, "{fname}"), &(*out)->data.{vname}.{fname})) {{ {id}_unref(*out); return -1; }};"#,
                ct.id(ns)
            )?;
        }
    }
    if !e.variants.is_empty() {
        writeln!(out, "    }} else {{").unwrap();
        writeln!(out, "        {id}_unref(*out);").unwrap();
        writeln!(out, "        return -1;").unwrap();
        writeln!(out, "    }}").unwrap();
    }
    writeln!(out, "    return 0;").unwrap();
    writeln!(out, "}}\n").unwrap();
    Ok(())
}

fn tagged_enum_to_json(out: &mut impl Write, id: &str, ns: &str, e: &EnumDef) -> core::fmt::Result {
    let id_u = id.to_uppercase();
    writeln!(out, "static inline dc_json_t {id}_to_json({id}_t* o) {{").unwrap();
    writeln!(out, "    dc_json_t obj = dc_json_new_object();").unwrap();
    writeln!(out, "    switch (o->{}) {{", e.tag_field).unwrap();
    for v in &e.variants {
        let vname = v.tag_value.to_case(Snake);
        let vtag = v.tag_value.to_case(ScreamingSnake);
        writeln!(out, "    case {id_u}_{vtag}: {{").unwrap();
        writeln!(
            out,
            "        obj = dc_json_add_to_object(obj, \"{}\", dc_json_new_string(\"{}\"));",
            e.tag_field, v.tag_value
        )
        .unwrap();
        for (fname, ct) in &v.fields {
            writeln!(
                out,
                r#"        obj = dc_json_add_to_object(obj, "{fname}", {}_to_json(o->data.{vname}.{fname}));"#,
                ct.id(ns)
            )
            .unwrap();
        }
        writeln!(out, "        break;").unwrap();
        writeln!(out, "    }}").unwrap();
    }
    writeln!(out, "    }}").unwrap();
    writeln!(out, "    return obj;").unwrap();
    writeln!(out, "}}\n").unwrap();
    Ok(())
}
