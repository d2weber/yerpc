use convert_case::{Case::Snake, Casing};
use core::fmt::Write;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use type_def::{EnumDef, EnumVariant, TypeDef, TypeDefInner};
use typescript_type_def::type_expr as ts;

mod tests;
pub(crate) mod type_def;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpr {
    Void,
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    String,
    Optional(Box<TypeExpr>),
    Array(Box<TypeExpr>),
    Struct(String),
    TaggedEnum(String),
    StringEnum(String),
    Tuple(Vec<TypeExpr>),
    Map(Box<TypeExpr>),
}

impl TypeExpr {
    pub fn from_info(info: &ts::TypeInfo) -> Self {
        Self::from(info)
    }
    pub fn c_type(&self, ns: &str) -> String {
        match self {
            Self::Void => "void".into(),
            Self::Bool => "bool".into(),
            Self::I8 => "int8_t".into(),
            Self::I16 => "int16_t".into(),
            Self::I32 => "int32_t".into(),
            Self::I64 => "int64_t".into(),
            Self::U8 => "uint8_t".into(),
            Self::U16 => "uint16_t".into(),
            Self::U32 => "uint32_t".into(),
            Self::U64 => "uint64_t".into(),
            Self::F32 => "float".into(),
            Self::F64 => "double".into(),
            Self::String => "const char*".into(),
            Self::Struct(_)
            | Self::TaggedEnum(_)
            | Self::Array(_)
            | Self::Map(_)
            | Self::Tuple(_) => {
                format!("{ns}{}_t*", self.type_slug())
            }
            Self::StringEnum(_) => format!("{ns}{}_t", self.type_slug()),
            Self::Optional(inner) if inner.is_scalar() => {
                format!("{ns}{}_t", self.type_slug())
            }
            Self::Optional(inner) => inner.c_type(ns),
        }
    }

    pub(crate) fn needs_heap_free(&self) -> bool {
        match self {
            TypeExpr::String
            | TypeExpr::Struct(_)
            | TypeExpr::TaggedEnum(_)
            | TypeExpr::Array(_)
            | TypeExpr::Map(_)
            | TypeExpr::Tuple(_) => true,
            TypeExpr::Optional(inner) => inner.needs_heap_free(),
            _ => false,
        }
    }

    pub fn has_type_def(&self) -> bool {
        matches!(
            self,
            TypeExpr::Struct(_) | TypeExpr::TaggedEnum(_) | TypeExpr::StringEnum(_)
        )
    }

    pub fn id(&self, ns: &str) -> String {
        format!("{ns}{}", self.type_slug())
    }

    pub fn type_slug(&self) -> String {
        match self {
            Self::Bool => "bool".into(),
            Self::I8 => "i8".into(),
            Self::I16 => "i16".into(),
            Self::I32 => "i32".into(),
            Self::I64 => "i64".into(),
            Self::U8 => "u8".into(),
            Self::U16 => "u16".into(),
            Self::U32 => "u32".into(),
            Self::U64 => "u64".into(),
            Self::F32 => "f32".into(),
            Self::F64 => "f64".into(),
            Self::String => "string".into(),
            Self::Struct(s) | TypeExpr::TaggedEnum(s) | Self::StringEnum(s) => s.to_case(Snake),
            Self::Array(inner) => format!("array_{}", inner.type_slug()),
            Self::Map(val) => format!("map_{}", val.type_slug()),
            Self::Tuple(elems) => {
                let parts: Vec<_> = elems.iter().map(|e| e.type_slug()).collect();
                format!("tuple_{}", parts.join("_"))
            }
            Self::Optional(inner) => format!("optional_{}", inner.type_slug()),
            TypeExpr::Void => "void".into(),
        }
    }

    pub fn is_scalar(&self) -> bool {
        matches!(
            self,
            Self::Bool
                | Self::StringEnum(_)
                | Self::I8
                | Self::I16
                | Self::I32
                | Self::I64
                | Self::U8
                | Self::U16
                | Self::U32
                | Self::U64
                | Self::F32
                | Self::F64
        )
    }

    pub fn zero_val(&self, ns: &str) -> String {
        let ns_upper = ns.to_uppercase();
        match self {
            TypeExpr::Bool => "false".into(),
            TypeExpr::StringEnum(s) => {
                format!("{ns_upper}{}_UNPARSED", s.to_case(Snake).to_uppercase())
            }
            TypeExpr::Optional(inner) if inner.is_scalar() => {
                let prefix = inner.type_slug();
                format!("({ns}optional_{prefix}_t){{false, {}}}", inner.zero_val(ns))
            }
            TypeExpr::String => r#""""#.to_owned(),
            TypeExpr::Struct(_) | TypeExpr::Array(_) | TypeExpr::Tuple(_) | TypeExpr::Map(_) => {
                "NULL".into()
            }
            _ => "0".into(),
        }
    }
}

