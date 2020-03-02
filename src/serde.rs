use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Schema {
    pub definitions: Option<HashMap<String, Schema>>,
    pub nullable: Option<bool>,
    pub ref_: Option<String>,
    pub type_: Option<String>,
    pub enum_: Option<Vec<String>>,
    pub elements: Option<Box<Schema>>,
    pub properties: Option<HashMap<String, Schema>>,
    pub optional_properties: Option<HashMap<String, Schema>>,
    pub additional_properties: Option<bool>,
    pub values: Option<Box<Schema>>,
    pub discriminator: Option<String>,
    pub mapping: Option<HashMap<String, Schema>>,
    pub metadata: Option<HashMap<String, Value>>,
}

#[cfg(feature = "fuzz")]
impl arbitrary::Arbitrary for Schema {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(Schema {
            definitions: arbitrary::Arbitrary::arbitrary(u)?,
            nullable: arbitrary::Arbitrary::arbitrary(u)?,
            ref_: arbitrary::Arbitrary::arbitrary(u)?,
            type_: arbitrary::Arbitrary::arbitrary(u)?,
            enum_: arbitrary::Arbitrary::arbitrary(u)?,
            elements: arbitrary::Arbitrary::arbitrary(u)?,
            properties: arbitrary::Arbitrary::arbitrary(u)?,
            optional_properties: arbitrary::Arbitrary::arbitrary(u)?,
            additional_properties: arbitrary::Arbitrary::arbitrary(u)?,
            values: arbitrary::Arbitrary::arbitrary(u)?,
            discriminator: arbitrary::Arbitrary::arbitrary(u)?,
            mapping: arbitrary::Arbitrary::arbitrary(u)?,

            // serde_json::Value does not derive Arbitrary. That's ok, because
            // for the fuzz tests we're doing, we don't really care about
            // manipulating arbitrary JSON values.
            //
            // So we'll always have metadata be None.
            metadata: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

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
}
