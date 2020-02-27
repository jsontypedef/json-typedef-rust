use crate::form;
use crate::serde;
use serde_json::Value;
use std::collections::HashMap;
use std::convert::TryFrom;

#[derive(Debug, Default, PartialEq)]
pub struct Schema {
    pub definitions: HashMap<String, Schema>,
    pub form: form::Form,
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, PartialEq)]
pub enum SerdeConvertError {
    NonRootDefinitions,
    InvalidType,
    DuplicatedEnumValue,
    RepeatedProperty,
    InvalidForm,
}

impl Schema {
    fn from_serde(root: bool, schema: serde::Schema) -> Result<Self, SerdeConvertError> {
        if let Some(ref_) = schema.ref_ {
            return Ok(Schema {
                definitions: HashMap::new(),
                form: form::Form::Ref(form::Ref {
                    nullable: false,
                    definition: ref_,
                }),
                extra: HashMap::new(),
            });
        }

        if let Some(type_) = schema.type_ {
            return Ok(Schema {
                definitions: HashMap::new(),
                form: form::Form::Type(form::Type {
                    nullable: false,
                    type_value: type_.parse().map_err(|_| SerdeConvertError::InvalidType)?,
                }),
                extra: HashMap::new(),
            });
        }

        if let Some(enum_) = schema.enum_ {
            return Ok(Schema {
                definitions: HashMap::new(),
                form: form::Form::Enum(form::Enum {
                    nullable: false,
                    values: enum_.into_iter().collect(),
                }),
                extra: HashMap::new(),
            });
        }

        if let Some(elements) = schema.elements {
            return Ok(Schema {
                definitions: HashMap::new(),
                form: form::Form::Elements(form::Elements {
                    nullable: false,
                    schema: Box::new(Self::from_serde(false, *elements)?),
                }),
                extra: HashMap::new(),
            });
        }

        Ok(Schema {
            definitions: HashMap::new(),
            form: form::Form::Empty,
            extra: HashMap::new(),
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
    use super::Schema;
    use crate::form;
    use crate::serde;
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
}
