use crate::form;
use crate::serde;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;

#[derive(Debug, Default, PartialEq)]
pub struct Schema {
    pub definitions: HashMap<String, Schema>,
    pub form: form::Form,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, PartialEq)]
pub enum SerdeConvertError {
    NonRootDefinitions,
    InvalidType(String),
    DuplicatedEnumValue(String),
    RepeatedProperty(String),
    InvalidForm,
}

impl Schema {
    fn from_serde(root: bool, schema: serde::Schema) -> Result<Self, SerdeConvertError> {
        if let Some(ref_) = schema.ref_ {
            return Ok(Schema {
                definitions: HashMap::new(),
                form: form::Form::Ref(form::Ref {
                    nullable: schema.nullable.unwrap_or_default(),
                    definition: ref_,
                }),
                metadata: HashMap::new(),
            });
        }

        if let Some(type_) = schema.type_ {
            return Ok(Schema {
                definitions: HashMap::new(),
                form: form::Form::Type(form::Type {
                    nullable: schema.nullable.unwrap_or_default(),
                    type_value: type_
                        .parse()
                        .map_err(|_| SerdeConvertError::InvalidType(type_))?,
                }),
                metadata: HashMap::new(),
            });
        }

        if let Some(enum_) = schema.enum_ {
            let mut values = HashSet::new();
            for val in enum_ {
                if values.contains(&val) {
                    return Err(SerdeConvertError::DuplicatedEnumValue(val));
                }

                values.insert(val);
            }

            return Ok(Schema {
                definitions: HashMap::new(),
                form: form::Form::Enum(form::Enum {
                    nullable: schema.nullable.unwrap_or_default(),
                    values,
                }),
                metadata: HashMap::new(),
            });
        }

        if let Some(elements) = schema.elements {
            return Ok(Schema {
                definitions: HashMap::new(),
                form: form::Form::Elements(form::Elements {
                    nullable: schema.nullable.unwrap_or_default(),
                    schema: Box::new(Self::from_serde(false, *elements)?),
                }),
                metadata: HashMap::new(),
            });
        }

        if schema.properties.is_some() || schema.optional_properties.is_some() {
            let has_required = schema.properties.is_some();

            let mut required = HashMap::new();
            for (name, sub_schema) in schema.properties.unwrap_or_default() {
                required.insert(name, Self::from_serde(false, sub_schema)?);
            }

            let mut optional = HashMap::new();
            for (name, sub_schema) in schema.optional_properties.unwrap_or_default() {
                if required.contains_key(&name) {
                    return Err(SerdeConvertError::RepeatedProperty(name));
                }

                optional.insert(name, Self::from_serde(false, sub_schema)?);
            }

            return Ok(Schema {
                definitions: HashMap::new(),
                form: form::Form::Properties(form::Properties {
                    nullable: schema.nullable.unwrap_or_default(),
                    required,
                    optional,
                    additional: schema.additional_properties.unwrap_or_default(),
                    has_required,
                }),
                metadata: HashMap::new(),
            });
        }

        Ok(Schema {
            definitions: HashMap::new(),
            form: form::Form::Empty,
            metadata: schema.metadata.unwrap_or_default(),
        })
    }
}

impl TryFrom<serde::Schema> for Schema {
    type Error = SerdeConvertError;

