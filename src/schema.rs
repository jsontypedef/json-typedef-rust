use crate::SerdeSchema;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub type Definitions = BTreeMap<String, Schema>;
pub type Metadata = BTreeMap<String, Value>;

#[derive(Clone, Debug, PartialEq)]
pub enum Schema {
    Empty {
        definitions: Definitions,
        metadata: Metadata,
    },
    Ref {
        definitions: Definitions,
        metadata: Metadata,
        nullable: bool,
        ref_: String,
    },
    Type {
        definitions: Definitions,
        metadata: Metadata,
        nullable: bool,
        type_: Type,
    },
    Enum {
        definitions: Definitions,
        metadata: Metadata,
        nullable: bool,
        enum_: BTreeSet<String>,
    },
    Elements {
        definitions: Definitions,
        metadata: Metadata,
        nullable: bool,
        elements: Box<Schema>,
    },
    Properties {
        definitions: Definitions,
        metadata: Metadata,
        nullable: bool,
        properties: BTreeMap<String, Schema>,
        optional_properties: BTreeMap<String, Schema>,
        properties_is_present: bool,
        additional_properties: bool,
    },
    Values {
        definitions: Definitions,
        metadata: Metadata,
        nullable: bool,
        values: Box<Schema>,
    },
    Discriminator {
        definitions: Definitions,
        metadata: Metadata,
        nullable: bool,
        discriminator: String,
        mapping: BTreeMap<String, Schema>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Boolean,
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
    String,
    Timestamp,
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum FromSerdeSchemaError {
    #[error("invalid combination of keywords in schema")]
    InvalidForm,

    #[error("invalid type: {0:?}")]
    InvalidType(String),

    #[error("duplicated enum value: {0:?}")]
    DuplicatedEnumValue(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SchemaValidateError {
    #[error("no such definition: {0:?}")]
    NoSuchDefinition(String),

    #[error("non-root definitions")]
    NonRootDefinitions,

    #[error("empty enum")]
    EmptyEnum,

    #[error("property repeated in optionalProperties: {0:?}")]
    RepeatedProperty(String),

    #[error("nullable schema in mapping")]
    NullableMapping,

    #[error("non-properties schema in mapping")]
    NonPropertiesMapping,

    #[error("discriminator redefined in mapping: {0:?}")]
    RepeatedDiscriminator(String),
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
    pub fn from_serde_schema(serde_schema: SerdeSchema) -> Result<Self, FromSerdeSchemaError> {
        let mut definitions = BTreeMap::new();
        for (name, sub_schema) in serde_schema.definitions.unwrap_or_default() {
            definitions.insert(name, Self::from_serde_schema(sub_schema)?);
        }

        let metadata = serde_schema.metadata.unwrap_or_default();
        let nullable = serde_schema.nullable.unwrap_or(false);

        // Ensure the schema is using a valid combination of keywords.
        let form_signature = [
            serde_schema.ref_.is_some(),
            serde_schema.type_.is_some(),
            serde_schema.enum_.is_some(),
            serde_schema.elements.is_some(),
            serde_schema.properties.is_some(),
            serde_schema.optional_properties.is_some(),
            serde_schema.additional_properties.is_some(),
            serde_schema.values.is_some(),
            serde_schema.discriminator.is_some(),
            serde_schema.mapping.is_some(),
        ];

        if !VALID_FORM_SIGNATURES.contains(&form_signature) {
            return Err(FromSerdeSchemaError::InvalidForm);
        }

        // From here on out, we can use the presence of certain keywords to
        // determine the form the schema takes on.
        //
        // We'll handle the empty form as a fallback, and handle the other forms
        // in standard order.
        if let Some(ref_) = serde_schema.ref_ {
            return Ok(Schema::Ref {
                definitions,
                metadata,
                nullable,
                ref_,
            });
        }

        if let Some(type_) = serde_schema.type_ {
            let type_ = match &type_[..] {
                "boolean" => Type::Boolean,
                "int8" => Type::Int8,
                "uint8" => Type::Uint8,
                "int16" => Type::Int16,
                "uint16" => Type::Uint16,
                "int32" => Type::Int32,
                "uint32" => Type::Uint32,
                "float32" => Type::Float32,
                "float64" => Type::Float64,
                "string" => Type::String,
                "timestamp" => Type::Timestamp,
                _ => return Err(FromSerdeSchemaError::InvalidType(type_)),
            };

            return Ok(Schema::Type {
                definitions,
                metadata,
                nullable,
                type_,
            });
        }

        if let Some(enum_) = serde_schema.enum_ {
            // We do this construction by hand, rather than using collect, to
            // detect the case of an enum value being repeated. This can't be
            // detected once the values are put in the set.
            let mut values = BTreeSet::new();
            for value in enum_ {
                if values.contains(&value) {
                    return Err(FromSerdeSchemaError::DuplicatedEnumValue(value));
                }

                values.insert(value);
            }

            return Ok(Schema::Enum {
                definitions,
                metadata,
                nullable,
                enum_: values,
            });
        }

        if let Some(elements) = serde_schema.elements {
            return Ok(Schema::Elements {
                definitions,
                metadata,
                nullable,
                elements: Box::new(Self::from_serde_schema(*elements)?),
            });
        }

        if serde_schema.properties.is_some() || serde_schema.optional_properties.is_some() {
            let properties_is_present = serde_schema.properties.is_some();
            let additional_properties = serde_schema.additional_properties.unwrap_or(false);

            let mut properties = BTreeMap::new();
            for (name, sub_schema) in serde_schema.properties.unwrap_or_default() {
                properties.insert(name, Self::from_serde_schema(sub_schema)?);
            }

            let mut optional_properties = BTreeMap::new();
            for (name, sub_schema) in serde_schema.optional_properties.unwrap_or_default() {
                optional_properties.insert(name, Self::from_serde_schema(sub_schema)?);
            }

            return Ok(Schema::Properties {
                definitions,
                metadata,
                nullable,
                properties,
                optional_properties,
                properties_is_present,
                additional_properties,
            });
        }

        if let Some(values) = serde_schema.values {
            return Ok(Schema::Values {
                definitions,
                metadata,
                nullable,
                values: Box::new(Self::from_serde_schema(*values)?),
            });
        }

        if let Some(discriminator) = serde_schema.discriminator {
            // This is safe because the form signature check ensures mapping is
            // present if discriminator is present.
            let mut mapping = BTreeMap::new();
            for (name, sub_schema) in serde_schema.mapping.unwrap() {
                mapping.insert(name, Self::from_serde_schema(sub_schema)?);
            }

            return Ok(Schema::Discriminator {
                definitions,
                metadata,
                nullable,
                discriminator,
                mapping,
            });
        }

        Ok(Schema::Empty {
            definitions,
            metadata,
        })
    }

    pub fn validate(&self) -> Result<(), SchemaValidateError> {
        self._validate(None)
    }

    fn _validate(&self, root: Option<&Self>) -> Result<(), SchemaValidateError> {
        let sub_root = root.or(Some(self));

        if root.is_some() && !self.definitions().is_empty() {
            return Err(SchemaValidateError::NonRootDefinitions);
        }

        for sub_schema in self.definitions().values() {
            sub_schema._validate(sub_root)?;
        }

        match self {
            Self::Empty { .. } => {}
            Self::Ref { ref_, .. } => {
                if !sub_root
                    .map(|r| r.definitions())
                    .unwrap()
                    .contains_key(ref_)
                {
                    return Err(SchemaValidateError::NoSuchDefinition(ref_.clone()));
                }
            }
            Self::Type { .. } => {}
            Self::Enum { enum_, .. } => {
                if enum_.is_empty() {
                    return Err(SchemaValidateError::EmptyEnum);
                }
            }
            Self::Elements { elements, .. } => {
                elements._validate(sub_root)?;
            }
            Self::Properties {
                properties,
                optional_properties,
                ..
            } => {
                for key in properties.keys() {
                    if optional_properties.contains_key(key) {
                        return Err(SchemaValidateError::RepeatedProperty(key.clone()));
                    }
                }

                for sub_schema in properties.values() {
                    sub_schema._validate(sub_root)?;
                }

                for sub_schema in optional_properties.values() {
                    sub_schema._validate(sub_root)?;
                }
            }
            Self::Values { values, .. } => {
                values._validate(sub_root)?;
            }
            Self::Discriminator {
                discriminator,
                mapping,
                ..
            } => {
                for sub_schema in mapping.values() {
                    if let Self::Properties {
                        nullable,
                        properties,
                        optional_properties,
                        ..
                    } = sub_schema
                    {
                        if *nullable {
                            return Err(SchemaValidateError::NullableMapping);
                        }

                        if properties.contains_key(discriminator)
                            || optional_properties.contains_key(discriminator)
                        {
                            return Err(SchemaValidateError::RepeatedDiscriminator(
                                discriminator.clone(),
                            ));
                        }
                    } else {
                        return Err(SchemaValidateError::NonPropertiesMapping);
                    }

                    sub_schema._validate(sub_root)?;
                }
            }
        }

        Ok(())
    }

    pub fn definitions(&self) -> &BTreeMap<String, Schema> {
        match self {
            Self::Empty { definitions, .. } => definitions,
            Self::Ref { definitions, .. } => definitions,
            Self::Enum { definitions, .. } => definitions,
            Self::Type { definitions, .. } => definitions,
            Self::Elements { definitions, .. } => definitions,
            Self::Properties { definitions, .. } => definitions,
            Self::Values { definitions, .. } => definitions,
            Self::Discriminator { definitions, .. } => definitions,
        }
    }

    pub fn metadata(&self) -> &BTreeMap<String, Value> {
        match self {
            Self::Empty { metadata, .. } => metadata,
            Self::Ref { metadata, .. } => metadata,
            Self::Enum { metadata, .. } => metadata,
            Self::Type { metadata, .. } => metadata,
            Self::Elements { metadata, .. } => metadata,
            Self::Properties { metadata, .. } => metadata,
            Self::Values { metadata, .. } => metadata,
            Self::Discriminator { metadata, .. } => metadata,
        }
    }

    pub fn nullable(&self) -> bool {
        match self {
            Self::Empty { .. } => true,
            Self::Ref { nullable, .. } => *nullable,
            Self::Enum { nullable, .. } => *nullable,
            Self::Type { nullable, .. } => *nullable,
            Self::Elements { nullable, .. } => *nullable,
            Self::Properties { nullable, .. } => *nullable,
            Self::Values { nullable, .. } => *nullable,
            Self::Discriminator { nullable, .. } => *nullable,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Schema, SerdeSchema};

    #[test]
    fn invalid_schemas() {
        use std::collections::BTreeMap;

        let test_cases: BTreeMap<String, serde_json::Value> = serde_json::from_str(include_str!(
            "../json-typedef-spec/tests/invalid_schemas.json"
        ))
        .expect("parse invalid_schemas.json");

        for (test_case_name, test_case) in test_cases {
            if let Ok(serde_schema) = serde_json::from_value::<SerdeSchema>(test_case) {
                if let Ok(schema) = Schema::from_serde_schema(serde_schema) {
                    if schema.validate().is_ok() {
                        panic!(
                            "failed to detect invalid schema: {}, got: {:?}",
                            test_case_name, schema
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn valid_schemas() {
        use std::collections::BTreeMap;

        #[derive(serde::Deserialize)]
        struct TestCase {
            schema: serde_json::Value,
        }

        let test_cases: BTreeMap<String, TestCase> =
            serde_json::from_str(include_str!("../json-typedef-spec/tests/validation.json"))
                .expect("parse validation.json");

        for (test_case_name, test_case) in test_cases {
            let serde_schema =
                serde_json::from_value::<SerdeSchema>(test_case.schema).expect(&test_case_name);
            let schema = Schema::from_serde_schema(serde_schema).expect(&test_case_name);
            schema.validate().expect(&test_case_name);
        }
    }
}
