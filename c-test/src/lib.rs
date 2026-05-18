#![cfg(test)]

use core::ptr::null_mut;
use std::ffi::{c_char, c_int, c_void, CStr, CString};

#[repr(C)]
struct tst_request_t {
    json: *const c_char,
}
#[repr(C)]
struct tst_coord_t {
    x: f64,
    y: f64,
}
#[repr(C)]
struct cJSON {
    _private: [u8; 0],
}
#[repr(C)]
#[derive(PartialEq, Debug)]
enum tst_color_kind {
    Red,
    Green,
    Custom,
    Named,
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
struct tst_color_custom {
    r: u8,
    g: u8,
    b: u8,
    alpha: tst_optional_u8_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct tst_color_named {
    name: *const c_char,
    hex: *const c_char,
}

#[repr(C)]
union tst_color_data {
    custom: tst_color_custom,
    named: tst_color_named,
}

#[repr(C)]
struct tst_color_t {
    kind: tst_color_kind,
    data: tst_color_data,
}

#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum tst_direction_t {
    Unknown,
    Up,
    Down,
    Left,
    Right,
}
#[repr(C)]
struct tst_tuple_u32_coord_t {
    field_0: u32,
    field_1: *mut tst_coord_t,
}
#[repr(C)]
struct tst_map_string_t {
    keys: *mut *const c_char,
    values: *mut *const c_char,
    len: usize,
}
#[repr(C)]
struct tst_map_coord_t {
    keys: *mut *mut c_char,
    values: *mut *mut tst_coord_t,
    len: usize,
}
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
struct tst_optional_u8_t {
    has_value: bool,
    value: u8,
}
#[repr(C)]
struct tst_optional_u32_t {
    has_value: bool,
    value: u32,
}
#[repr(C)]
struct tst_array_u32_t {
    items: *mut u32,
    len: usize,
}
#[repr(C)]
struct tst_array_coord_t {
    items: *mut *mut tst_coord_t,
    len: usize,
}
#[repr(C)]
struct tst_result_t {
    id: u32,
    error_code: i32,
    result: *mut cJSON,
    error_message: *const c_char,
    root_: *mut cJSON,
}

#[link(name = "cpp-ffi", kind = "static")]
extern "C" {
    fn tst_build_create_shape(
        _rpc_id: u32,
        name: *const c_char,
        origin: *const tst_coord_t,
        visible: bool,
    ) -> *mut tst_request_t;
    fn tst_build_notify(msg: *const c_char) -> *mut tst_request_t;
    fn tst_request_unref(r: *mut tst_request_t);

    fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char;
    fn cJSON_Delete(item: *mut cJSON);
    fn cJSON_free(ptr: *mut c_void);

    fn tst_color_to_json(o: *const tst_color_t) -> *mut cJSON;
    fn tst_color_new() -> *mut tst_color_t;
    fn tst_color_unref(o: *mut tst_color_t);
    fn tst_direction_to_json(v: tst_direction_t) -> *mut cJSON;

    fn tst_parse_response(json: *const c_char) -> *mut tst_result_t;
    fn tst_result_unref(r: *mut tst_result_t);
    fn tst_parse_add(r: *mut tst_result_t) -> f32;
    fn tst_parse_echo(r: *mut tst_result_t) -> *mut c_char;
    fn tst_parse_get_color(r: *mut tst_result_t) -> *mut tst_color_t;
    fn tst_parse_send_msg(r: *mut tst_result_t) -> *mut tst_tuple_u32_coord_t;
    fn tst_coord_new(x: f64, y: f64) -> *mut tst_coord_t;
    fn tst_optional_coord_new(x: *mut tst_coord_t) -> *mut tst_coord_t;
    fn tst_optional_coord_unref(x: *mut tst_coord_t);
    fn tst_tuple_u32_coord_new(
        field_0: u32,
        field_1: *mut tst_coord_t,
    ) -> *mut tst_tuple_u32_coord_t;
    fn tst_tuple_u32_coord_unref(o: *mut tst_tuple_u32_coord_t);
    fn tst_parse_get_info(r: *mut tst_result_t) -> *mut tst_map_string_t;
    fn tst_map_string_new(len: c_int) -> *mut tst_map_string_t;
    fn tst_map_string_unref(m: *mut tst_map_string_t);
    fn tst_parse_get_coords_by_id(r: *mut tst_result_t) -> *mut tst_map_coord_t;
    fn tst_map_coord_unref(m: *mut tst_map_coord_t);
    fn tst_build_get_size(_rpc_id: u32, path: *const c_char) -> *mut tst_request_t;
    fn tst_parse_get_size(r: *mut tst_result_t) -> u64;
    fn tst_parse_get_selected_id(r: *mut tst_result_t) -> tst_optional_u32_t;
    fn tst_string_new(s: *const c_char) -> *const c_char;
    fn tst_string_unref(s: *mut c_char);
    fn tst_parse_get_ids(r: *mut tst_result_t) -> *mut tst_array_u32_t;
    fn tst_array_u32_new(len: c_int) -> *mut tst_array_u32_t;
    fn tst_array_u32_unref(a: *mut tst_array_u32_t);
    fn tst_parse_get_points(r: *mut tst_result_t) -> *mut tst_array_coord_t;
    fn tst_array_coord_unref(a: *mut tst_array_coord_t);
    fn cpp_tests() -> c_int;
}

#[test]
fn run_cpp_tests() {
    let n_failures = unsafe { cpp_tests() };
    assert_eq!(n_failures, 0);
}

fn get_json(req: *mut tst_request_t) -> serde_json::Value {
    let json = unsafe { CStr::from_ptr((*req).json) }.to_bytes();
    let v: serde_json::Value = serde_json::from_slice(json).unwrap();
    unsafe { tst_request_unref(req) };
    v
}

unsafe fn parse_response(json: &str) -> *mut tst_result_t {
    let cs = CString::new(json).unwrap();
    tst_parse_response(cs.as_ptr())
}

unsafe fn to_json_value<T>(
    obj: *const T,
    to_json: unsafe extern "C" fn(*const T) -> *mut cJSON,
) -> serde_json::Value {
    let cj = to_json(obj);
    let printed = cJSON_PrintUnformatted(cj);
    cJSON_Delete(cj);
    let s = CStr::from_ptr(printed).to_str().unwrap().to_owned();
    cJSON_free(printed as *mut c_void);
    serde_json::from_str(&s).unwrap()
}

#[test]
fn test_build_request() {
    let name = CString::new("circle").unwrap();
    let origin = tst_coord_t { x: 1.0, y: 2.0 };
    let j = get_json(unsafe { tst_build_create_shape(4, name.as_ptr(), &origin, true) });
    assert_eq!(j["jsonrpc"], "2.0");
    assert_eq!(j["id"], 4);
    assert_eq!(j["method"], "create_shape");
    assert_eq!(j["params"][0], "circle");
    assert_eq!(j["params"][1]["x"], 1.0);
    assert_eq!(j["params"][2], true);
}

#[test]
fn test_build_notification() {
    let msg = CString::new("ping").unwrap();
    let j = get_json(unsafe { tst_build_notify(msg.as_ptr()) });
    assert_eq!(j["method"], "notify");
    assert_eq!(j["params"][0], "ping");
    assert!(j.get("id").is_none());
}

#[test]
fn test_parse_primitive() {
    unsafe {
        let r = parse_response(r#"{"jsonrpc":"2.0","id":42,"result":3.5}"#);
        assert_eq!((*r).error_code, 0);
        assert_eq!((*r).id, 42);
        let v = tst_parse_add(r);
        assert!((v - 3.5f32).abs() < 1e-6);
        tst_result_unref(r);
    }
}

#[test]
fn test_parse_string() {
    unsafe {
        let r = parse_response(r#"{"jsonrpc":"2.0","id":2,"result":"hello"}"#);
        assert_eq!((*r).error_code, 0);
        let s = tst_parse_echo(r);
        assert!(!s.is_null());
        assert_eq!(CStr::from_ptr(s).to_str().unwrap(), "hello");
        tst_string_unref(s);
        tst_result_unref(r);
    }
}

#[test]
fn test_color_new() {
    unsafe {
        let color = tst_color_new();
        (*color).kind = tst_color_kind::Named;
        (*color).data.named.name = string_new("foo");
        (*color).data.named.hex = string_new("0xFFFFFF");
        tst_color_unref(color);
    }
}

#[test]
fn test_roundtrip_color_unit() {
    unsafe {
        let r = parse_response(r#"{"jsonrpc":"2.0","id":3,"result":{"kind":"Red"}}"#);
        let color = tst_parse_get_color(r);
        assert_eq!((*color).kind, tst_color_kind::Red);
        assert!(!color.is_null());
        let j = to_json_value(color, tst_color_to_json);
        tst_color_unref(color);
        tst_result_unref(r);
        assert_eq!(j["kind"], "Red");
    }
}

#[test]
fn test_roundtrip_color_struct() {
    unsafe {
        let r = parse_response(
            r#"{"jsonrpc":"2.0","id":4,"result":{"kind":"Custom","r":10,"g":20,"b":30,"alpha":128}}"#,
        );
        let color = tst_parse_get_color(r);
        assert!(!color.is_null());
        let j = to_json_value(color, tst_color_to_json);
        tst_color_unref(color);
        tst_result_unref(r);
        assert_eq!(j["kind"], "Custom");
        assert_eq!(j["r"], 10);
        assert_eq!(j["g"], 20);
        assert_eq!(j["b"], 30);
        assert_eq!(j["alpha"], 128);
    }
}

#[test]
fn test_roundtrip_color_optional_null() {
    unsafe {
        let r = parse_response(
            r#"{"jsonrpc":"2.0","id":4,"result":{"kind":"Custom","r":10,"g":20,"b":30}}"#,
        );
        let color = tst_parse_get_color(r);
        assert!(!color.is_null());
        let j = to_json_value(color, tst_color_to_json);
        tst_color_unref(color);
        tst_result_unref(r);
        assert_eq!(j["kind"], "Custom");
        assert!(j["alpha"].is_null());
    }
}

#[test]
fn test_roundtrip_color_newtype_variant() {
    unsafe {
        let r = parse_response(
            r##"{"jsonrpc":"2.0","id":5,"result":{"kind":"Named","name":"coral","hex":"#FF7F50"}}"##,
        );
        let color = tst_parse_get_color(r);
        assert!(!color.is_null());
        let j = to_json_value(color, tst_color_to_json);
        tst_color_unref(color);
        tst_result_unref(r);
        assert_eq!(j["kind"], "Named");
        assert_eq!(j["name"], "coral");
        assert_eq!(j["hex"], "#FF7F50");
    }
}

#[test]
fn test_parse_error_response() {
    unsafe {
        let r = parse_response(
            r#"{"jsonrpc":"2.0","id":4,"error":{"code":-32601,"message":"Method not found"}}"#,
        );
        assert_eq!((*r).error_code, -32601);
        assert_eq!(
            CStr::from_ptr((*r).error_message).to_str().unwrap(),
            "Method not found"
        );
        tst_result_unref(r);
    }
}

#[test]
fn test_parse_type_mismatch_in_envelope() {
    unsafe {
        let r = parse_response(r#"{"jsonrpc":"2.0","id":"not_a_number","result":12.3}"#);
        assert_eq!((*r).error_code, -32700);
        tst_result_unref(r);
    }
}

#[test]
fn test_parse_type_mismatch() {
    unsafe {
        let r = parse_response(r#"{"jsonrpc":"2.0","id":5,"result":"not_a_number"}"#);
        assert_eq!((*r).error_code, 0);
        let v = tst_parse_add(r);
        assert_eq!((*r).error_code, -32700);
        assert_eq!(v, 0.0);
        tst_result_unref(r);
    }
}

#[test]
fn test_array_new() {
    unsafe {
        let x = tst_array_u32_new(3);
        assert_eq!((*x).len, 3);
        *(*x).items.add(0) = 10;
        *(*x).items.add(1) = 20;
        *(*x).items.add(2) = 30;
        tst_array_u32_unref(x);
    }
}

#[test]
fn test_parse_array_primitive() {
    unsafe {
        let r = parse_response(r#"{"jsonrpc":"2.0","id":6,"result":[10,20,30]}"#);
        assert_eq!((*r).error_code, 0);
        let a = tst_parse_get_ids(r);
        assert!(!a.is_null());
        assert_eq!((*a).len, 3);
        assert_eq!(*(*a).items.add(0), 10);
        assert_eq!(*(*a).items.add(1), 20);
        assert_eq!(*(*a).items.add(2), 30);
        tst_array_u32_unref(a);
        tst_result_unref(r);
    }
}

#[test]
fn test_parse_array_struct() {
    unsafe {
        let r = parse_response(
            r#"{"jsonrpc":"2.0","id":7,"result":[{"x":1.0,"y":2.0},{"x":3.0,"y":4.0}]}"#,
        );
        assert_eq!((*r).error_code, 0);
        let a = tst_parse_get_points(r);
        assert!(!a.is_null());
        assert_eq!((*a).len, 2);
        let c0 = *(*a).items.add(0);
        let c1 = *(*a).items.add(1);
        assert!(((*c0).x - 1.0).abs() < 1e-10);
        assert!(((*c1).y - 4.0).abs() < 1e-10);
        tst_array_coord_unref(a);
        tst_result_unref(r);
    }
}

#[test]
fn test_optional_coord_new() {
    unsafe {
        let x = tst_optional_coord_new(tst_coord_new(1.0, 2.0));
        assert!(!x.is_null());
        assert_eq!((*x).x, 1.0);
        assert_eq!((*x).y, 2.0);
        tst_optional_coord_unref(x);
    }
}

#[test]
fn test_optional_coord_new_nullopt() {
    unsafe {
        let x = tst_optional_coord_new(null_mut());
        assert!(x.is_null());
        tst_optional_coord_unref(x);
    }
}

#[test]
fn test_parse_optional_present() {
    unsafe {
        let r = parse_response(r#"{"jsonrpc":"2.0","id":8,"result":42}"#);
        assert_eq!((*r).error_code, 0);
        let o = tst_parse_get_selected_id(r);
        assert!(o.has_value);
        assert_eq!(o.value, 42);
        tst_result_unref(r);
    }
}

#[test]
fn test_parse_optional_null() {
    unsafe {
        let r = parse_response(r#"{"jsonrpc":"2.0","id":9,"result":null}"#);
        assert_eq!((*r).error_code, 0);
        let o = tst_parse_get_selected_id(r);
        assert!(!o.has_value);
        tst_result_unref(r);
    }
}
#[test]
fn test_tuple_new() {
    unsafe {
        let x = tst_tuple_u32_coord_new(42, tst_coord_new(1.0, 2.0));
        assert_eq!((*x).field_0, 42);
        assert_eq!((*(*x).field_1).x, 1.0);
        assert_eq!((*(*x).field_1).y, 2.0);
        tst_tuple_u32_coord_unref(x);
    }
}
#[test]
fn test_parse_tuple() {
    unsafe {
        let r = parse_response(r#"{"jsonrpc":"2.0","id":10,"result":[42,{"x":1.5,"y":2.5}]}"#);
        assert_eq!((*r).error_code, 0);
        let t = tst_parse_send_msg(r);
        assert!(!t.is_null());
        assert_eq!((*t).field_0, 42);
        assert!(!(*t).field_1.is_null());
        assert!(((*(*t).field_1).x - 1.5).abs() < 1e-10);
        assert!(((*(*t).field_1).y - 2.5).abs() < 1e-10);
        tst_tuple_u32_coord_unref(t);
        tst_result_unref(r);
    }
}
#[test]
fn test_map_string_new() {
    unsafe {
        let x = tst_map_string_new(2);
        assert_eq!((*x).len, 2);
        *(*x).keys.add(0) = string_new("k0");
        *(*x).keys.add(1) = string_new("k1");
        *(*x).values.add(0) = string_new("v0");
        *(*x).values.add(1) = string_new("v1");
        tst_map_string_unref(x);
    }
}

fn string_new(s: &str) -> *const c_char {
    unsafe { tst_string_new(CString::new(s).unwrap().as_ptr()) }
}

#[test]
fn test_parse_map_string() {
    unsafe {
        let r = parse_response(r#"{"jsonrpc":"2.0","id":11,"result":{"arch":"x86","os":"linux"}}"#);
        assert_eq!((*r).error_code, 0);
        let m = tst_parse_get_info(r);
        assert!(!m.is_null());
        assert_eq!((*m).len, 2);
        let k0 = CStr::from_ptr(*(*m).keys.add(0)).to_str().unwrap();
        let v0 = CStr::from_ptr(*(*m).values.add(0)).to_str().unwrap();
        let k1 = CStr::from_ptr(*(*m).keys.add(1)).to_str().unwrap();
        let v1 = CStr::from_ptr(*(*m).values.add(1)).to_str().unwrap();
        assert!((k0 == "arch" && v0 == "x86") || (k0 == "os" && v0 == "linux"));
        assert!((k1 == "arch" && v1 == "x86") || (k1 == "os" && v1 == "linux"));
        tst_map_string_unref(m);
        tst_result_unref(r);
    }
}

#[test]
fn test_parse_map_struct() {
    unsafe {
        let r = parse_response(
            r#"{"jsonrpc":"2.0","id":12,"result":{"1":{"x":1.0,"y":2.0},"2":{"x":3.0,"y":4.0}}}"#,
        );
        assert_eq!((*r).error_code, 0);
        let m = tst_parse_get_coords_by_id(r);
        assert!(!m.is_null());
        assert_eq!((*m).len, 2);
        let k0 = CStr::from_ptr(*(*m).keys.add(0)).to_str().unwrap();
        let c0 = *(*m).values.add(0);
        assert!(!c0.is_null());
        assert!(k0 == "1" || k0 == "2");
        tst_map_coord_unref(m);
        tst_result_unref(r);
    }
}
#[test]
fn test_pathbuf_and_usize() {
    let path = CString::new("/tmp/test").unwrap();
    let j = get_json(unsafe { tst_build_get_size(1, path.as_ptr()) });
    assert_eq!(j["params"][0], "/tmp/test");
    unsafe {
        let r = parse_response(r#"{"jsonrpc":"2.0","id":1,"result":1024}"#);
        assert_eq!(tst_parse_get_size(r), 1024u64);
        tst_result_unref(r);
    }
}