    fn try_from(schema: serde::Schema) -> Result<Self, Self::Error> {
        Self::from_serde(true, schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::convert::TryInto;

    #[test]
    fn from_empty() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Empty,
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({}))
                .unwrap()
                .try_into(),
        )
    }

    #[test]
    fn from_empty_with_metadata() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Empty,
                metadata: vec![("foo".to_owned(), json!("bar"))].into_iter().collect(),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "metadata": {
                    "foo": "bar"
                }
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_ref() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Ref(form::Ref {
                    nullable: false,
                    definition: "foo".to_owned(),
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "ref": "foo",
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_ref_with_nullable() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Ref(form::Ref {
                    nullable: true,
                    definition: "foo".to_owned(),
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "ref": "foo",
                "nullable": true,
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_type() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Type(form::Type {
                    nullable: false,
                    type_value: form::TypeValue::Boolean
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "type": "boolean",
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_type_with_nullable() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Type(form::Type {
                    nullable: true,
                    type_value: form::TypeValue::Boolean
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "type": "boolean",
                "nullable": true,
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_type_with_invalid_value() {
        let result: Result<Schema, SerdeConvertError> =
            serde_json::from_value::<serde::Schema>(json!({
                "type": "foo",
            }))
            .unwrap()
            .try_into();

        assert_eq!(
            Err(SerdeConvertError::InvalidType("foo".to_owned())),
            result
        )
    }

    #[test]
    fn from_enum() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Enum(form::Enum {
                    nullable: false,
                    values: vec!["foo".to_owned(), "bar".to_owned()]
                        .into_iter()
                        .collect(),
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "enum": ["foo", "bar"],
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_enum_with_nullable() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Enum(form::Enum {
                    nullable: true,
                    values: vec!["foo".to_owned(), "bar".to_owned()]
                        .into_iter()
                        .collect(),
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "enum": ["foo", "bar"],
                "nullable": true,
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_enum_with_repeated_value() {
        let result: Result<Schema, SerdeConvertError> =
            serde_json::from_value::<serde::Schema>(json!({
                "enum": ["foo", "bar", "foo"],
            }))
            .unwrap()
            .try_into();

        assert_eq!(
            Err(SerdeConvertError::DuplicatedEnumValue("foo".to_owned())),
            result
        )
    }

    #[test]
    fn from_elements() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Elements(form::Elements {
                    nullable: false,
                    schema: Default::default(),
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "elements": {},
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_elements_with_nullable() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Elements(form::Elements {
                    nullable: true,
                    schema: Default::default(),
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "elements": {},
                "nullable": true,
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_properties() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Properties(form::Properties {
                    nullable: false,
                    required: vec![("foo".to_owned(), Default::default())]
                        .into_iter()
                        .collect(),
                    optional: vec![("bar".to_owned(), Default::default())]
                        .into_iter()
                        .collect(),
                    additional: false,
                    has_required: true,
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "properties": {
                    "foo": {},
                },
                "optionalProperties": {
                    "bar": {},
                },
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_properties_without_optional() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Properties(form::Properties {
                    nullable: false,
                    required: vec![("foo".to_owned(), Default::default())]
                        .into_iter()
                        .collect(),
                    optional: HashMap::new(),
                    additional: false,
                    has_required: true,
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "properties": {
                    "foo": {},
                },
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_properties_without_required() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Properties(form::Properties {
                    nullable: false,
                    required: HashMap::new(),
                    optional: vec![("foo".to_owned(), Default::default())]
                        .into_iter()
                        .collect(),
                    additional: false,
                    has_required: false,
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "optionalProperties": {
                    "foo": {},
                },
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_properties_with_additional() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Properties(form::Properties {
                    nullable: false,
                    required: vec![("foo".to_owned(), Default::default())]
                        .into_iter()
                        .collect(),
                    optional: vec![("bar".to_owned(), Default::default())]
                        .into_iter()
                        .collect(),
                    additional: true,
                    has_required: true,
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "properties": {
                    "foo": {},
                },
                "optionalProperties": {
                    "bar": {},
                },
                "additionalProperties": true,
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_properties_with_nullable() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Properties(form::Properties {
                    nullable: true,
                    required: vec![("foo".to_owned(), Default::default())]
                        .into_iter()
                        .collect(),
                    optional: vec![("bar".to_owned(), Default::default())]
                        .into_iter()
                        .collect(),
                    additional: false,
                    has_required: true,
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "properties": {
                    "foo": {},
                },
                "optionalProperties": {
                    "bar": {},
                },
                "nullable": true,
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_properties_with_repeated_keys() {
        let result: Result<Schema, SerdeConvertError> =
            serde_json::from_value::<serde::Schema>(json!({
                "properties": {
                    "foo": {},
                },
                "optionalProperties": {
                    "foo": {},
                },
                "nullable": true,
            }))
            .unwrap()
            .try_into();

        assert_eq!(
            Err(SerdeConvertError::RepeatedProperty("foo".to_owned())),
            result
        )
    }
}
