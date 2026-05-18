* Fix enum variant with defined scalar

```
#[derive(TypeDef)]
#[serde(tag = "kind")]
pub enum CustomResult {
    Ok(Shape),
    Error(u32),
}
```

* Clean up TypeExpr construction from ts types
* Clean up Tests
* Add docs to rpc methods
* Fix namespaces (also for json) (remove `ns` param passing?)
* Add dc::String type (should remove special handling)
* Allow iteration of Containers
* Fix struct member nullptr dereferences (and similar issues)
* Simplify forward declaration hassle
* Fix -pedantic warnings
* Fix Array/Map ctor with zero entries (currently the default ctor is used)
* Consider removing Map::find (its broken for Options)