fn ctype_from_expr(expr: &ts::TypeExpr) -> TypeExpr {
    match expr {
        ts::TypeExpr::Ref(info) => TypeExpr::from(*info),
        ts::TypeExpr::Name(n) => match n.name.0 {
            "number" => TypeExpr::F64,
            "string" => TypeExpr::String,
            "boolean" => TypeExpr::Bool,
            "null" | "undefined" | "void" | "never" => TypeExpr::Void,
            "Record" if n.generic_args.len() == 2 => {
                TypeExpr::Map(Box::new(ctype_from_expr(&n.generic_args[1])))
            }
            other => TypeExpr::Struct(other.to_string()),
        },
        ts::TypeExpr::Array(a) => TypeExpr::Array(Box::new(ctype_from_expr(a.item))),
        ts::TypeExpr::Union(u) => {
            let non_null: Vec<_> = u
                .members
                .iter()
                .filter(|m| !matches!(m, ts::TypeExpr::Name(n) if n.name.0 == "null"))
                .collect();
            if non_null.len() < u.members.len() && non_null.len() == 1 {
                TypeExpr::Optional(Box::new(ctype_from_expr(non_null[0])))
            } else if non_null.len() == 1 {
                ctype_from_expr(non_null[0])
            } else {
                TypeExpr::Void // TODO: tagged union
            }
        }
        ts::TypeExpr::Tuple(t) if t.elements.is_empty() => TypeExpr::Void,
        ts::TypeExpr::Tuple(t) => {
            TypeExpr::Tuple(t.elements.iter().map(|e| ctype_from_expr(e)).collect())
        }
        ts::TypeExpr::String(_) => TypeExpr::String,
        _ => TypeExpr::Void,
    }
}

