#![cfg(test)]

mod utils;

use super::*;
use anyhow::Result;
use proptest::prelude::*;
use std::sync::OnceLock;
use utils::compile_and_run;

#[test]
fn hardcoded_tests() -> Result<()> {
    let ns = "tst_";
    let coord = TypeDef {
        name: "Coord".into(),
        def: TypeDefInner::Struct {
            fields: vec![("x".into(), TypeExpr::F64), ("y".into(), TypeExpr::F64)],
        },
    };
    let shape = TypeDef {
        name: "Shape".into(),
        def: TypeDefInner::Struct {
            fields: vec![
                ("name".into(), TypeExpr::String),
                ("origin".into(), TypeExpr::Struct("Coord".into())),
                (
                    "points".into(),
                    TypeExpr::Array(Box::new(TypeExpr::Struct("Coord".into()))),
                ),
                (
                    "meta".into(),
                    TypeExpr::Optional(Box::new(TypeExpr::Struct("Coord".into()))),
                ),
            ],
        },
    };
    let defs = vec![coord, shape.clone()];
    let methods = vec![Method::new(
        "ident",
        vec![("param".to_owned(), shape.as_ctype())],
        Some(shape.as_ctype()),
        false,
        true,
    )];
    let main_c = format!(
        r#"
static int _test_failures = 0;
#define ASSERT(cond) if (!(cond)) {{ \
    fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond); _test_failures++; }}
#define ASSERT_STR_EQ(a, b) if (strcmp(a, b) != 0) {{ \
    fprintf(stderr, "FAIL %s:%d: \"%s\" != \"%s\"\n", __FILE__, __LINE__, a, b); _test_failures++; }}
#define ASSERT_INT_EQ(a, b) if ((a) != (b)) {{ \
    fprintf(stderr, "FAIL %s:%d: %d != %d\n", __FILE__, __LINE__, (a), (b)); _test_failures++; }}
#define ASSERT_SZ_EQ(a, b) if ((a) != (b)) {{ \
    fprintf(stderr, "FAIL %s:%d: %zu != %zu\n", __FILE__, __LINE__, (size_t)(a), (size_t)(b)); _test_failures++; }}
#define FABS(x) ((x) < 0 ? -(x) : (x))
#define ASSERT_DBL_CLOSE(a, b) if (FABS((a) - (b)) >= 1e-9) {{ \
    fprintf(stderr, "FAIL %s:%d: %f != %f (eps=1e-9)\n", __FILE__, __LINE__, (a), (b)); _test_failures++; }}


void test_build_fn() {{
    tst_coord_t* p0 = tst_coord_new(1.5, 2.5);
    tst_coord_t* p1 = tst_coord_new(0, 0);
    tst_coord_t* p2 = tst_coord_new(3, 4);
    tst_array_coord_t* arr = tst_array_coord_new(2);
    arr->items[0] = p1;
    arr->items[1] = p2;
    tst_shape_t* shape = tst_shape_new(tst_string_new("triangle"), p0, arr, NULL);
    tst_request_t* req = tst_build_ident(1337, shape);
    ASSERT_STR_EQ(req->json, "{{\"jsonrpc\":\"2.0\",\"id\":1337,\"method\":\"ident\",\"params\":[{{\"name\":\"triangle\",\"origin\":{{\"x\":1.5,\"y\":2.5}},\"points\":[{{\"x\":0,\"y\":0}},{{\"x\":3,\"y\":4}}],\"meta\":null}}]}}");
    tst_request_unref(req);
    tst_shape_unref(shape);
}}

void test_parse_fn() {{
    tst_result_t* r = tst_parse_response("{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{{\"name\":\"triangle\",\"origin\":{{\"x\":1.5,\"y\":2.5}},\"points\":[{{\"x\":0,\"y\":0}},{{\"x\":3,\"y\":4}}],\"meta\":null}}}}");
    ASSERT_INT_EQ(r->error_code, 0);
    tst_shape_t* shape = tst_parse_ident(r);
    ASSERT(shape != NULL);
    ASSERT_STR_EQ(shape->name, "triangle");
    ASSERT(shape->points);
    ASSERT_SZ_EQ(shape->points->len, 2);
    ASSERT(shape->points->items);
    tst_coord_t* p0 = shape->points->items[0];
    ASSERT(p0);
    ASSERT_DBL_CLOSE(p0->x, 0.0);
    ASSERT_DBL_CLOSE(p0->y, 0.0);
    tst_coord_t* p1 = shape->points->items[1];
    ASSERT(p1);
    ASSERT_DBL_CLOSE(p1->x, 3.0);
    ASSERT_DBL_CLOSE(p1->y, 4.0);
    tst_shape_unref(shape);
    tst_result_unref(r);
}}

void test_errors() {{
{{
    {ns}result_t* r = {ns}parse_response("not json");
    ASSERT(r->error_code != 0);
    {ns}result_unref(r);
}}
{{
    {ns}result_t* r = {ns}parse_response("{{\"jsonrpc\":\"2.0\",\"id\":1,\"error\":{{\"code\":-32601,\"message\":\"not found\"}}}}");
    ASSERT(r->error_code == -32601);
    ASSERT_STR_EQ(r->error_message, "not found");
    {ns}result_unref(r);
}}
{{
    {ns}result_t* r = {ns}parse_response("{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":42}}");
    {ns}shape_t* val = {ns}parse_ident(r);
    ASSERT(val == NULL);
    ASSERT(r->error_code != 0);
    {ns}result_unref(r);
}}
}}

int main() {{
    test_build_fn();
    test_parse_fn();
    test_errors();
    return _test_failures;
}}
"#
    );
    compile_and_run(&main_c, &methods, &defs, ns)
}

fn json_roundtrip_main(t: &TypeExpr, json: String, ns: &str) -> String {
    let json = json.replace('\\', r#"\\"#).replace('"', r#"\""#);
    let ct = t.c_type(&ns);
    let id = t.id(&ns);
    let unref = if t.needs_heap_free() {
        &format! {"{id}_unref(s);"}
    } else {
        ""
    };
    format!(
        r#"
#define ASSERT(cond) if (!(cond)) {{ fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond); failures++; }}
int main() {{
    int failures = 0;
    dc_json_doc_t doc = dc_json_doc_parse("{json}");
    dc_json_t root = dc_json_root(doc);
    {ct} s;
    ASSERT(!{id}_from_json(root, &s));
    dc_json_t node = {id}_to_json(s);
    char* json_str = dc_json_print(node);
    ASSERT(strcmp(json_str, "{json}") == 0);
    free(json_str);
    {unref}
    dc_json_doc_free(doc);

    // dc_json_t feels like a stack value, but that's a lie for cjson, we have to free it
    dc_json_doc_t doc2 = dc_json_doc_new();
    dc_json_t root2 = dc_json_root(doc2);
    dc_json_add_to_object(root2, "dummy", node);
    dc_json_doc_free(doc2);
    return failures;
}}
        "#
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn roundtrip((t, defs, json) in utils::arb_json_roundtrip()) {
        let main = json_roundtrip_main(&t,  json , "tst_");
        let m = Method::new("foo", vec![], Some(t), false, false);
        compile_and_run(&main, &[m], &defs, "tst_").map_err(|e| TestCaseError::fail(e.to_string()))?;
    }
}
