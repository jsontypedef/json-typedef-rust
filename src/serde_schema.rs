use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct SerdeSchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definitions: Option<BTreeMap<String, SerdeSchema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub elements: Option<Box<SerdeSchema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<BTreeMap<String, SerdeSchema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_properties: Option<BTreeMap<String, SerdeSchema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Box<SerdeSchema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub discriminator: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapping: Option<BTreeMap<String, SerdeSchema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, Value>>,
}