impl From<&ts::TypeInfo> for TypeExpr {
    fn from(info: &ts::TypeInfo) -> Self {
        match info {
            ts::TypeInfo::Native(n) => ctype_from_expr(&n.r#ref),
            ts::TypeInfo::Defined(d) => match d.def.name.0 {
                "U8" => TypeExpr::U8,
                "U16" => TypeExpr::U16,
                "U32" => TypeExpr::U32,
                "U64" => TypeExpr::U64,
                "I8" => TypeExpr::I8,
                "I16" => TypeExpr::I16,
                "I32" => TypeExpr::I32,
                "I64" => TypeExpr::I64,
                "F32" => TypeExpr::F32,
                "F64" => TypeExpr::F64,
                "Usize" | "Isize" => TypeExpr::U64,
                _ => ctype_from_defined(d),
            },
        }
    }
}

pub fn is_string_enum(u: &ts::TypeUnion) -> bool {
    !u.members.is_empty()
        && u.members
            .iter()
            .all(|m| matches!(m, ts::TypeExpr::String(_)))
}

fn ctype_from_defined(d: &ts::DefinedTypeInfo) -> TypeExpr {
    match &d.def.def {
        ts::TypeExpr::Object(_) => TypeExpr::Struct(d.def.name.0.to_string()),
        ts::TypeExpr::Union(u) if is_string_enum(u) => {
            TypeExpr::StringEnum(d.def.name.0.to_string())
        }
        ts::TypeExpr::Union(_) => TypeExpr::TaggedEnum(d.def.name.0.to_string()),
        other => ctype_from_expr(other),
    }
}

fn walk_expr(
    expr: &'static ts::TypeExpr,
    seen: &mut HashSet<&'static str>,
    out: &mut Vec<&'static ts::TypeDefinition>,
) {
    match expr {
        ts::TypeExpr::Ref(info) => walk_info(info, seen, out),
        ts::TypeExpr::Array(a) => walk_expr(a.item, seen, out),
        ts::TypeExpr::Tuple(t) => {
            for e in t.elements {
                walk_expr(e, seen, out);
            }
        }
        ts::TypeExpr::Object(o) => {
            for f in o.fields {
                walk_expr(&f.r#type, seen, out);
            }
        }
        ts::TypeExpr::Union(u) => {
            for m in u.members {
                walk_expr(m, seen, out);
            }
        }
        ts::TypeExpr::Intersection(i) => {
            for m in i.members {
                walk_expr(m, seen, out);
            }
        }
        ts::TypeExpr::Name(n) => {
            for g in n.generic_args {
                walk_expr(g, seen, out);
            }
        }
        ts::TypeExpr::String(_) => {}
    }
}

fn walk_info(
    info: &'static ts::TypeInfo,
    seen: &mut HashSet<&'static str>,
    out: &mut Vec<&'static ts::TypeDefinition>,
) {
    match info {
        ts::TypeInfo::Native(n) => walk_expr(&n.r#ref, seen, out),
        ts::TypeInfo::Defined(d) => {
            for g in d.generic_args {
                walk_expr(g, seen, out);
            }
            if !seen.insert(d.def.name.0) {
                return;
            }
            walk_expr(&d.def.def, seen, out);
            if matches!(&d.def.def, ts::TypeExpr::Object(_) | ts::TypeExpr::Union(_)) {
                out.push(&d.def);
            }
        }
    }
}

pub fn collect_type_defs<T: typescript_type_def::TypeDef>() -> Vec<TypeDef> {
    let mut seen = HashSet::new();
    let mut raw = vec![];
    walk_info(&T::INFO, &mut seen, &mut raw);
    raw.into_iter()
        .filter_map(|d| {
            {
                let name = d.name.0.to_string();
                match &d.def {
                    ts::TypeExpr::Object(obj)
                        if obj.index_signature.is_some() && obj.fields.is_empty() =>
                    {
                        Err(())
                    }
                    ts::TypeExpr::Object(obj) => Ok(TypeDef {
                        name,
                        def: TypeDefInner::Struct {
                            fields: generate_struct_fields(obj),
                        },
                    }),
                    ts::TypeExpr::Union(u) => {
                        if let Some(variants) = get_string_enum_variants(u) {
                            Ok(TypeDef {
                                name,
                                def: TypeDefInner::StringEnum { variants },
                            })
                        } else if let Some(def) = parse_internally_tagged_union(u) {
                            Ok(TypeDef {
                                name,
                                def: TypeDefInner::TaggedEnum { def },
                            })
                        } else {
                            panic!("Could not convert union {u:?}",);
                        }
                    }
                    _ => Err(()),
                }
            }
            .ok()
        })
        .collect()
}

pub fn resolve_to_object<'a>(expr: &'a ts::TypeExpr) -> Option<&'a ts::TypeObject> {
    match expr {
        ts::TypeExpr::Object(o) => Some(o),
        ts::TypeExpr::Ref(ts::TypeInfo::Defined(d)) => resolve_to_object(&d.def.def),
        _ => None,
    }
}

fn parse_internally_tagged_union(u: &ts::TypeUnion) -> Option<EnumDef> {
    if u.members.is_empty() {
        return None;
    }
    let mut tag_field = None;
    let mut variants = vec![];
    for m in u.members {
        match m {
            ts::TypeExpr::Object(obj) if obj.fields.len() == 1 => {
                let f = &obj.fields[0];
                if let ts::TypeExpr::String(s) = &f.r#type {
                    let tf = f.name.value.to_string();
                    if tag_field.get_or_insert_with(|| tf.clone()) != &tf {
                        return None;
                    }
                    variants.push(EnumVariant {
                        tag_value: s.value.to_string(),
                        fields: vec![],
                    });
                } else {
                    return None;
                }
            }
            ts::TypeExpr::Intersection(i) if i.members.len() == 2 => {
                let tag_obj = match &i.members[0] {
                    ts::TypeExpr::Object(a) => a,
                    _ => return None,
                };
                let data_obj = match resolve_to_object(&i.members[1]) {
                    Some(o) => o,
                    None => return None,
                };
                if tag_obj.fields.len() != 1 {
                    return None;
                }
                let f = &tag_obj.fields[0];
                if let ts::TypeExpr::String(s) = &f.r#type {
                    let tf = f.name.value.to_string();
                    if tag_field.get_or_insert_with(|| tf.clone()) != &tf {
                        return None;
                    }
                    let fields = generate_struct_fields(data_obj);
                    variants.push(EnumVariant {
                        tag_value: s.value.to_string(),
                        fields,
                    });
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(EnumDef {
        tag_field: tag_field?,
        variants,
    })
}

fn generate_struct_fields(obj: &ts::TypeObject) -> Vec<(String, TypeExpr)> {
    obj.fields
        .iter()
        .map(|f| {
            let name = f.name.value.to_string();
            let mut ct = ctype_from_expr(&f.r#type);
            if f.optional && !matches!(ct, TypeExpr::Optional(_)) {
                ct = TypeExpr::Optional(Box::new(ct));
            }
            (name, ct)
        })
        .collect()
}

pub(crate) fn apply_recursive<E>(
    types: &[TypeExpr],
    mut f: impl FnMut(&TypeExpr) -> Result<(), E>,
) -> Result<(), E> {
    let mut seen: HashSet<String> = HashSet::new();
    fn walk<E>(
        ct: &TypeExpr,
        seen: &mut HashSet<String>,
        f: &mut dyn FnMut(&TypeExpr) -> Result<(), E>,
    ) -> Result<(), E> {
        match ct {
            TypeExpr::Array(inner) | TypeExpr::Map(inner) | TypeExpr::Optional(inner) => {
                walk(inner, seen, f)?;
            }
            TypeExpr::Tuple(elems) => {
                for e in elems {
                    walk(e, seen, f)?;
                }
            }
            _ => {}
        }
        if seen.insert(ct.type_slug()) {
            f(ct)?;
        }
        Ok(())
    }
    for ct in types {
        walk(ct, &mut seen, &mut f)?;
    }
    Ok(())
}

fn emit_type_expr_forward_declarations(
    out: &mut impl Write,
    types: &[TypeExpr],
    ns: &str,
) -> core::fmt::Result {
    apply_recursive(types, |t| {
        if t.is_scalar() | t.has_type_def() {
            return Ok(());
        };
        writeln!(out, "typedef struct {id} {id}_t;", id = t.id(ns))
    })
}

fn emit_type_expr_early(out: &mut impl Write, types: &[TypeExpr], ns: &str) -> core::fmt::Result {
    apply_recursive(types, |t| {
        if t.has_type_def() {
            return Ok(());
        };
        let id = t.id(ns);
        let ct = t.c_type(ns);
        match t {
            TypeExpr::Array(inner) => {
                writeln!(out, "typedef struct {id} {{")?;
                writeln!(out, "    {}* items;", inner.c_type(ns))?;
                writeln!(out, "    size_t len;")?;
                writeln!(out, "}} {id}_t;")?;
                writeln!(out, "static inline {id}_t* {id}_new(size_t len);")?;
                writeln!(out, "static inline void {id}_unref({ct} a);")?;
                writeln!(out)?;
            }
            TypeExpr::Map(inner) => {
                writeln!(out, "typedef struct {id} {{")?;
                writeln!(out, "    const char** keys;")?;
                writeln!(out, "    {}* values;", inner.c_type(ns))?;
                writeln!(out, "    size_t len;")?;
                writeln!(out, "}} {id}_t;")?;
                writeln!(out, "static inline {id}_t* {id}_new(size_t len);")?;
                writeln!(out, "static inline void {id}_unref({ct} m);")?;
                writeln!(out)?;
            }
            TypeExpr::Tuple(elems) => {
                writeln!(out, "typedef struct {id} {{")?;
                for (i, ct) in elems.iter().enumerate() {
                    writeln!(out, "    {} field_{i};", ct.c_type(ns))?;
                }
                writeln!(out, "}} {id}_t;")?;
                writeln!(
                    out,
                    "static inline {id}_t* {id}_new({});",
                    elems
                        .iter()
                        .map(|t| t.c_type(ns))
                        .reduce(|a, b| a + ", " + &b)
                        .unwrap_or_default()
                )?;
                writeln!(out, "static inline void {id}_unref({ct} m);")?;
                writeln!(out)?;
            }
            TypeExpr::Optional(inner) if inner.is_scalar() => {
                writeln!(out, "typedef struct {id} {{")?;
                writeln!(out, "    bool has_value;")?;
                writeln!(out, "    {} value;", inner.c_type(ns))?;
                writeln!(out, "}} {id}_t;\n")?;
            }
            TypeExpr::Optional(inner) => {
                writeln!(
                    out,
                    "static inline {id}_t* {id}_new({} x);",
                    inner.c_type(ns)
                )?;
                writeln!(out, "static inline void {id}_unref({ct} o);")?;
            }
            _ => (),
        };
        writeln!(out, "static inline dc_json_t {id}_to_json({ct} o);")?;
        writeln!(
            out,
            "static inline int {id}_from_json(dc_json_t v, {ct}* r);"
        )?;
        Ok(())
    })
}

fn emit_type_expr_late(out: &mut impl Write, types: &[TypeExpr], ns: &str) -> core::fmt::Result {
    apply_recursive(types, |t| {
        if t.has_type_def() {
            return Ok(());
        };
        emit_container_struct_and_unref(out, ns, t)?;
        emit_from_json(out, t, ns)?;
        emit_to_json(out, t, ns)
    })
}

fn emit_container_struct_and_unref(
    out: &mut impl Write,
    ns: &str,
    t: &TypeExpr,
) -> Result<(), std::fmt::Error> {
    let id = t.id(ns);
    let ct = t.c_type(ns);
    Ok(match t {
        TypeExpr::Array(inner) => {
            writeln!(out, "static inline {id}_t* {id}_new(size_t len) {{")?;
            writeln!(
                out,
                "    {id}_t* __result = ({id}_t*)calloc(1, sizeof({id}_t));"
            )?;
            writeln!(out, "    __result->len = len;")?;
            writeln!(
                out,
                "    __result->items = ({0}*)calloc(len, sizeof({0}));",
                inner.c_type(ns)
            )?;
            writeln!(out, "    return __result;")?;
            writeln!(out, "}}")?;
            writeln!(out)?;
            writeln!(out, "static inline void {id}_unref({ct} a) {{")?;
            writeln!(out, "    if (!a) return;")?;
            if inner.needs_heap_free() {
                writeln!(out, "    for (size_t i = 0; i < a->len; i++) {{")?;
                writeln!(out, "        {}_unref(a->items[i]);", inner.id(ns))?;
                writeln!(out, "    }}")?;
            }
            writeln!(out, "    free(a->items);")?;
            writeln!(out, "    free(a);")?;
            writeln!(out, "}}")?;
            writeln!(out)?;
        }
        TypeExpr::Map(inner) => {
            writeln!(out, "static inline {id}_t* {id}_new(size_t len) {{")?;
            writeln!(
                out,
                "    {id}_t* __result = ({id}_t*)calloc(1, sizeof({id}_t));"
            )?;
            writeln!(out, "    __result->len = len;")?;
            writeln!(
                out,
                "    __result->keys = (const char**)calloc(len, sizeof(char*));"
            )?;
            writeln!(
                out,
                "    __result->values = ({0}*)calloc(len, sizeof({0}));",
                inner.c_type(ns)
            )?;
            writeln!(out, "    return __result;")?;
            writeln!(out, "}}")?;
            writeln!(out)?;
            writeln!(out, "static inline void {id}_unref({ct} m) {{")?;
            writeln!(out, "    if (!m) return;")?;
            writeln!(out, "    for (size_t i = 0; i < m->len; i++) {{")?;
            writeln!(out, "        {ns}string_unref(m->keys[i]);")?;
            if inner.needs_heap_free() {
                writeln!(out, "        {}_unref(m->values[i]);", inner.id(ns))?;
            }
            writeln!(out, "    }}")?;
            writeln!(out, "    free(m->keys);")?;
            writeln!(out, "    free(m->values);")?;
            writeln!(out, "    free(m);")?;
            writeln!(out, "}}")?;
            writeln!(out)?;
        }
        TypeExpr::Tuple(elems) => {
            writeln!(out, "static inline void {id}_unref({ct} o) {{")?;
            writeln!(out, "    if (!o) return;")?;
            for (i, ct) in elems
                .iter()
                .enumerate()
                .filter(|(_, t)| t.needs_heap_free())
            {
                writeln!(out, "    {}_unref(o->field_{i});", ct.id(ns))?;
            }
            writeln!(out, "    free(o);")?;
            writeln!(out, "}}")?;
            writeln!(
                out,
                "static inline {id}_t* {id}_new({}) {{",
                elems
                    .iter()
                    .enumerate()
                    .map(|(i, t)| format!("{} field_{i}", t.c_type(ns)))
                    .reduce(|a, b| a + ", " + &b)
                    .unwrap_or_default()
            )?;
            writeln!(
                out,
                "    {id}_t* __result = ({id}_t*)calloc(1, sizeof({id}_t));"
            )?;
            for i in 0..elems.len() {
                writeln!(out, "    __result->field_{i} = field_{i};")?;
            }
            writeln!(out, "    return __result;")?;
            writeln!(out, "}}")?;
        }
        TypeExpr::Optional(inner) if !inner.is_scalar() => {
            writeln!(
                out,
                "static inline {id}_t* {id}_new({} x) {{",
                inner.c_type(ns)
            )?;
            writeln!(out, "    return ({id}_t*)(x);")?;
            writeln!(out, "}}")?;
            writeln!(out, "static inline void {id}_unref({ct} o) {{")?;
            writeln!(out, "    {}_unref(o);", inner.id(ns))?;
            writeln!(out, "}}")?;
        }
        _ => (),
    })
}

fn emit_to_json(o: &mut impl Write, t: &TypeExpr, ns: &str) -> core::fmt::Result {
    let id = t.id(ns);
    let ct = t.c_type(ns);
    writeln!(o, "static inline dc_json_t {id}_to_json({ct} o) {{")?;
    match t {
        TypeExpr::Bool => writeln!(o, "    return dc_json_new_bool(o);")?,
        TypeExpr::I8
        | TypeExpr::I16
        | TypeExpr::I32
        | TypeExpr::I64
        | TypeExpr::U8
        | TypeExpr::U16
        | TypeExpr::U32
        | TypeExpr::U64
        | TypeExpr::F32
        | TypeExpr::F64 => writeln!(o, "    return dc_json_new_number((double)o);")?,
        TypeExpr::String => writeln!(o, "    return dc_json_new_string(o);")?,
        TypeExpr::Optional(inner) if inner.is_scalar() => writeln!(
            o,
            "    return o.has_value ? {}_to_json(o.value) : dc_json_new_null();",
            inner.id(ns)
        )?,

        TypeExpr::Optional(inner) => writeln!(
            o,
            "    return o ? {}_to_json(o): dc_json_new_null();",
            inner.id(ns)
        )?,
        TypeExpr::Array(inner) => {
            writeln!(o, "    dc_json_t json = dc_json_new_array();")?;
            writeln!(o, "    for (size_t i = 0; i < o->len; i++) {{")?;
            writeln!(
                o,
                "        dc_json_add_to_array(json, {}_to_json(o->items[i]));",
                inner.id(ns)
            )?;
            writeln!(o, "    }}")?;
            writeln!(o, "    return json;")?;
        }
        TypeExpr::Tuple(elems) => {
            writeln!(o, "    dc_json_t json = dc_json_new_array();")?;
            for (i, ct) in elems.iter().enumerate() {
                writeln!(
                    o,
                    "        dc_json_add_to_array(json, {}_to_json(o->field_{i}));",
                    ct.id(ns)
                )?;
            }
            writeln!(o, "    return json;")?;
        }
        TypeExpr::Map(inner) => {
            writeln!(o, "    dc_json_t json = dc_json_new_object();")?;
            writeln!(o, "    if (o) for (size_t i = 0; i < o->len; i++) {{")?;
            writeln!(
                o,
                "        dc_json_add_to_object(json, o->keys[i], {}_to_json(o->values[i]));",
                inner.id(ns)
            )?;
            writeln!(o, "    }}")?;
            writeln!(o, "    return json;")?;
        }
        TypeExpr::Void
        | TypeExpr::Struct(_)
        | TypeExpr::TaggedEnum(_)
        | TypeExpr::StringEnum(_) => panic!(),
    };
    writeln!(o, "}}")
}

fn emit_from_json(o: &mut impl Write, t: &TypeExpr, ns: &str) -> core::fmt::Result {
    let id = t.id(ns);
    let ct = t.c_type(ns);
    writeln!(
        o,
        "static inline int {id}_from_json(dc_json_t v, {ct}* r) {{"
    )?;
    match t {
        TypeExpr::Bool => {
            writeln!(o, "    if (!dc_json_is_bool(v)) return -1;")?;
            writeln!(o, "    *r = dc_json_get_bool(v);")?;
        }
        TypeExpr::I8
        | TypeExpr::I16
        | TypeExpr::I32
        | TypeExpr::I64
        | TypeExpr::U8
        | TypeExpr::U16
        | TypeExpr::U32
        | TypeExpr::U64
        | TypeExpr::F32
        | TypeExpr::F64 => {
            writeln!(o, "    if (!dc_json_is_number(v)) return -1;")?;
            writeln!(o, "    *r = ({ct})dc_json_get_double(v);")?;
        }
        TypeExpr::String => {
            writeln!(o, "    if (!dc_json_is_string(v)) return -1;")?;
            writeln!(o, "    *r = dc_json_copy_string(v);")?;
        }
        TypeExpr::Optional(inner) => {
            writeln!(o, "    if (!dc_json_is_valid(v) || dc_json_is_null(v)) {{")?;
            writeln!(o, "        *r = {};", t.zero_val(ns))?;
            writeln!(o, "        return 0;")?;
            writeln!(o, "    }}")?;
            if inner.is_scalar() {
                writeln!(
                    o,
                    "    if ({}_from_json(v, &r->value)) {{ return -1; }};",
                    inner.id(ns)
                )?;
                writeln!(o, "    r->has_value = true;")?;
            } else {
                writeln!(
                    o,
                    "    if ({}_from_json(v, r)) {{ return -1; }};",
                    inner.id(ns)
                )?;
            }
        }
        TypeExpr::Array(inner) => {
            writeln!(o, "    if (!dc_json_is_array(v)) return -1;").unwrap();
            writeln!(o, "    *r = ({ct})calloc(1, sizeof({id}_t));").unwrap();
            writeln!(o, "    (*r)->len = (size_t)dc_json_len(v);").unwrap();
            writeln!(
                o,
                "(*r)->items = ({}*)calloc((*r)->len, sizeof((*r)->items[0]));",
                inner.c_type(ns)
            )?;
            writeln!(
                o,
                "    dc_json_array_iter_t it = dc_json_array_iter_new(v);"
            )?;
            writeln!(
                o,
                "    for (size_t i = 0; i < (*r)->len; i++, it = dc_json_array_iter_next(it)) {{"
            )?;
            writeln!(
                o,
                "        if (!dc_json_array_iter_is_valid(it)) return -1;"
            )?;
            writeln!(
                    o,
                    "        if ({}_from_json(dc_json_array_iter_value(it), &(*r)->items[i])) {{ {id}_unref(*r); return -1; }};",
                    inner.id(ns)
                )?;
            writeln!(o, "    }}")?;
        }
        TypeExpr::Tuple(elems) => {
            writeln!(o, "    if (!dc_json_is_array(v)) return -1;").unwrap();
            writeln!(o, "    *r = ({ct})calloc(1, sizeof({id}_t));").unwrap();
            writeln!(
                o,
                "    dc_json_array_iter_t it = dc_json_array_iter_new(v);"
            )?;
            for (i, el) in elems.iter().enumerate() {
                writeln!(
                    o,
                    "        if (!dc_json_array_iter_is_valid(it)) return -1;"
                )?;
                writeln!(
                        o,
                        "    if ({}_from_json(dc_json_array_iter_value(it), &(*r)->field_{i})) {{ {id}_unref(*r); return -1; }};",
                        el.id(ns)
                    )?;
                writeln!(o, "    it = dc_json_array_iter_next(it);")?;
            }
        }
        TypeExpr::Map(inner) => {
            writeln!(o, "    if (!dc_json_is_object(v)) return -1;").unwrap();
            writeln!(o, "    *r = ({ct})calloc(1, sizeof({id}_t));").unwrap();
            writeln!(o, "    (*r)->len = (size_t)dc_json_len(v);").unwrap();
            writeln!(
                o,
                "    (*r)->keys = (const char**)calloc((*r)->len, sizeof(char*));",
            )?;
            writeln!(
                o,
                "    (*r)->values = ({}*)calloc((*r)->len, sizeof((*r)->values[0]));",
                inner.c_type(ns)
            )?;
            writeln!(o, "    dc_json_map_iter_t it = dc_json_map_iter_new(v);")?;
            writeln!(
                o,
                "    for (size_t i = 0; i < (*r)->len; i++, it = dc_json_map_iter_next(it)) {{"
            )?;
            writeln!(o, "        if (!dc_json_map_iter_is_valid(it)) return -1;")?;
            writeln!(o, "        (*r)->keys[i] = dc_json_map_iter_copy_key(it);")?;
            writeln!(o, "        if (!(*r)->keys[i]) return -1;")?;
            writeln!(
                    o,
                    "        if ({}_from_json(dc_json_map_iter_value(it), &(*r)->values[i])) {{ {id}_unref(*r); return -1; }};",
                    inner.id(ns)
                )?;
            writeln!(o, "    }}")?;
        }
        TypeExpr::Void
        | TypeExpr::Struct(_)
        | TypeExpr::TaggedEnum(_)
        | TypeExpr::StringEnum(_) => panic!(),
    };
    writeln!(o, "    return 0;")?;
    writeln!(o, "}}")
}

pub(crate) fn collect_all_type_exprs(methods: &[Method], defs: &[TypeDef]) -> Vec<TypeExpr> {
    let mut types: Vec<TypeExpr> = defs.iter().flat_map(|d| d.field_ctypes()).collect();
    for m in methods {
        if let Some(ct) = &m.output {
            types.push(ct.clone());
        }
        for (_, ct) in &m.args {
            types.push(ct.clone());
        }
    }
    types
}

fn generate_header(methods: &[Method], defs: &[TypeDef], ns: &str) -> String {
    let mut out = header_preamble(ns);
    let all_type_exprs = collect_all_type_exprs(methods, defs);
    emit_type_expr_forward_declarations(&mut out, &all_type_exprs, ns).unwrap();
    for d in defs {
        d.emit_forward_decls(&mut out, ns).unwrap();
    }
    for d in defs {
        d.emit_late_decls(&mut out, ns).unwrap();
    }
    emit_type_expr_early(&mut out, &all_type_exprs, ns).unwrap();
    for d in defs {
        d.emit_definitions(&mut out, ns).unwrap();
    }
    emit_type_expr_late(&mut out, &all_type_exprs, ns).unwrap();
    for m in methods {
        out.push_str(&m.build_fn(ns));
        if let Some(p) = m.parse_fn(ns) {
            out.push_str(&p);
        }
    }
    out
}

fn header_preamble(ns: &str) -> String {
    format!(
        r#"#pragma once

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "dc_json_cjson.h"
/* dc_json backend must be included before this header */

typedef struct {{
    const char* json;   /* serialized JSON-RPC string (owned) */
}} {ns}request_t;

static inline const char* {ns}string_new(const char* s) {{ return strdup(s); }}
static inline void {ns}string_unref(const char* s) {{ free((char*)s); }}

static inline void {ns}request_unref({ns}request_t* r) {{
    if (r) {{
        free((void*)r->json);
        free(r);
    }}
}}

typedef struct {{
    uint32_t      id;
    int32_t       error_code;     /* 0 = success, -32700 = parse fail, else JSON-RPC error */
    dc_json_t     result;         /* node in doc_, valid when error_code == 0 */
    const char*   error_message;  /* owned string or literal, valid when error_code != 0 */
    dc_json_doc_t doc_;        /* owns the parsed tree */
}} {ns}result_t;

static inline {ns}result_t* {ns}parse_response(const char* json) {{
    {ns}result_t* r = ({ns}result_t*)calloc(1, sizeof({ns}result_t));
    if (!r) return NULL;

    r->doc_ = dc_json_doc_parse(json);
    dc_json_t root = dc_json_root(r->doc_);
    if (!dc_json_is_valid(root)) {{
        r->error_code = -32700;
        r->error_message = strdup("Failed to parse JSON");
        return r;
    }}

    dc_json_t id = dc_json_get(root, "id");
    if (!dc_json_is_number(id)) {{
        r->error_code = -32700;
        r->error_message = strdup("Repsonse does not have a valid `id`");
        return r;
    }}
    r->id = (uint32_t)dc_json_get_double(id);

    dc_json_t error = dc_json_get(root, "error");
    if (dc_json_is_valid(error) && dc_json_is_object(error)) {{
        dc_json_t code = dc_json_get(error, "code");
        dc_json_t message = dc_json_get(error, "message");
        r->error_code = dc_json_is_number(code) ? (int32_t)dc_json_get_double(code) : -32700;
        r->error_message = dc_json_is_string(message) ? dc_json_copy_string(message) : strdup("Unknown error");
        return r;
    }}

    dc_json_t result = dc_json_get(root, "result");
    if (!dc_json_is_valid(result)) {{
        r->error_code = -32700;
        r->error_message = strdup("Response has neither 'result' nor 'error'");
        return r;
    }}

    r->result = result;
    return r;
}}

static inline void {ns}result_unref({ns}result_t* r) {{
    if (r) {{
        dc_json_doc_free(r->doc_);
        free((char*)r->error_message);
        free(r);
    }}
}}
"#
    )
}

pub fn write_files(outdir: &Path, methods: &[Method], defs: &[TypeDef], ns: &str) {
    fs::create_dir_all(&outdir).expect(&format!(
        "Failed to create directory `{}`",
        outdir.display()
    ));
    fs::write(&outdir.join("rpc.h"), &generate_header(methods, defs, ns))
        .expect("Failed to write rpc.h");

    let mut writer = std::io::BufWriter::new(
        fs::File::create(outdir.join("rpc.hpp")).expect("Failed to create rpc.hpp"),
    );
    crate::cpp::generate_hpp(&mut writer, methods, defs, "rpc.h", ns)
        .expect("Failed to format rpc.hpp");

    fs::write(
        &outdir.join("dc_json_decl.h"),
        include_str!("dc_json_decl.h"),
    )
    .unwrap();

    fs::write(
        &outdir.join("dc_json_cjson.h"),
        include_str!("dc_json_cjson.h"),
    )
    .unwrap();

    fs::write(
        &outdir.join("dc_json_qt.hpp"),
        include_str!("dc_json_qt.hpp"),
    )
    .unwrap();
}

pub fn get_string_enum_variants(u: &ts::TypeUnion) -> Option<Vec<String>> {
    if !is_string_enum(u) {
        return None;
    }
    Some(
        u.members
            .iter()
            .filter_map(|m| match m {
                ts::TypeExpr::String(s) => Some(s.value.to_string()),
                _ => None,
            })
            .collect(),
    )
}

#[derive(Debug, Clone)]
pub struct Method {
    pub c_name: String,
    pub rpc_name: String,
    pub args: Vec<(String, TypeExpr)>,
    pub output: Option<TypeExpr>,
    pub is_notification: bool,
    pub is_positional: bool,
}

impl Method {
    pub fn new(
        rpc_name: &str,
        args: Vec<(String, TypeExpr)>,
        output: Option<TypeExpr>,
        is_notification: bool,
        is_positional: bool,
    ) -> Self {
        Self {
            c_name: rpc_name.to_case(Snake),
            rpc_name: rpc_name.into(),
            args,
            output: output.filter(|ct| *ct != TypeExpr::Void),
            is_notification,
            is_positional,
        }
    }

    pub fn build_fn(&self, ns: &str) -> String {
        let mut c_params: Vec<String> = if self.is_notification {
            vec![]
        } else {
            vec!["uint32_t _rpc_id".into()]
        };
        c_params.extend(
            self.args
                .iter()
                .map(|(name, ty)| format!("{} {name}", ty.c_type(ns))),
        );
        let mut body = String::new();
        writeln!(body, "    dc_json_doc_t doc = dc_json_doc_new();").unwrap();
        writeln!(body, "    dc_json_t req = dc_json_root(doc);").unwrap();
        writeln!(
            body,
            "    req = dc_json_add_to_object(req, \"jsonrpc\", dc_json_new_string(\"2.0\"));"
        )
        .unwrap();
        if !self.is_notification {
            writeln!(body, "    req = dc_json_add_to_object(req, \"id\", dc_json_new_number((double)_rpc_id));").unwrap();
        }
        writeln!(
            body,
            "    req = dc_json_add_to_object(req, \"method\", dc_json_new_string(\"{}\"));",
            self.rpc_name
        )
        .unwrap();
        if !self.args.is_empty() {
            if self.is_positional {
                writeln!(body, "    dc_json_t params = dc_json_new_array();").unwrap();
                for (name, ty) in &self.args {
                    writeln!(
                        body,
                        "    dc_json_add_to_array(params, {}_to_json({name}));",
                        ty.id(ns)
                    )
                    .unwrap();
                }
                writeln!(
                    body,
                    "    req = dc_json_add_to_object(req, \"params\", params);"
                )
                .unwrap();
            } else {
                writeln!(body, "    dc_json_t params = dc_json_new_object();").unwrap();
                for (name, ty) in &self.args {
                    writeln!(
                        body,
                        "    params = dc_json_add_to_object(params, \"{name}\", {}_to_json({name}));",
                        ty.id(ns)
                    )
                    .unwrap();
                }
                writeln!(
                    body,
                    "    req = dc_json_add_to_object(req, \"params\", params);"
                )
                .unwrap();
            }
        }
        writeln!(
            body,
            "    {ns}request_t* r = ({ns}request_t*)calloc(1, sizeof({ns}request_t));"
        )
        .unwrap();
        writeln!(body, "    r->json = dc_json_print(req);").unwrap();
        writeln!(body, "    dc_json_doc_free(doc);").unwrap();
        writeln!(body, "    return r;").unwrap();
        format!(
            "static inline {ns}request_t* {ns}build_{}({}) {{\n{}}}\n",
            self.c_name,
            c_params.join(", "),
            body
        )
    }

    pub fn parse_fn(&self, ns: &str) -> Option<String> {
        let ct = self.output.as_ref()?;
        if self.is_notification {
            return None;
        }
        let ret_type = ct.c_type(ns);
        let zero = ct.zero_val(ns);
        let mut out = String::new();
        writeln!(
            out,
            "static inline {ret_type} {ns}parse_{}({ns}result_t* r) {{",
            self.c_name
        )
        .unwrap();
        writeln!(out, "    if (!r || r->error_code != 0) return {zero};").unwrap();
        writeln!(out, "    {ret_type} o;").unwrap();
        writeln!(out, "    if ({}_from_json(r->result, &o)) {{", ct.id(ns)).unwrap();
        writeln!(out, "        r->error_code = -32700;").unwrap();
        writeln!(out, "        r->error_message = strdup(\"parse error\");").unwrap();
        writeln!(out, "        return {zero};").unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "    return o;").unwrap();
        writeln!(out, "}}").unwrap();
        Some(out)
    }
}
