//! An implementation of [JSON Type Definition](https://jsontypedef.com), [RFC
//! 8927](https://tools.ietf.org/html/rfc8927).
//!
//! `jtd` lets you parse and ensure the validity of JSON Typedef schemas, and
//! then validate JSON data against those schemas. If your goal is instead to
//! generate Rust types from JSON Typedef schemas, see
//! [`jtd-codegen`](https://github.com/jsontypedef/json-typedef-codegen).
//!
//! # Quick start
//!
//! Here's how you can parse a JSON Typedef schema and then use it to validate
//! data against that schema.
//!
//! ```
//! use jtd::Schema;
//! use serde_json::json;
//!
//! let schema = Schema::from_serde_schema(
//!     serde_json::from_value(json!({
//!         "properties": {
//!             "foo": { "type": "string" },
//!             "bar": { "type": "boolean" }
//!         }
//!     }))
//!     .expect("Parse schema"),
//! )
//! .expect("Construct schema from JSON data");
//!
//! schema.validate().expect("Invalid schema");
//!
//! // This input is ok, so validate comes back empty.
//! let input_ok = json!({ "foo": "xxx", "bar": true });
//! assert!(jtd::validate(&schema, &input_ok, Default::default()).unwrap().is_empty());
//!
//! // This input is bad (bar has type string, not boolean), so validate does
//! // not come back empty.
//! let input_bad = json!({ "foo": "xxx", "bar": "false" });
//! assert!(!jtd::validate(&schema, &input_bad, Default::default()).unwrap().is_empty());
//! ```
//!
//! Or, at a high level:
//!
//! 1. Use `serde_json` to parse JSON data into a [`SerdeSchema`].
//! 2. Convert that into a [`Schema`] using [`Schema::from_serde_schema`].
//! 3. Optionally, ensure that schema is "valid" using [`Schema::validate`].
//! 4. Verify data against that schema using [`validate()`].
//!
//! # Common usage
//!
//! The example above shows you how you can quickly use JSON Typedef to check
//! whether data is valid. But in the real world, you usually want to know what
//! the validation errors were, rather than just flatly rejecting input as
//! "invalid" without any further details.
//!
//! One benefit of JSON Type Definition is that the exact data inside the
//! validation errors is part of the specification; that means validation errors
//! are portable. Here's an example of what those validation errors look like,
//! and how you can access them with this crate.
//!
//! ```
//! use jtd::{Schema, ValidationErrorIndicator};
//! use serde_json::json;
//!
//! let schema = Schema::from_serde_schema(
//!     serde_json::from_value(json!({
//!         "properties": {
//!             "name": { "type": "string" },
//!             "age": { "type": "uint32" },
//!             "phones": {
//!                 "elements": {
//!                     "type": "string"
//!                 }
//!             }
//!         }
//!     }))
//!     .expect("Parse schema"),
//! )
//! .expect("Construct schema from JSON data");
//!
//! schema.validate().expect("Invalid schema");
//!
//! // Since this first example is valid, we'll get back an empty list of
//! // validation errors.
//! let input_ok = json!({
//!     "name": "John Doe",
//!     "age": 43,
//!     "phones": ["+44 1234567", "+44 2345678"]
//! });
//!
//! assert_eq!(
//!     Vec::<ValidationErrorIndicator>::new(),
//!     jtd::validate(&schema, &input_ok, Default::default()).unwrap(),
//! );
//!
//! // This example is invalid, so we'll get back three validation errors:
//! //
//! // 1. "name" is required but not present,
//! // 2. "age" has the wrong type
//! // 3. "phones[1]" has the wrong type
//! let input_bad = json!({
//!     "age": "43",
//!     "phones": ["+44 1234567", 442345678]
//! });
//!
//! // Each error indicator has two pieces of information: the path to the part
//! // of the input that was rejected (the "instance path"), and the part of the
//! // schema that rejected it (the "schema path").
//! //
//! // The exact values of the instance path and schema path is specified in the
//! // JSON Type Definition spec.
//! assert_eq!(
//!     vec![
//!         // "age" has the wrong type (required by "/properties/age/type")
//!         ValidationErrorIndicator {
//!             instance_path: vec!["age".into()],
//!             schema_path: vec!["properties".into(), "age".into(), "type".into()],
//!         },
//!
//!         // "name" is missing (required by "/properties/name")
//!         ValidationErrorIndicator {
//!             instance_path: vec![],
//!             schema_path: vec!["properties".into(), "name".into()],
//!         },
//!
//!         // "phones/1" has the wrong type (required by "/properties/phones/elements/type")
//!         ValidationErrorIndicator {
//!             instance_path: vec!["phones".into(), "1".into()],
//!             schema_path: vec![
//!                 "properties".into(),
//!                 "phones".into(),
//!                 "elements".into(),
//!                 "type".into()
//!             ],
//!         },
//!     ],
//!     jtd::validate(&schema, &input_bad, Default::default()).unwrap(),
//! );
//! ```
//!
//! # Advanced usage
//!
//! The examples above skim over some details of how you can use this crate.
//! Here are pieces of documentation that you may find relevant:
//!
//! * If you want to convert JSON Type Defintion schemas to/from JSON, and
//!   validate whether a schema is valid, see [`SerdeSchema`],
//!   [`Schema::from_serde_schema`], and [`Schema::validate`].
//!
//! * If you want better performance out of [`validate()`], see
//!   [`ValidateOptions`] to see how you can make validation faster.
//!
//! # Security considerations
//!
//! If you're running [`validate()`] with untrusted schemas (untrusted inputs is
//! fine), then be aware of this security consideration from RFC 8927:
//!
//! > Implementations that evaluate user-inputted schemas SHOULD implement
//! > mechanisms to detect and abort circular references that might cause a
//! > naive implementation to go into an infinite loop.  Without such
//! > mechanisms, implementations may be vulnerable to denial-of-service
//! > attacks.
//!
//! This crate supports that "detect and abort" mechanism via
//! [`ValidateOptions::with_max_depth`]. Please see that documentation if you're
//! validating data against untrusted schemas.

mod schema;
mod serde_schema;
mod validate;

pub use schema::*;
pub use serde_schema::*;
pub use validate::*;
