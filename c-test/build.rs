use std::{collections::HashMap, fs};

use typescript_type_def::TypeDef;
use yerpc::{c, c::TypeExpr};

#[derive(TypeDef)]
pub struct Coord {
    pub x: f64,
    pub y: f64,
}

#[derive(TypeDef)]
pub struct Shape {
    pub name: String,
    pub points: Vec<Coord>,
}

#[derive(TypeDef)]
#[serde(tag = "kind")]
pub enum CustomResult {
    Ok(Shape),
    Error,
}

#[derive(TypeDef)]
pub struct Dummy {
    pub my_opt_u32: Option<u32>,
    pub my_opt_coord: Option<Coord>,
    pub my_arr_u32: Vec<u32>,
    pub my_arr_coord: Vec<Coord>,
    pub my_tuple_u32_coord: (u32, Coord),
    pub my_map_u32: HashMap<String, u32>,
    pub my_map_coord: HashMap<String, Coord>,
    pub my_map_str: HashMap<String, String>,
    pub other: (Coord, Shape, CustomResult, Color, Direction),
}

#[derive(TypeDef)]
#[serde(tag = "kind")]
pub enum Color {
    Red,
    Green,
    Custom {
        r: u8,
        g: u8,
        b: u8,
        alpha: Option<u8>,
    },
    Named(ColorName),
}

#[derive(TypeDef)]
pub struct ColorName {
    pub name: String,
    pub hex: String,
}

#[derive(TypeDef)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

