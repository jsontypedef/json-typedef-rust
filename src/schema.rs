use crate::form;
use crate::serde;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::convert::{TryFrom, TryInto};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Schema {
    pub definitions: HashMap<String, Schema>,
    pub form: form::Form,
    pub metadata: HashMap<String, Value>,
}

#[cfg(feature = "fuzz")]
impl arbitrary::Arbitrary for Schema {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(Schema {
            definitions: arbitrary::Arbitrary::arbitrary(u)?,
            form: arbitrary::Arbitrary::arbitrary(u)?,

            // serde_json::Value does not derive Arbitrary. That's ok, because
            // for the fuzz tests we're doing, we don't really care about
            // manipulating arbitrary JSON values.
            //
            // So we'll always have metadata be None.
            metadata: HashMap::new(),
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum SerdeConvertError {
    InvalidForm,
    InvalidType(String),
    DuplicatedEnumValue(String),
}

#[derive(Debug, PartialEq)]
pub enum ValidateError {
    NoSuchDefinition(String),
    NonRootDefinitions,
    EmptyEnum,
    RepeatedProperty(String),
    MappingNullable,
    MappingNotPropertiesForm,
}

// Index of valid form "signatures" -- i.e., combinations of the presence of the
// keywords (in order):
//
// ref type enum elements properties optionalProperties additionalProperties
// values discriminator mapping
//
// The keywords "definitions", "nullable", and "metadata" are not included here,
// because they would restrict nothing.
const VALID_FORM_SIGNATURES: [[bool; 10]; 13] = [
    // Empty form
    [
        false, false, false, false, false, false, false, false, false, false,
    ],
    // Ref form
    [
        true, false, false, false, false, false, false, false, false, false,
    ],
    // Type form
    [
        false, true, false, false, false, false, false, false, false, false,
    ],
    // Enum form
    [
        false, false, true, false, false, false, false, false, false, false,
    ],
    // Elements form
    [
        false, false, false, true, false, false, false, false, false, false,
    ],
    // Properties form -- properties or optional properties or both, and never
    // additional properties on its own
    [
        false, false, false, false, true, false, false, false, false, false,
    ],
    [
        false, false, false, false, false, true, false, false, false, false,
    ],
    [
        false, false, false, false, true, true, false, false, false, false,
    ],
    [
        false, false, false, false, true, false, true, false, false, false,
    ],
    [
        false, false, false, false, false, true, true, false, false, false,
    ],
    [
        false, false, false, false, true, true, true, false, false, false,
    ],
    // Values form
    [
        false, false, false, false, false, false, false, true, false, false,
    ],
    // Discriminator form
    [
        false, false, false, false, false, false, false, false, true, true,
    ],
];

impl Schema {
    pub fn validate(&self) -> Result<(), ValidateError> {
        self.validate_with_root(None)
    }

    fn validate_with_root(&self, root: Option<&Self>) -> Result<(), ValidateError> {
        if root.is_none() && !self.definitions.is_empty() {
            return Err(ValidateError::NonRootDefinitions);
        }

        match &self.form {
            form::Form::Empty | form::Form::Type(_) => {}
            form::Form::Enum(form::Enum { values, .. }) => {
                if values.is_empty() {
                    return Err(ValidateError::EmptyEnum);
                }
            }
            form::Form::Ref(form::Ref { definition, .. }) => {
                if !root.unwrap_or(&self).definitions.contains_key(definition) {
                    return Err(ValidateError::NoSuchDefinition(definition.clone()));
                }
            }
            form::Form::Elements(form::Elements { schema, .. }) => {
                schema.validate_with_root(root)?;
            }
            form::Form::Properties(form::Properties {
                required, optional, ..
            }) => {
                for schema in required.values() {
                    schema.validate_with_root(root)?;
                }

                for (name, schema) in optional {
                    if required.contains_key(name) {
                        return Err(ValidateError::RepeatedProperty(name.clone()));
                    }

                    schema.validate_with_root(root)?;
                }
            }
            form::Form::Values(form::Values { schema, .. }) => {
                schema.validate_with_root(root)?;
            }
            form::Form::Discriminator(form::Discriminator {
                discriminator,
                mapping,
                ..
            }) => {
                for schema in mapping.values() {
                    schema.validate_with_root(root)?;

                    match &schema.form {
                        form::Form::Properties(form::Properties {
                            required,
                            optional,
                            nullable,
                            ..
                        }) => {
                            if *nullable {
                                return Err(ValidateError::MappingNullable);
                            }

                            if required.contains_key(discriminator)
                                || optional.contains_key(discriminator)
                            {
                                return Err(ValidateError::RepeatedProperty(discriminator.clone()));
                            }
                        }
                        _ => {
                            return Err(ValidateError::MappingNotPropertiesForm);
                        }
                    }
                }
            }
        };

        Ok(())
    }
}

impl TryFrom<serde::Schema> for Schema {
    type Error = SerdeConvertError;

    fn try_from(schema: serde::Schema) -> Result<Self, Self::Error> {
        let form_signature = [
            schema.ref_.is_some(),
            schema.type_.is_some(),
            schema.enum_.is_some(),
            schema.elements.is_some(),
            schema.properties.is_some(),
            schema.optional_properties.is_some(),
            schema.additional_properties.is_some(),
            schema.values.is_some(),
            schema.discriminator.is_some(),
            schema.mapping.is_some(),
        ];

        if !VALID_FORM_SIGNATURES.contains(&form_signature) {
            return Err(SerdeConvertError::InvalidForm);
        }

        let mut definitions = HashMap::new();
        for (name, sub_schema) in schema.definitions.unwrap_or_default() {
            definitions.insert(name, sub_schema.try_into()?);
        }

        if let Some(ref_) = schema.ref_ {
            return Ok(Schema {
                definitions,
                form: form::Form::Ref(form::Ref {
                    nullable: schema.nullable.unwrap_or_default(),
                    definition: ref_,
                }),
                metadata: schema.metadata.unwrap_or_default(),
            });
        }

        if let Some(type_) = schema.type_ {
            return Ok(Schema {
                definitions,
                form: form::Form::Type(form::Type {
                    nullable: schema.nullable.unwrap_or_default(),
                    type_value: type_
                        .parse()
                        .map_err(|_| SerdeConvertError::InvalidType(type_))?,
                }),
                metadata: schema.metadata.unwrap_or_default(),
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
                definitions,
                form: form::Form::Enum(form::Enum {
                    nullable: schema.nullable.unwrap_or_default(),
                    values,
                }),
                metadata: schema.metadata.unwrap_or_default(),
            });
        }

        if let Some(elements) = schema.elements {
            return Ok(Schema {
                definitions,
                form: form::Form::Elements(form::Elements {
                    nullable: schema.nullable.unwrap_or_default(),
                    schema: Box::new((*elements).try_into()?),
                }),
                metadata: schema.metadata.unwrap_or_default(),
            });
        }

        if schema.properties.is_some() || schema.optional_properties.is_some() {
            let has_required = schema.properties.is_some();

            let mut required = HashMap::new();
            for (name, sub_schema) in schema.properties.unwrap_or_default() {
                required.insert(name, sub_schema.try_into()?);
            }

            let mut optional = HashMap::new();
            for (name, sub_schema) in schema.optional_properties.unwrap_or_default() {
                optional.insert(name, sub_schema.try_into()?);
            }

            return Ok(Schema {
                definitions,
                form: form::Form::Properties(form::Properties {
                    nullable: schema.nullable.unwrap_or_default(),
                    required,
                    optional,
                    additional: schema.additional_properties.unwrap_or_default(),
                    has_required,
                }),
                metadata: schema.metadata.unwrap_or_default(),
            });
        }

        if let Some(values) = schema.values {
            return Ok(Schema {
                definitions,
                form: form::Form::Values(form::Values {
                    nullable: schema.nullable.unwrap_or_default(),
                    schema: Box::new((*values).try_into()?),
                }),
                metadata: schema.metadata.unwrap_or_default(),
            });
        }

        if let Some(discriminator) = schema.discriminator {
            let mut mapping = HashMap::new();
            for (name, sub_schema) in schema.mapping.unwrap() {
                mapping.insert(name, sub_schema.try_into()?);
            }

            return Ok(Schema {
                definitions,
                form: form::Form::Discriminator(form::Discriminator {
                    nullable: schema.nullable.unwrap_or_default(),
                    discriminator,
                    mapping,
                }),
                metadata: schema.metadata.unwrap_or_default(),
            });
        }

        Ok(Schema {
            definitions,
            form: form::Form::Empty,
            metadata: schema.metadata.unwrap_or_default(),
        })
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
    fn from_empty_with_definitions() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Empty,
                definitions: vec![("foo".to_owned(), Default::default())]
                    .into_iter()
                    .collect(),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "definitions": {
                    "foo": {}
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
    fn from_values() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Values(form::Values {
                    nullable: false,
                    schema: Default::default(),
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "values": {},
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_values_with_nullable() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Values(form::Values {
                    nullable: true,
                    schema: Default::default(),
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "values": {},
                "nullable": true,
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_discriminator() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Discriminator(form::Discriminator {
                    nullable: false,
                    discriminator: "foo".to_owned(),
                    mapping: vec![("bar".to_owned(), Default::default())]
                        .into_iter()
                        .collect(),
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "discriminator": "foo",
                "mapping": {
                    "bar": {}
                }
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_discriminator_with_nullable() {
        assert_eq!(
            Ok(Schema {
                form: form::Form::Discriminator(form::Discriminator {
                    nullable: true,
                    discriminator: "foo".to_owned(),
                    mapping: vec![("bar".to_owned(), Default::default())]
                        .into_iter()
                        .collect(),
                }),
                ..Default::default()
            }),
            serde_json::from_value::<serde::Schema>(json!({
                "discriminator": "foo",
                "mapping": {
                    "bar": {}
                },
                "nullable": true,
            }))
            .unwrap()
            .try_into(),
        )
    }

    #[test]
    fn from_invalid_forms() {
        let invalid_forms = vec![
            json!({"ref": "foo", "type": "uint32"}),
            json!({"type": "uint32", "enum": ["foo"]}),
            json!({"enum": ["foo"], "elements": {}}),
            json!({"elements": {}, "properties": {}}),
            json!({"elements": {}, "optionalProperties": {}}),
            json!({"elements": {}, "additionalProperties": true}),
            json!({"properties": {}, "values": {}}),
            json!({"values": {}, "discriminator": "foo"}),
            json!({"discriminator": "foo"}),
            json!({"mapping": {}}),
        ];

        for invalid_form in invalid_forms {
            let result: Result<Schema, SerdeConvertError> =
                serde_json::from_value::<serde::Schema>(invalid_form)
                    .unwrap()
                    .try_into();
            assert_eq!(Err(SerdeConvertError::InvalidForm), result);
        }
    }

    #[test]
    fn from_empty_with_definitions_containing_definitions() {
        let schema: Schema = serde_json::from_value::<serde::Schema>(json!({
            "definitions": {
                "foo": {
                    "definitions": {"foo": {}}
                }
            },
        }))
        .unwrap()
        .try_into()
        .unwrap();

        assert_eq!(Err(ValidateError::NonRootDefinitions), schema.validate());
    }

    #[test]
    fn from_enum_with_empty_array() {
        let schema: Schema = serde_json::from_value::<serde::Schema>(json!({
            "enum": []
        }))
        .unwrap()
        .try_into()
        .unwrap();

        assert_eq!(Err(ValidateError::EmptyEnum), schema.validate());
    }

    #[test]
    fn from_properties_with_repeated_keys() {
        let schema: Schema = serde_json::from_value::<serde::Schema>(json!({
            "properties": {
                "foo": {},
            },
            "optionalProperties": {
                "foo": {},
            },
            "nullable": true,
        }))
        .unwrap()
        .try_into()
        .unwrap();

        assert_eq!(
            Err(ValidateError::RepeatedProperty("foo".to_owned())),
            schema.validate()
        );
    }

    #[test]
    fn from_discriminator_with_non_properties_mapping() {
        let schema: Schema = serde_json::from_value::<serde::Schema>(json!({
            "discriminator": "foo",
            "mapping": {
                "foo": {
                    "values": {}
                }
            }
        }))
        .unwrap()
        .try_into()
        .unwrap();

        assert_eq!(
            Err(ValidateError::MappingNotPropertiesForm),
            schema.validate()
        );
    }

    #[test]
    fn from_discriminator_with_mapping_redefining_discriminator() {
        let schema: Schema = serde_json::from_value::<serde::Schema>(json!({
            "discriminator": "foo",
            "mapping": {
                "foo": {
                    "properties": { "foo": {}}
                }
            }
        }))
        .unwrap()
        .try_into()
        .unwrap();

        assert_eq!(
            Err(ValidateError::RepeatedProperty("foo".to_owned())),
            schema.validate()
        );
    }

    #[test]
    fn spec_invalid_schemas_suite() {
        let test_cases: HashMap<String, Value> = serde_json::from_str(include_str!(
            "../json-typedef-spec/tests/invalid_schemas.json"
        ))
        .unwrap();

        for (name, invalid_schema) in test_cases {
            dbg!(&invalid_schema);
            if let Ok(schema) = serde_json::from_value::<serde::Schema>(invalid_schema) {
                dbg!(&schema);
                let result: Result<Schema, SerdeConvertError> = schema.try_into();

                if let Ok(schema) = result {
                    dbg!(&name, &schema);
                    assert!(schema.validate().is_err(), name);
                }
            }
        }
    }
}
