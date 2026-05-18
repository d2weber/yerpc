use super::*;
use anyhow::{bail, Context, Result};
use proptest::{prelude::*, sample};
use std::{
    env::consts::EXE_SUFFIX,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn compile_and_run(
    main_c: &str,
    methods: &[Method],
    defs: &[TypeDef],
    ns: &str,
) -> Result<()> {
    let mut dir = tempfile::Builder::new().disable_cleanup(true).tempdir()?;
    write_files(dir.path(), methods, defs, ns);
    let src = format!("#include \"dc_json_cjson.h\"\n#include \"rpc.h\"\n#include <stdio.h>\n#include <string.h>\n{main_c}");
    std::fs::write(dir.path().join("main.c"), &src)?;
    let CompiledDeps {
        includes,
        object_files,
    } = compiled_deps();
    let exe = link_exe(dir.path(), &["main.c"], &object_files, &includes)?;
    run_exe(&exe)?;
    dir.disable_cleanup(false); // Only cleanup if there has been no error
    Ok(())
}

pub(crate) fn arb_json_roundtrip() -> impl Strategy<Value = (TypeExpr, Vec<TypeDef>, String)> {
    arb_typedef().prop_flat_map(|(ct, defs)| {
        let json = arb_json_for_ctype(&ct, &defs);
        (Just(ct), Just(defs), json)
    })
}

fn arb_typedef() -> impl Strategy<Value = (TypeExpr, Vec<TypeDef>)> {
    let leaf = prop_oneof![
        Just(TypeExpr::Bool),
        Just(TypeExpr::U32),
        Just(TypeExpr::I64),
        Just(TypeExpr::F64),
        Just(TypeExpr::String),
    ]
    .prop_map(|ct| (ct, Vec::<TypeDef>::new()))
    .boxed()
    .prop_union(
        arb_string_enum_def()
            .prop_map(|d| (d.as_ctype(), vec![d]))
            .boxed(),
    );
    leaf.prop_recursive(3, 20, 4, move |inner| {
        prop_oneof![
            inner
                .clone()
                .prop_filter_map("No direclty nested optionals", |(ct, defs)| match ct {
                    TypeExpr::Optional(_) => None,
                    _ => Some((TypeExpr::Optional(ct.into()), defs)),
                }),
            inner
                .clone()
                .prop_map(|(ct, defs)| (TypeExpr::Array(ct.into()), defs)),
            inner
                .clone()
                .prop_map(|(ct, defs)| (TypeExpr::Map(ct.into()), defs)),
            prop::collection::vec(inner.clone(), 1..=3).prop_map(|inner| {
                (
                    TypeExpr::Tuple(inner.iter().cloned().map(|(ct, _)| ct).collect()),
                    inner.into_iter().flat_map(|(_, defs)| defs).collect(),
                )
            }),
            arb_struct(&inner),
            arb_tagged_enum(&inner)
        ]
    })
    .prop_filter_map("Only some top level Types supported", |(ct, mut defs)| {
        defs.sort_unstable_by(|a, b| a.name.cmp(&b.name));
        defs.dedup_by(|a, b| a.name == b.name);
        Some((ct, defs))
    })
}

fn arb_tagged_enum(
    inner: &BoxedStrategy<(TypeExpr, Vec<TypeDef>)>,
) -> impl Strategy<Value = (TypeExpr, Vec<TypeDef>)> {
    prop::collection::vec(prop::collection::vec(inner.clone(), 0..4), 1..3).prop_map(|inner| {
        let d = TypeDef {
            name: inner
                .iter()
                .map(|t| {
                    t.iter()
                        .map(|(ct, _)| ct.type_slug())
                        .fold("V".to_owned(), |a, b| a + "_" + &b)
                })
                .fold("TE".to_owned(), |a, b| a + "__" + &b),
            def: TypeDefInner::TaggedEnum {
                def: EnumDef {
                    tag_field: "type".to_owned(),
                    variants: inner
                        .iter()
                        .enumerate()
                        .map(|(i, t)| EnumVariant {
                            tag_value: format!("v{i}"),
                            fields: t
                                .into_iter()
                                .enumerate()
                                .map(|(j, (ct, _))| (format!("f{j}"), ct.clone()))
                                .collect(),
                        })
                        .collect(),
                },
            },
        };
        (
            d.as_ctype(),
            inner
                .into_iter()
                .flatten()
                .flat_map(|(_, defs)| defs)
                .chain(std::iter::once(d))
                .collect(),
        )
    })
}

fn arb_struct(
    inner: &BoxedStrategy<(TypeExpr, Vec<TypeDef>)>,
) -> impl Strategy<Value = (TypeExpr, Vec<TypeDef>)> {
    prop::collection::vec(inner.clone(), 0..=3).prop_map(|inner| {
        let d = TypeDef {
            name: inner
                .iter()
                .map(|(ct, _)| ct.type_slug())
                .fold("S".to_owned(), |a, b| a + "_" + &b),
            def: TypeDefInner::Struct {
                fields: inner
                    .iter()
                    .enumerate()
                    .map(|(i, (ct, _))| (format!("field_{i}"), ct.clone()))
                    .collect(),
            },
        };
        (
            d.as_ctype(),
            inner
                .into_iter()
                .flat_map(|(_, defs)| defs)
                .chain(std::iter::once(d))
                .collect(),
        )
    })
}

fn arb_string_enum_def() -> BoxedStrategy<TypeDef> {
    (2..=4usize)
        .prop_map(|n| TypeDef {
            name: format!("SE{n}"),
            def: TypeDefInner::StringEnum {
                variants: (0..n).map(|i| format!("Var{i}")).collect(),
            },
        })
        .boxed()
}

fn arb_json_for_ctype(ct: &TypeExpr, defs: &[TypeDef]) -> BoxedStrategy<String> {
    match ct {
        TypeExpr::Bool => any::<bool>().prop_map(|b| b.to_string()).boxed(),
        TypeExpr::I8 | TypeExpr::I16 | TypeExpr::I32 | TypeExpr::I64 => {
            (-100i64..100).prop_map(|n| n.to_string()).boxed()
        }
        TypeExpr::U8 | TypeExpr::U16 | TypeExpr::U32 | TypeExpr::U64 => {
            (0u64..200).prop_map(|n| n.to_string()).boxed()
        }
        TypeExpr::F32 | TypeExpr::F64 => (-100.0f64..100.0)
            .prop_map(|f| {
                format!("{f:.6}")
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_owned()
            })
            .boxed(),
        TypeExpr::String => "[a-z]{1,8}".prop_map(|s| format!(r#""{}""#, s)).boxed(),
        TypeExpr::Array(inner) => proptest::collection::vec(arb_json_for_ctype(inner, &defs), 0..4)
            .prop_map(|v| format!("[{}]", v.join(",")))
            .boxed(),
        TypeExpr::Map(val) => {
            proptest::collection::vec(("[a-z]{1,4}", arb_json_for_ctype(&val, &defs)), 0..4)
                .prop_map(|pairs| {
                    let entries: Vec<_> = pairs
                        .into_iter()
                        .map(|(k, v)| format!(r#""{}":{}"#, k, v))
                        .collect();
                    format!("{{{}}}", entries.join(","))
                })
                .boxed()
        }
        TypeExpr::Optional(inner) => {
            prop_oneof![Just("null".into()), arb_json_for_ctype(inner, defs)].boxed()
        }
        TypeExpr::Tuple(elems) => elems
            .into_iter()
            .map(|e| arb_json_for_ctype(e, defs))
            .fold(Just(vec![]).boxed(), |acc, s| {
                (acc, s)
                    .prop_map(|(mut v, e)| {
                        v.push(e);
                        v
                    })
                    .boxed()
            })
            .prop_map(|v| format!("[{}]", v.join(",")))
            .boxed(),
        TypeExpr::Struct(name) => {
            let def = defs
                .iter()
                .find(|d| d.name == *name)
                .expect("Definition named `{name}` has to be available");
            let TypeDef {
                def: TypeDefInner::Struct { fields },
                ..
            } = def.clone()
            else {
                panic!("Definition does not match: {def:?}");
            };
            fields
                .into_iter()
                .map(|(fname, ct)| (fname, arb_json_for_ctype(&ct, defs)))
                .fold(Just(vec![]).boxed(), |acc, (fname, s)| {
                    (acc, s)
                        .prop_map(move |(mut v, val)| {
                            v.push(format!(r#""{fname}":{val}"#));
                            v
                        })
                        .boxed()
                })
                .prop_map(|entries| format!("{{{}}}", entries.join(",")))
                .boxed()
        }
        TypeExpr::TaggedEnum(name) => {
            let def = defs
                .iter()
                .find(|d| d.name == *name)
                .expect("Definition named `{name}` has to be available");
            let TypeDef {
                def: TypeDefInner::TaggedEnum { def: variants },
                ..
            } = def.clone()
            else {
                panic!("Definition does not match: {def:?}");
            };

            let variant_strats = variants.variants.into_iter().map(|v| {
                v.fields
                    .into_iter()
                    .map(|(fname, ct)| (fname.clone(), arb_json_for_ctype(&ct, defs)))
                    .fold(
                        Just(vec![format!(r#""type":"{}""#, v.tag_value)]).boxed(),
                        |acc, (fname, s)| {
                            (acc, s)
                                .prop_map(move |(mut v, val)| {
                                    v.push(format!(r#""{}":{}"#, fname, val));
                                    v
                                })
                                .boxed()
                        },
                    )
                    .prop_map(|entries| format!("{{{}}}", entries.join(",")))
                    .boxed()
            });
            proptest::strategy::Union::new(variant_strats).boxed()
        }
        TypeExpr::StringEnum(name) => {
            let def = defs
                .iter()
                .find(|d| d.name == *name)
                .expect("Definition named `{name}` has to be available");
            let TypeDef {
                def: TypeDefInner::StringEnum { variants },
                ..
            } = def.clone()
            else {
                panic!("Definition does not match: {def:?}");
            };
            sample::select(variants)
                .prop_map(|v| format!(r#""{v}""#))
                .boxed()
        }
        TypeExpr::Void => panic!(),
    }
}

struct CompiledDeps {
    includes: Vec<PathBuf>,
    object_files: Vec<PathBuf>,
}
static COMPILED_DEPS: OnceLock<CompiledDeps> = OnceLock::new();

fn compiled_deps() -> &'static CompiledDeps {
    COMPILED_DEPS.get_or_init(|| {
        let out = std::env::temp_dir().join("yerpc_cjson_objs");
        let _ = std::fs::remove_dir_all(&out);
        fs::create_dir_all(out.join("include/cjson")).unwrap();
        fs::write(
            &out.join("include/cjson/cJSON.h"),
            include_str!("include/cjson/cJSON.h"),
        )
        .unwrap();
        fs::write(&out.join("cJSON.c"), include_str!("include/cjson/cJSON.c")).unwrap();
        CompiledDeps {
            includes: vec![out.join("include")],
            object_files: cc_build()
                .include(&out.join("include/cjson"))
                .out_dir(&out)
                .file(out.join("cJSON.c"))
                .compile_intermediates(),
        }
    })
}

pub(crate) fn cc_build() -> cc::Build {
    let mut b = cc::Build::new();
    b.target(current_platform::CURRENT_PLATFORM)
        .host(current_platform::CURRENT_PLATFORM)
        .opt_level(0)
        .warnings(true)
        .warnings_into_errors(true)
        .emit_rerun_if_env_changed(false)
        .cargo_metadata(false)
        .cargo_warnings(false)
        .cargo_output(false);
    b
}

fn has_tcc() -> bool {
    static HAS_TCC: OnceLock<bool> = OnceLock::new();
    *HAS_TCC.get_or_init(|| Command::new("tcc").arg("-v").output().is_ok())
}

pub(crate) fn link_exe(
    dir: &Path,
    srcs: &[&str],
    objs: &[PathBuf],
    includes: &[PathBuf],
) -> Result<PathBuf> {
    let (mut cmd, msvc_like) = if has_tcc() {
        (Command::new("tcc"), false)
    } else {
        let compiler = cc_build().get_compiler();
        (compiler.to_command(), compiler.is_like_msvc())
    };
    let exe = dir.join(format!("test{}", EXE_SUFFIX));
    cmd.current_dir(dir);
    let [mut out_flag, inc_flag] = if msvc_like {
        ["/Fe:", "/I"].map(OsString::from)
    } else {
        ["-o", "-I"].map(OsString::from)
    };
    out_flag.push(exe.as_os_str());
    cmd.arg(out_flag);
    for i in includes {
        let mut arg = inc_flag.clone();
        arg.push(i.as_os_str());
        cmd.arg(arg);
    }
    cmd.args(srcs);
    cmd.args(objs);
    let output = cmd.output().context("Failed to launch linking command")?;
    if output.status.success() {
        Ok(exe)
    } else {
        bail!(
            "Failed creating the executalbe:\n{cmd:?}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

pub(crate) fn run_exe(exe: &Path) -> Result<()> {
    let mut cmd = std::process::Command::new(exe);
    let output = cmd.output().context("Failed to launch the executable")?;
    if output.status.success() {
        Ok(())
    } else {
        bail!(
            "Failure when running the executable: {}\nCommand: {cmd:?}\n{}\n{}",
            output.status.to_string(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