fn main() {
    let defs = c::collect_type_defs::<Dummy>();
    let methods = vec![
        c::Method::new(
            "dummy",
            vec![],
            Some(TypeExpr::from(&<Dummy>::INFO)),
            false,
            true,
        ),
        c::Method::new(
            "add",
            vec![
                ("a".into(), TypeExpr::from(&<f32>::INFO)),
                ("b".into(), TypeExpr::from(&<f32>::INFO)),
            ],
            Some(TypeExpr::from(&<f32>::INFO)),
            false,
            true,
        ),
        c::Method::new(
            "echo",
            vec![("msg".into(), TypeExpr::from(&<String>::INFO))],
            Some(TypeExpr::from(&<String>::INFO)),
            false,
            true,
        ),
        c::Method::new(
            "roundtrip_coord",
            vec![("x".into(), TypeExpr::from(&<Coord>::INFO))],
            Some(TypeExpr::from(&<Coord>::INFO)),
            false,
            true,
        ),
        c::Method::new(
            "get_origin",
            vec![("id".into(), TypeExpr::from(&<u32>::INFO))],
            Some(TypeExpr::from(&<Coord>::INFO)),
            false,
            true,
        ),
        c::Method::new(
            "get_color",
            vec![("id".into(), TypeExpr::from(&<u32>::INFO))],
            Some(TypeExpr::from(&<Color>::INFO)),
            false,
            true,
        ),
        c::Method::new(
            "create_shape",
            vec![
                ("name".into(), TypeExpr::from(&<String>::INFO)),
                ("origin".into(), TypeExpr::from(&<Coord>::INFO)),
                ("visible".into(), TypeExpr::from(&<bool>::INFO)),
            ],
            Some(TypeExpr::from(&<u32>::INFO)),
            false,
            true,
        ),
        c::Method::new(
            "send_msg",
            vec![
                ("chat_id".into(), TypeExpr::from(&<u32>::INFO)),
                ("text".into(), TypeExpr::from(&<String>::INFO)),
            ],
            Some(TypeExpr::from(&<(u32, Coord)>::INFO)),
            false,
            true,
        ),
        c::Method::new(
            "get_selected_id",
            vec![],
            Some(TypeExpr::from(&<Option<u32>>::INFO)),
            false,
            true,
        ),
        c::Method::new(
            "get_ids",
            vec![],
            Some(TypeExpr::from(&<Vec<u32>>::INFO)),
            false,
            true,
        ),
        c::Method::new(
            "get_points",
            vec![],
            Some(TypeExpr::from(&<Vec<Coord>>::INFO)),
            false,
            true,
        ),
        c::Method::new(
            "get_info",
            vec![],
            Some(TypeExpr::from(
                &<std::collections::HashMap<String, String>>::INFO,
            )),
            false,
            true,
        ),
        c::Method::new(
            "get_coords_by_id",
            vec![],
            Some(TypeExpr::from(
                &<std::collections::HashMap<String, Coord>>::INFO,
            )),
            false,
            true,
        ),
        c::Method::new(
            "get_size",
            vec![("path".into(), TypeExpr::from(&<std::path::PathBuf>::INFO))],
            Some(TypeExpr::from(&<usize>::INFO)),
            false,
            true,
        ),
        c::Method::new(
            "get_direction",
            vec![("id".into(), TypeExpr::from(&<u32>::INFO))],
            Some(TypeExpr::from(&<Direction>::INFO)),
            false,
            true,
        ),
        c::Method::new(
            "notify",
            vec![("msg".into(), TypeExpr::from(&<String>::INFO))],
            None,
            true,
            true,
        ),
        c::Method::new(
            "sleep",
            vec![("delay".into(), TypeExpr::from(&<f64>::INFO))],
            None,
            false,
            true,
        ),
    ];
    let outdir = std::env::var("OUT_DIR").unwrap();
    // println!("cargo::warning=outdir: {}", outdir);
    let out = std::path::PathBuf::from(outdir);
    c::write_files(&out, &methods, defs.as_slice(), "tst_");

    let rpc_h = fs::read_to_string(out.join("rpc.h"))
        .unwrap()
        .replace("static inline ", "");
    fs::write(out.join("rpc.h"), rpc_h).unwrap();

    std::fs::write(
        out.join("cpp-ffi.cpp"),
        r#"
#include "dc_json_cjson.h"
#include "rpc.hpp"

#include <iostream>
#include <mutex>
#include <queue>
#include <condition_variable>


static int _test_failures = 0;
#define _TEST_FAIL(fmt, ...) do { \
    fprintf(stderr, "FAIL %s:%d: " fmt "\n", __FILE__, __LINE__, ##__VA_ARGS__); \
    _test_failures++; \
} while(0)
#define ASSERT(cond) do { if (!(cond)) _TEST_FAIL("%s", #cond); } while(0)
#define ASSERT_STR_EQ(a, b) do { \
    const std::string _assert_a = (a), _assert_b = (b); \
    if (_assert_a != _assert_b) _TEST_FAIL("\"%s\" != \"%s\"", _assert_a.c_str(), _assert_b.c_str()); \
} while(0)
#define ASSERT_INT_EQ(a, b) do { \
    const int _assert_a = (a), _assert_b = (b); \
    if (_assert_a != _assert_b) _TEST_FAIL("\"%d\" != \"%d\"", _assert_a, _assert_b); \
} while(0)
#define FABS(x) ((x) < 0 ? -(x) : (x))
#define ASSERT_DBL_CLOSE(a, b) do { \
    double _assert_a = (a), _assert_b = (b); \
    if (FABS(_assert_a - _assert_b) >= 1e-5) { \
        _TEST_FAIL("%f != %f (eps=1e-5)", _assert_a, _assert_b); \
    } \
} while(0)

class ThreadSafeQueue {
    std::queue<std::string> q_;
    mutable std::mutex mtx_;
    std::condition_variable cv_;
    bool done_ = false;

public:
    void push(std::string val) {
        { std::lock_guard lk(mtx_); q_.push(std::move(val)); }
        cv_.notify_one();
    }

    const std::string pop() {
        std::unique_lock lk(mtx_);
        cv_.wait(lk, [&]{ return !q_.empty() || done_; });
        if (q_.empty()) return "";
        std::string val = std::move(q_.front());
        q_.pop();
        return val;
    }

    void close() {
        { std::lock_guard lk(mtx_); done_ = true; }
        cv_.notify_all();
    }
};

