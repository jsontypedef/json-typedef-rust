use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    pub definitions: Option<HashMap<String, Schema>>,
    pub nullable: Option<bool>,
    pub type_: Option<String>,
    pub enum_: Option<Vec<String>>,
    pub elements: Option<Box<Schema>>,
    pub properties: Option<HashMap<String, Schema>>,
    pub optional_properties: Option<HashMap<String, Schema>>,
    pub values: Option<Box<Schema>>,
    pub discriminator: Option<Discriminator>,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
pub struct Discriminator {
    pub tag: String,
    pub mapping: HashMap<String, Schema>,
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
                values: Some(Box::new(super::Schema {
                    type_: Some("uint32".to_owned()),
                    ..Default::default()
                })),
                discriminator: Some(super::Discriminator {
                    tag: "foo".to_owned(),
                    mapping: vec![(
                        "foo".to_owned(),
                        super::Schema {
                            type_: Some("uint32".to_owned()),
                            ..Default::default()
                        }
                    )]
                    .into_iter()
                    .collect(),
                }),
            },
            serde_json::from_value(json!({
                "definitions": {
                    "foo": {
                        "type": "uint32",
                    },
                },
                "nullable": true,
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
                "values": {
                    "type": "uint32",
                },
                "discriminator": {
                    "tag": "foo",
                    "mapping": {
                        "foo": {
                            "type": "uint32",
                        },
                    },
                },
            }))
            .unwrap()
        );
    }
}
