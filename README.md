# json-typedef-rust: A Rust implementation of JSON Typedef ![Crates.io](https://img.shields.io/crates/v/jtd) ![Docs.rs](https://docs.rs/jtd/badge.svg)

[JSON Type Definition](https://jsontypedef.com), aka
[RFC8927](https://tools.ietf.org/html/rfc8927), is an easy-to-learn,
standardized way to define a schema for JSON data. You can use JSON Typedef to
portably validate data across programming languages, create dummy data, generate
code, and more.

`jtd` is a Rust implementation of JSON Typedef. You can use this crate to parse
JSON Typedef schemas, validate JSON data against those schemas, or build your
own tooling on top of JSON Typedef.

Here's an example of this crate in action:

```rust
use jtd::{Schema, ValidationErrorIndicator};
use serde_json::json;

let schema = Schema::from_serde_schema(
    serde_json::from_value(json!({
        "properties": {
            "name": { "type": "string" },
            "age": { "type": "uint32" },
            "phones": {
                "elements": {
                    "type": "string"
                }
            }
        }
    })).unwrap()).unwrap();

// Since this first example is valid, we'll get back an empty list of
// validation errors.
let input_ok = json!({
    "name": "John Doe",
    "age": 43,
    "phones": ["+44 1234567", "+44 2345678"]
});

assert_eq!(
    Vec::<ValidationErrorIndicator>::new(),
    jtd::validate(&schema, &input_ok, Default::default()).unwrap(),
);

// This example is invalid, so we'll get back three validation errors:
//
// 1. "name" is required but not present,
// 2. "age" has the wrong type
// 3. "phones[1]" has the wrong type
let input_bad = json!({
    "age": "43",
    "phones": ["+44 1234567", 442345678]
});

// Each error indicator has two pieces of information: the path to the part
// of the input that was rejected (the "instance path"), and the part of the
// schema that rejected it (the "schema path").
//
// The exact values of the instance path and schema path is specified in the
// JSON Type Definition spec.
assert_eq!(
    vec![
        // "age" has the wrong type (required by "/properties/age/type")
        ValidationErrorIndicator {
            instance_path: vec!["age".into()],
            schema_path: vec!["properties".into(), "age".into(), "type".into()],
        },

        // "name" is missing (required by "/properties/name")
        ValidationErrorIndicator {
            instance_path: vec![],
            schema_path: vec!["properties".into(), "name".into()],
        },

        // "phones/1" has the wrong type (required by "/properties/phones/elements/type")
        ValidationErrorIndicator {
            instance_path: vec!["phones".into(), "1".into()],
            schema_path: vec![
                "properties".into(),
                "phones".into(),
                "elements".into(),
                "type".into()
            ],
        },
    ],
    jtd::validate(&schema, &input_bad, Default::default()).unwrap(),
);
```

## What is JSON Type Definition?

[JSON Type Definition](https://jsontypedef.com) is a schema format for JSON
data. A JSON Type Definition schema describes what is and isn't a "valid" JSON
document. JSON Type Definition is easy to learn, portable (there are
functionally-identical implementations across many programming languages) and
standardized (the spec is set in stone as [IETF RFC
8927](https://tools.ietf.org/html/rfc8927)).

Here's an example of a JSON Type Definition schema:

```json
{
    "properties": {
        "name": {
            "type": "string"
        },
        "isAdmin": {
            "type": "boolean"
        }
    }
}
```

This schema considers any object with a `name` property (whose value must be a
string), an `isAdmin` property (whose value must a boolean), and no other
properties, to be valid.

To learn more about JSON Type Definition, [check out the online documentation at
jsontypedef.com](https://jsontypedef.com).

## Installation

Install this crate by adding the following to your `Cargo.toml`:

```toml
jtd = "0.2"
```

## Usage

For detailed documentation on how to use this crate, consult [the full API
documentation on docs.rs](https://docs.rs/jtd).