struct TestTransport {
    std::shared_ptr<ThreadSafeQueue> in = std::make_shared<ThreadSafeQueue>();
    std::shared_ptr<ThreadSafeQueue> out = std::make_shared<ThreadSafeQueue>();
    const char* read() { _buf = in->pop(); return _buf.c_str(); }
    void send(const char* json) { out->push(json); }
    void close() { in->close(); out->close(); }
    TestTransport clone() { return *this; }
private:
    std::string _buf;
};

void logger(std::string msg) {
    std::cerr << msg << std::endl;
}

void get_double() {
    TestTransport t;
    dc::Rpc<TestTransport> rpc(t.clone(), logger);
    auto x = rpc.add_request(2.2, 4.4);
    ASSERT(t.out->pop().rfind(R"({"jsonrpc":"2.0","id":1,"method":"add","params":[2.2)", 0) == 0);
    t.in->push("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":6.6}");
    ASSERT_DBL_CLOSE(x.get().result, 6.6);
}

void get_string() {
    TestTransport t;
    dc::Rpc<TestTransport> rpc(t.clone(), logger);
    auto x = rpc.echo_request("foo");
    ASSERT_STR_EQ(t.out->pop().c_str(), "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"echo\",\"params\":[\"foo\"]}");
    t.in->push("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"bar\"}");
    ASSERT_STR_EQ(x.get().result, "bar");
}

void test_cpp_types() {
    { // Array of ints
        auto x = dc::ArrayU32(1, 2, 3);
        ASSERT_INT_EQ(x.size(), 3);
        ASSERT_INT_EQ(x[0], 1);
        ASSERT_INT_EQ(x[1], 2);
        ASSERT_INT_EQ(x[2], 3);
    }
    { // Array of structs
        auto x = dc::ArrayCoord(dc::Coord(1,2), dc::Coord(3,4));
        ASSERT_INT_EQ(x.size(), 2);
        ASSERT_INT_EQ(x[0].x(), 1);
        ASSERT_INT_EQ(x[0].y(), 2);
        ASSERT_INT_EQ(x[1].x(), 3);
        ASSERT_INT_EQ(x[1].y(), 4);
    }
    { // Nested Struct
        auto x = dc::Shape("triangle", dc::ArrayCoord(dc::Coord(1,2)));
        ASSERT_INT_EQ(x.points().size(), 1);
        ASSERT_INT_EQ(x.points()[0].x(), 1);
        ASSERT_INT_EQ(x.points()[0].y(), 2);
        ASSERT_STR_EQ(x.name(), "triangle");
    }
    { // Tuple
        auto x = dc::TupleU32Coord(42, dc::Coord(1,2));
        ASSERT_INT_EQ(x.field_0(), 42);
        ASSERT_INT_EQ(x.field_1().x(), 1);
        ASSERT_INT_EQ(x.field_1().y(), 2);
    }
    { // Map scalar
        using kv = dc::MapU32::kv;
        auto x = dc::MapU32(kv{"k0", 42});
        ASSERT_INT_EQ(x.find("k0"), 42);
        ASSERT(!x.find("oops"));
    }
    { // MapString
        using kv = dc::MapString::kv;
        auto x = dc::MapString(kv{"k0", "v0"});
        ASSERT_STR_EQ(x.find("k0"), "v0");
        ASSERT(!x.find("oops"));
    }
    { // MapCoord
        using kv = dc::MapCoord::kv;
        auto x = dc::MapCoord(kv{"k0", dc::Coord(1,2)});
        ASSERT_INT_EQ(x.find("k0").x(), 1);
        ASSERT_INT_EQ(x.find("k0").y(), 2);
        // ASSERT_INT_EQ(x.find("oops").x(), 0);
        ASSERT(!x.find("oops"));
    }
    { // Optional scalar
        auto x = dc::OptionalU8();
        ASSERT(!x);
        ASSERT_INT_EQ(x.inner(), 0);
        auto y = dc::OptionalU8(42);
        ASSERT(y);
        ASSERT_INT_EQ(y.inner(), 42);
    }
    { // Optional struct
        auto x = dc::OptionalCoord();
        ASSERT(!x);
        ASSERT(!x.inner());
        auto y = dc::OptionalCoord(dc::Coord(1,2));
        ASSERT(y);
        ASSERT(y.inner());
        ASSERT_INT_EQ(y.inner().x(), 1);
        ASSERT_INT_EQ(y.inner().y(), 2);
    }
    { // Tagged enum
        auto x = dc::Color();
        x.set_variant_custom(1, 2, 3, dc::OptionalU8());
        ASSERT(!x.as_variant_red());
        ASSERT(!x.as_variant_green());
        auto named = x.as_variant_named();
        ASSERT(!named);
        ASSERT_STR_EQ(named.hex(), "");
        ASSERT_STR_EQ(named.name(), "");
        auto custom = x.as_variant_custom();
        ASSERT(custom);
        ASSERT_INT_EQ(custom.r(), 1);
        ASSERT_INT_EQ(custom.g(), 2);
        ASSERT_INT_EQ(custom.b(), 3);
    }
    { // Taggged enum nested struct variant
        auto x = dc::CustomResult();
        x.set_variant_ok("foo", dc::ArrayCoord(dc::Coord(1,2)));
        ASSERT(!x.as_variant_error());
        auto v = x.as_variant_ok();
        ASSERT(v);
        ASSERT_STR_EQ(v.name(), "foo");
        ASSERT(v.points());
        ASSERT(v.points()[0]);
        ASSERT_INT_EQ(v.points()[0].x(), 1);
        ASSERT_INT_EQ(v.points()[0].y(), 2);
    }
}

