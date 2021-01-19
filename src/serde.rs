use crate::schema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Schema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definitions: Option<BTreeMap<String, Schema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub elements: Option<Box<Schema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<BTreeMap<String, Schema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_properties: Option<BTreeMap<String, Schema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Box<Schema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub discriminator: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapping: Option<BTreeMap<String, Schema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, Value>>,
}

impl From<schema::Schema> for Schema {
    fn from(schema: schema::Schema) -> Schema {
        use crate::form;

        let mut out = Schema::default();

        if !schema.definitions.is_empty() {
            out.definitions = Some(
                schema
                    .definitions
                    .into_iter()
                    .map(|(k, v)| (k, v.into()))
                    .collect(),
            );
        }

        match schema.form {
            form::Form::Empty => {}
            form::Form::Ref(form::Ref {
                nullable,
                definition,
            }) => {
                if nullable {
                    out.nullable = Some(true);
                }

                out.ref_ = Some(definition);
            }
            form::Form::Type(form::Type {
                nullable,
                type_value,
            }) => {
                if nullable {
                    out.nullable = Some(true);
                }

                out.type_ = Some(
                    match type_value {
                        form::TypeValue::Boolean => "boolean",
                        form::TypeValue::Float32 => "float32",
                        form::TypeValue::Float64 => "float64",
                        form::TypeValue::Int8 => "int8",
                        form::TypeValue::Uint8 => "uint8",
                        form::TypeValue::Int16 => "int16",
                        form::TypeValue::Uint16 => "uint16",
                        form::TypeValue::Int32 => "int32",
                        form::TypeValue::Uint32 => "uint32",
                        form::TypeValue::String => "string",
                        form::TypeValue::Timestamp => "timestamp",
                    }
                    .to_owned(),
                )
            }
            form::Form::Enum(form::Enum { nullable, values }) => {
                if nullable {
                    out.nullable = Some(true);
                }

                out.enum_ = Some(values.into_iter().collect());
            }
            form::Form::Elements(form::Elements { nullable, schema }) => {
                if nullable {
                    out.nullable = Some(true);
                }

                out.elements = Some(Box::new((*schema).into()));
            }
            form::Form::Properties(form::Properties {
                nullable,
                required,
                optional,
                additional,
                has_required,
            }) => {
                if nullable {
                    out.nullable = Some(true);
                }

                if has_required {
                    out.properties =
                        Some(required.into_iter().map(|(k, v)| (k, v.into())).collect());
                }

                if !optional.is_empty() {
                    out.optional_properties =
                        Some(optional.into_iter().map(|(k, v)| (k, v.into())).collect());
                }

                if additional {
                    out.additional_properties = Some(true);
                }
            }
            form::Form::Values(form::Values { nullable, schema }) => {
                if nullable {
                    out.nullable = Some(true);
                }

                out.values = Some(Box::new((*schema).into()));
            }
            form::Form::Discriminator(form::Discriminator {
                nullable,
                discriminator,
                mapping,
            }) => {
                if nullable {
                    out.nullable = Some(true);
                }

                out.discriminator = Some(discriminator);
                out.mapping = Some(mapping.into_iter().map(|(k, v)| (k, v.into())).collect());
            }
        }

        if !schema.metadata.is_empty() {
            out.metadata = Some(schema.metadata);
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn serialize_partial() {
        // Fields are None by default. These shouldn't be serialized.
        assert_eq!(
            "{\"ref\":\"foo\"}",
            serde_json::to_string(&super::Schema {
                ref_: Some("foo".to_owned()),
                ..Default::default()
            })
            .unwrap()
        );
    }

    #[test]
    fn parse_empty() {
        assert_eq!(
            super::Schema::default(),
            serde_json::from_value(json!({})).unwrap()
        );
    }

    #[test]
    fn parse_partial() {
        assert_eq!(
            super::Schema {
                nullable: Some(true),
                optional_properties: Some(
                    vec![(
                        "foo".to_owned(),
                        super::Schema {
                            type_: Some("uint32".to_owned()),
                            ..Default::default()
                        }
                    )]
                    .into_iter()
                    .collect()
                ),
                ..Default::default()
            },
            serde_json::from_value(json!({
                "optionalProperties": {
                    "foo": {
                        "type": "uint32",
                    },
                },
                "nullable": true,
            }))
            .unwrap()
        );
    }

    #[test]
    fn parse_full() {
        assert_eq!(
            super::Schema {
                definitions: Some(
                    vec![(
                        "foo".to_owned(),
                        super::Schema {
                            type_: Some("uint32".to_owned()),
                            ..Default::default()
                        }
                    )]
                    .into_iter()
                    .collect()
                ),
                nullable: Some(true),
                ref_: Some("foo".to_owned()),
                type_: Some("uint32".to_owned()),
                enum_: Some(vec!["foo".to_owned(), "bar".to_owned()]),
                elements: Some(Box::new(super::Schema {
                    type_: Some("uint32".to_owned()),
                    ..Default::default()
                })),
                properties: Some(
                    vec![(
                        "foo".to_owned(),
                        super::Schema {
                            type_: Some("uint32".to_owned()),
                            ..Default::default()
                        }
                    )]
                    .into_iter()
                    .collect()
                ),
                optional_properties: Some(
                    vec![(
                        "foo".to_owned(),
                        super::Schema {
                            type_: Some("uint32".to_owned()),
                            ..Default::default()
                        }
                    )]
                    .into_iter()
                    .collect()
                ),
                additional_properties: Some(true),
                values: Some(Box::new(super::Schema {
                    type_: Some("uint32".to_owned()),
                    ..Default::default()
                })),
                discriminator: Some("foo".to_owned()),
                mapping: Some(
                    vec![(
                        "foo".to_owned(),
                        super::Schema {
                            type_: Some("uint32".to_owned()),
                            ..Default::default()
                        }
                    )]
                    .into_iter()
                    .collect()
                ),
                metadata: Some(vec![("foo".to_owned(), json!("bar"))].into_iter().collect()),
            },
            serde_json::from_value(json!({
                "definitions": {
                    "foo": {
                        "type": "uint32",
                    },
                },
                "nullable": true,
                "ref": "foo",
                "type": "uint32",
                "enum": ["foo", "bar"],
                "elements": {
                    "type": "uint32",
                },
                "properties": {
                    "foo": {
                        "type": "uint32",
                    },
                },
                "optionalProperties": {
                    "foo": {
                        "type": "uint32",
                    },
                },
                "additionalProperties": true,
                "values": {
                    "type": "uint32",
                },
                "discriminator": "foo",
                "mapping": {
                    "foo": {
                        "type": "uint32",
                    },
                },
                "metadata": {
                    "foo": "bar",
                },
            }))
            .unwrap()
        );
    }

    #[test]
    fn from_empty() {
        assert_roundtrip_try_into_from(json!({}));
    }

    #[test]
    fn from_ref() {
        assert_roundtrip_try_into_from(json!({"ref": "foo"}));
        assert_roundtrip_try_into_from(json!({"ref": "foo", "nullable": true}));
    }

    #[test]
    fn from_type() {
        assert_roundtrip_try_into_from(json!({"type": "boolean"}));
        assert_roundtrip_try_into_from(json!({"type": "boolean", "nullable": true}));

        assert_roundtrip_try_into_from(json!({"type": "int8"}));
        assert_roundtrip_try_into_from(json!({"type": "uint8"}));
        assert_roundtrip_try_into_from(json!({"type": "int16"}));
        assert_roundtrip_try_into_from(json!({"type": "uint16"}));
        assert_roundtrip_try_into_from(json!({"type": "int32"}));
        assert_roundtrip_try_into_from(json!({"type": "uint32"}));
        assert_roundtrip_try_into_from(json!({"type": "string"}));
        assert_roundtrip_try_into_from(json!({"type": "timestamp"}));
    }

    #[test]
    fn from_enum() {
        assert_roundtrip_try_into_from(json!({ "enum": ["foo"] }));
        assert_roundtrip_try_into_from(json!({ "enum": ["foo"], "nullable": true }));
    }

    #[test]
    fn from_elements() {
        assert_roundtrip_try_into_from(json!({ "elements": { "type": "boolean" } }));
        assert_roundtrip_try_into_from(
            json!({ "elements": { "type": "boolean" }, "nullable": true }),
        );
    }

    #[test]
    fn from_properties() {
        assert_roundtrip_try_into_from(json!({ "properties": { "foo": { "type": "boolean" }}}));
        assert_roundtrip_try_into_from(
            json!({ "optionalProperties": { "foo": { "type": "boolean" }}}),
        );
        assert_roundtrip_try_into_from(
            json!({ "properties": { "foo": { "type": "boolean" }}, "nullable": true }),
        );
        assert_roundtrip_try_into_from(
            json!({ "optionalProperties": { "foo": { "type": "boolean" }}, "nullable": true }),
        );
        assert_roundtrip_try_into_from(json!({
            "properties": { "foo": { "type": "boolean" }},
            "optionalProperties": { "bar": { "type": "boolean" }},
        }));
        assert_roundtrip_try_into_from(json!({
            "properties": { "foo": { "type": "boolean" }},
            "optionalProperties": { "bar": { "type": "boolean" }},
            "nullable": true,
        }));
    }

    #[test]
    fn from_values() {
        assert_roundtrip_try_into_from(json!({ "values": { "type": "boolean" } }));
        assert_roundtrip_try_into_from(
            json!({ "values": { "type": "boolean" }, "nullable": true }),
        );
    }

    #[test]
    fn from_discriminator() {
        assert_roundtrip_try_into_from(json!({
            "discriminator": "foo",
            "mapping": {
                "foo": {
                    "properties": { "bar": { "type": "boolean" }},
                },
            },
        }));

        assert_roundtrip_try_into_from(json!({
            "discriminator": "foo",
            "mapping": {
                "foo": {
                    "properties": { "bar": { "type": "boolean" }}
                }
            },
            "nullable": true,
        }));
    }

    fn assert_roundtrip_try_into_from(json: serde_json::Value) {
        use crate::schema;
        use std::convert::TryInto;

        let serde_schema: super::Schema = serde_json::from_value(json).unwrap();
        let schema: schema::Schema = serde_schema.clone().try_into().unwrap();

        assert_eq!(serde_schema, schema.into());
    }
}