void get_origin() {
    TestTransport t;
    dc::Rpc<TestTransport> rpc(t.clone(), logger);
    auto x = rpc.get_origin_request(42);
    ASSERT_STR_EQ(t.out->pop().c_str(), R"({"jsonrpc":"2.0","id":1,"method":"get_origin","params":[42]})");
    t.in->push(R"({"jsonrpc":"2.0","id":1,"result":{"x": 4.0, "y": 5.0}})");
    dc::Coord v = x.get().result;
    ASSERT_DBL_CLOSE(v.x(), 4.0);
    ASSERT_DBL_CLOSE(v.y(), 5.0);
}


struct EchoTransport {
    const char* read() { _buf = replace_method(q->pop()); return _buf.c_str(); }
    void send(const char* json) { q->push(json); }
    void close() { q->close(); }
private:
    std::string replace_method(const std::string& s) {
        if (s.empty()) return s;
        auto a = s.find(R"("method":")");
        auto b = s.find(R"(","params":[)");
        auto c = s.rfind(']');
        ASSERT(a != std::string::npos && b != std::string::npos && c != std::string::npos);
        return s.substr(0, a) + R"("result":)" + s.substr(b + 12, c - b - 12) + s.substr(c + 1);
    }
    std::shared_ptr<ThreadSafeQueue> q = std::make_shared<ThreadSafeQueue>();
    std::string _buf;
};

void test_echo() {
    dc::Rpc<EchoTransport> rpc{{}, logger};
    ASSERT_STR_EQ(rpc.echo("foo"), "foo");

    auto x = rpc.roundtrip_coord(dc::Coord(1, 3));
    ASSERT_DBL_CLOSE(x.x(), 1);
    ASSERT_DBL_CLOSE(x.y(), 3);
}

extern "C" int cpp_tests() {
    get_double();
    get_string();
    get_origin();
    test_cpp_types();
    test_echo();
    return _test_failures;
}
"#,
    )
    .unwrap();

    cc::Build::new()
        .cpp(true)
        .file(out.join("cpp-ffi.cpp"))
        .file("../yerpc/src/c/tests/include/cjson/cJSON.c")
        .include("../yerpc/src/c/tests/include")
        .compile("cpp-ffi");
}
