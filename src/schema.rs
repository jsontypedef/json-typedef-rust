use crate::SerdeSchema;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// A convenience alias for the JSON Typedef `definitions` keyword value.
pub type Definitions = BTreeMap<String, Schema>;

/// A convenience alias for the JSON Typedef `metadata` keyword value.
pub type Metadata = BTreeMap<String, Value>;

/// A pattern-matching-friendly representation of a JSON Typedef schema.
///
/// Each variant of this schema corresponds to one of the eight "forms" a schema
/// may take on. All of the forms share the following fields:
///
/// * `definitions` corresponds to the JSON Typedef keyword of the same name.
///    This should only be non-empty on root schemas. Otherwise,
///    [`Schema::validate`] will return
///    [`SchemaValidateError::NonRootDefinitions`].
///
/// * `metadata` corresponds to the JSON Typedef keyword of the same name. Use
///   this to convey information not pertinent to validation, such as hints for
///   code generation. Do not expect other parties to understand the fields
///   inside metadata unless you've agreed upon them out-of-band.
///
/// Except for [`Schema::Empty`], all of the forms also share one additional
/// field:
///
/// * `nullable` corresponds to the JSON Typedef keyword of the same name. If
///   set to "true", then regardless of any other considerations the schema will
///   accept JSON `null` as valid.
///
/// [`Schema::Empty`] omits `nullable` because it's redundant; schemas of the
/// empty form already accept `null` anyway.
///
/// For convenience, these three common properties have associated borrowing
/// "getters": [`Schema::definitions`], [`Schema::metadata`], and
/// [`Schema::nullable`].
///
/// If you are trying to parse a JSON Typedef schema from JSON, see
/// [`SerdeSchema`] and [`Schema::from_serde_schema`].
///
/// ```
/// use jtd::{SerdeSchema, Schema};
/// use serde_json::json;
///
/// assert_eq!(
///     Schema::from_serde_schema(serde_json::from_value(json!({
///         "elements": {
///             "type": "uint32",
///             "nullable": true
///         }
///     })).unwrap()).unwrap(),
///     jtd::Schema::Elements {
///         definitions: Default::default(),
///         metadata: Default::default(),
///         nullable: false,
///         elements: Box::new(jtd::Schema::Type {
///             definitions: Default::default(),
///             metadata: Default::default(),
///             nullable: true,
///             type_: jtd::Type::Uint32,
///         })
///     }
/// );
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum Schema {
    /// The [empty](https://tools.ietf.org/html/rfc8927#section-2.2.1) form.
    ///
    /// The empty form will accept all inputs. It corresponds to the "top" type
    /// of many programming language, like Java's `Object` or TypeScript's
    /// `any`.
    Empty {
        definitions: Definitions,
        metadata: Metadata,
    },

    /// The [ref](https://tools.ietf.org/html/rfc8927#section-2.2.2) form.
    ///
    /// The ref form accepts whatever the definition it refers to accepts.
    Ref {
        definitions: Definitions,
        metadata: Metadata,
        nullable: bool,

        /// The name of the definition being referred to.
        ref_: String,
    },

    /// The [type](https://tools.ietf.org/html/rfc8927#section-2.2.3) form.
    ///
    /// The type form accepts JSON "primitives" (booleans, numbers, strings)
    /// whose value fall within a certain "type". These types are enumerated in
    /// [`Type`].
    Type {
        definitions: Definitions,
        metadata: Metadata,
        nullable: bool,

        /// The type of primitive value accepted.
        type_: Type,
    },

    /// The [enum](https://tools.ietf.org/html/rfc8927#section-2.2.4) form.
    ///
    /// The enum form accepts JSON strings whose values are within an enumerated
    /// set.
    Enum {
        definitions: Definitions,
        metadata: Metadata,
        nullable: bool,

        /// The values the schema accepts.
        enum_: BTreeSet<String>,
    },

    /// The [elements](https://tools.ietf.org/html/rfc8927#section-2.2.5) form.
    ///
    /// The elements form accepts JSON arrays, and each element of the array is
    /// validated against a sub-schema.
    Elements {
        definitions: Definitions,
        metadata: Metadata,
        nullable: bool,

        /// A schema for the elements of the array.
        elements: Box<Schema>,
    },

    /// The [properties](https://tools.ietf.org/html/rfc8927#section-2.2.6)
    /// form.
    ///
    /// The properties form accepts JSON objects being used as "structs".
    Properties {
        definitions: Definitions,
        metadata: Metadata,
        nullable: bool,

        /// The required properties of the "struct", and the schema that each
        /// must satisfy.
        properties: BTreeMap<String, Schema>,

        /// The optional properties of the "struct", and the schema that each
        /// must satisfy if present.
        optional_properties: BTreeMap<String, Schema>,

        /// Whether the `properties` keyword is present on the schema.
        ///
        /// It is invalid to set this to `false` while having `properties` be
        /// non-empty.
        ///
        /// This is used only to handle the corner case of a properties-form
        /// schema being used to validate a non-object; in order to ensure the
        /// returned `schema_path` points to a part of the schema that really
        /// exists, validators need to be able to tell the difference between
        /// `properties` being an empty object versus being omitted from the
        /// schema.
        ///
        /// This field does not affect whether an input is valid. It only
        /// affects the `schema_path` that will be returned if that input is not
        /// an object. For more details, see the first sub-bullet after
        /// "Otherwise" in [RFC 8927, Section
        /// 3.3.6](https://tools.ietf.org/html/rfc8927#section-3.3.6).
        ///
        /// [`Schema::from_serde_schema`] correctly handles populating this
        /// field. If you are constructing schemas by hand and want to play it
        /// safe, it is always safe to set this to `true`.
        properties_is_present: bool,

        /// Whether additional properties not specified in `properties` or
        /// `optional_properties` are permitted.
        additional_properties: bool,
    },

    /// The [values](https://tools.ietf.org/html/rfc8927#section-2.2.7) form.
    ///
    /// The values form accepts JSON objects being used as "dictionaries"; each
    /// value of the dictionary is validated against a sub-schema.
    Values {
        definitions: Definitions,
        metadata: Metadata,
        nullable: bool,

        /// A schema for the values of the "dictionary" object.
        values: Box<Schema>,
    },

    /// The [discriminator](https://tools.ietf.org/html/rfc8927#section-2.2.8)
    /// form.
    ///
    /// The discriminator form accepts JSON objects being used as "discriminated
    /// unions", or "tagged unions".
    Discriminator {
        definitions: Definitions,
        metadata: Metadata,
        nullable: bool,

        /// The "discriminator" property of the schema.
        ///
        /// For an input to be valid, this property must exist and its value
        /// must be a key in `mapping`.
        discriminator: String,

        /// A mapping from the value of the `discriminator` property in the
        /// input to a schema that the rest of the input (without the
        /// `discriminator` property) must satisfy.
        mapping: BTreeMap<String, Schema>,
    },
}

/// The values [`Schema::Type::type_`] may take on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    /// Either JSON `true` or `false`.
    Boolean,

    /// A JSON number with zero fractional part within the range of [`i8`].
    Int8,

    /// A JSON number with zero fractional part within the range of [`u8`].
    Uint8,

    /// A JSON number with zero fractional part within the range of [`i16`].
    Int16,

    /// A JSON number with zero fractional part within the range of [`u16`].
    Uint16,

    /// A JSON number with zero fractional part within the range of [`i32`].
    Int32,

    /// A JSON number with zero fractional part within the range of [`u32`].
    Uint32,

    /// A JSON number. Code generators will treat this like a Rust [`f32`].
    Float32,

    /// A JSON number. Code generators will treat this like a Rust [`f64`].
    Float64,

    /// A JSON string.
    String,

    /// A JSON string encoding a [RFC3339](https://tools.ietf.org/html/rfc3339)
    /// timestamp.
    Timestamp,
}

/// Errors that may arise from [`Schema::from_serde_schema`].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum FromSerdeSchemaError {
    /// Indicates the schema uses an invalid combination of keywords.
    ///
    /// ```
    /// use jtd::{FromSerdeSchemaError, Schema, SerdeSchema};
    ///
    /// assert_eq!(
    ///     Err(FromSerdeSchemaError::InvalidForm),
    ///
    ///     // it's invalid to have both "type" and "enum" on a schema
    ///     Schema::from_serde_schema(SerdeSchema {
    ///         type_: Some("uint8".to_owned()),
    ///         enum_: Some(Default::default()),
    ///         ..Default::default()
    ///     })
    /// )
    /// ```
    #[error("invalid combination of keywords in schema")]
    InvalidForm,

    /// Indicates the schema uses a value for `type` that isn't in [`Type`].
    ///
    /// ```
    /// use jtd::{FromSerdeSchemaError, Schema, SerdeSchema};
    ///
    /// assert_eq!(
    ///     Err(FromSerdeSchemaError::InvalidType("uint64".to_owned())),
    ///
    ///     // there is no uint64 in JSON Typedef
    ///     Schema::from_serde_schema(SerdeSchema {
    ///         type_: Some("uint64".to_owned()),
    ///         ..Default::default()
    ///     })
    /// )
    /// ```
    #[error("invalid type: {0:?}")]
    InvalidType(String),

    /// Indicates the schema has the same value appearing twice in an `enum`.
    ///
    /// ```
    /// use jtd::{FromSerdeSchemaError, Schema, SerdeSchema};
    ///
    /// assert_eq!(
    ///     Err(FromSerdeSchemaError::DuplicatedEnumValue("foo".to_owned())),
    ///
    ///     // it's invalid to have the same value appear twice in an enum array
    ///     Schema::from_serde_schema(SerdeSchema {
    ///         enum_: Some(vec!["foo".into(), "bar".into(), "foo".into()]),
    ///         ..Default::default()
    ///     })
    /// )
    /// ```
    #[error("duplicated enum value: {0:?}")]
    DuplicatedEnumValue(String),
}

/// Errors that may arise from [`Schema::validate`].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SchemaValidateError {
    /// Indicates the schema has a `ref` to a definition that doesn't exist.
    ///
    /// ```
    /// use jtd::{Schema, SchemaValidateError};
    ///
    /// assert_eq!(
    ///     Err(SchemaValidateError::NoSuchDefinition("foo".into())),
    ///
    ///     // a "ref" without definitions is always invalid
    ///     Schema::Ref {
    ///         definitions: Default::default(),
    ///         metadata: Default::default(),
    ///         nullable: Default::default(),
    ///         ref_: "foo".into(),
    ///     }.validate(),
    /// )
    /// ```
    #[error("no such definition: {0:?}")]
    NoSuchDefinition(String),

    /// Indicates the schema has non-empty `definitions` below the root level.
    ///
    /// ```
    /// use jtd::{Schema, SchemaValidateError};
    ///
    /// assert_eq!(
    ///     Err(SchemaValidateError::NonRootDefinitions),
    ///
    ///     // definitions can only be present at the root level
    ///     Schema::Elements {
    ///         definitions: Default::default(),
    ///         metadata: Default::default(),
    ///         nullable: Default::default(),
    ///         elements: Box::new(Schema::Empty {
    ///             definitions: vec![(
    ///                 "foo".to_owned(),
    ///                 Schema::Empty {
    ///                     definitions: Default::default(),
    ///                     metadata: Default::default(),
    ///                 }
    ///             )].into_iter().collect(),
    ///             metadata: Default::default(),
    ///         }),
    ///     }.validate(),
    /// )
    /// ```
    #[error("non-root definitions")]
    NonRootDefinitions,

    /// Indicates the schema has an `enum` with no values in it.
    ///
    /// ```
    /// use jtd::{Schema, SchemaValidateError};
    ///
    /// assert_eq!(
    ///     Err(SchemaValidateError::EmptyEnum),
    ///
    ///     // empty enums are illegal
    ///     Schema::Enum {
    ///         definitions: Default::default(),
    ///         metadata: Default::default(),
    ///         nullable: Default::default(),
    ///         enum_: Default::default(),
    ///     }.validate(),
    /// )
    /// ```
    #[error("empty enum")]
    EmptyEnum,

    /// Indicates the schema has the same property appear in `properties` and
    /// `optional_properties`.
    ///
    /// ```
    /// use jtd::{Schema, SchemaValidateError};
    ///
    /// assert_eq!(
    ///     Err(SchemaValidateError::RepeatedProperty("foo".into())),
    ///
    ///     // properties and optional_properties must not overlap
    ///     Schema::Properties {
    ///         definitions: Default::default(),
    ///         metadata: Default::default(),
    ///         nullable: Default::default(),
    ///         properties: vec![(
    ///             "foo".to_owned(),
    ///             Schema::Empty {
    ///                 definitions: Default::default(),
    ///                 metadata: Default::default(),
    ///             },
    ///         )].into_iter().collect(),
    ///         optional_properties: vec![(
    ///             "foo".to_owned(),
    ///             Schema::Empty {
    ///                 definitions: Default::default(),
    ///                 metadata: Default::default(),
    ///             },
    ///         )].into_iter().collect(),
    ///         properties_is_present: true,
    ///         additional_properties: false,
    ///     }.validate(),
    /// )
    /// ```
    #[error("property repeated in optionalProperties: {0:?}")]
    RepeatedProperty(String),

    /// Indicates the schema has a value in `mapping` with `nullable` set to
    /// `true`.
    ///
    /// ```
    /// use jtd::{Schema, SchemaValidateError};
    ///
    /// assert_eq!(
    ///     Err(SchemaValidateError::NullableMapping),
    ///
    ///     // mappings must not be nullable
    ///     Schema::Discriminator {
    ///         definitions: Default::default(),
    ///         metadata: Default::default(),
    ///         nullable: Default::default(),
    ///         discriminator: "foo".into(),
    ///         mapping: vec![(
    ///             "bar".to_owned(),
    ///             Schema::Properties {
    ///                 definitions: Default::default(),
    ///                 metadata: Default::default(),
    ///                 nullable: true,
    ///                 properties: Default::default(),
    ///                 optional_properties: Default::default(),
    ///                 properties_is_present: true,
    ///                 additional_properties: false,
    ///             }
    ///         )].into_iter().collect(),
    ///     }.validate(),
    /// );
    /// ```
    #[error("nullable schema in mapping")]
    NullableMapping,

    /// Indicates the schema has a value in `mapping` that isn't a
    /// [`Schema::Properties`].
    ///
    /// ```
    /// use jtd::{Schema, SchemaValidateError};
    ///
    /// assert_eq!(
    ///     Err(SchemaValidateError::NonPropertiesMapping),
    ///
    ///     // mappings must be of the properties form
    ///     Schema::Discriminator {
    ///         definitions: Default::default(),
    ///         metadata: Default::default(),
    ///         nullable: Default::default(),
    ///         discriminator: "foo".into(),
    ///         mapping: vec![(
    ///             "bar".to_owned(),
    ///             Schema::Empty {
    ///                 definitions: Default::default(),
    ///                 metadata: Default::default(),
    ///             }
    ///         )].into_iter().collect(),
    ///     }.validate(),
    /// );
    /// ```
    #[error("non-properties schema in mapping")]
    NonPropertiesMapping,

    /// Indicates the schema has a value in `mapping` whose `properties` or
    /// `optional_properties` contains `discriminator`.
    ///
    /// ```
    /// use jtd::{Schema, SchemaValidateError};
    ///
    /// assert_eq!(
    ///     Err(SchemaValidateError::RepeatedDiscriminator("foo".into())),
    ///
    ///     // mappings must not re-define the discriminator property
    ///     Schema::Discriminator {
    ///         definitions: Default::default(),
    ///         metadata: Default::default(),
    ///         nullable: Default::default(),
    ///         discriminator: "foo".into(),
    ///         mapping: vec![(
    ///             "bar".to_owned(),
    ///             Schema::Properties {
    ///                 definitions: Default::default(),
    ///                 metadata: Default::default(),
    ///                 nullable: Default::default(),
    ///                 properties: vec![(
    ///                     "foo".into(),
    ///                     Schema::Empty {
    ///                         definitions: Default::default(),
    ///                         metadata: Default::default(),
    ///                     }
    ///                 )].into_iter().collect(),
    ///                 optional_properties: Default::default(),
    ///                 properties_is_present: true,
    ///                 additional_properties: false,
    ///             }
    ///         )].into_iter().collect(),
    ///     }.validate(),
    /// );
    /// ```
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
    /// Converts a [`Schema`] into a [`SerdeSchema`].
    ///
    /// ```
    /// use jtd::{Schema, SerdeSchema, Type};
    ///
    /// assert_eq!(
    ///     SerdeSchema {
    ///         type_: Some("uint8".to_owned()),
    ///         ..Default::default()
    ///     },
    ///     Schema::Type {
    ///         definitions: Default::default(),
    ///         metadata: Default::default(),
    ///         nullable: false,
    ///         type_: Type::Uint8,
    ///     }.into_serde_schema(),
    /// );
    /// ```
    pub fn into_serde_schema(self) -> SerdeSchema {
        let mut serde_schema: SerdeSchema = Default::default();

        match self {
            Schema::Empty {
                definitions,
                metadata,
            } => {
                serde_schema.definitions = Self::definitions_into_serde_schema(definitions);
                serde_schema.metadata = Self::metadata_into_serde_schema(metadata);
            }

            Schema::Ref {
                definitions,
                metadata,
                nullable,
                ref_,
            } => {
                serde_schema.definitions = Self::definitions_into_serde_schema(definitions);
                serde_schema.metadata = Self::metadata_into_serde_schema(metadata);
                serde_schema.nullable = Self::nullable_into_serde_schema(nullable);
                serde_schema.ref_ = Some(ref_);
            }

            Schema::Type {
                definitions,
                metadata,
                nullable,
                type_,
            } => {
                serde_schema.definitions = Self::definitions_into_serde_schema(definitions);
                serde_schema.metadata = Self::metadata_into_serde_schema(metadata);
                serde_schema.nullable = Self::nullable_into_serde_schema(nullable);
                serde_schema.type_ = Some(
                    match type_ {
                        Type::Boolean => "boolean",
                        Type::Int8 => "int8",
                        Type::Uint8 => "uint8",
                        Type::Int16 => "int16",
                        Type::Uint16 => "uint16",
                        Type::Int32 => "int32",
                        Type::Uint32 => "uint32",
                        Type::Float32 => "float32",
                        Type::Float64 => "float64",
                        Type::String => "string",
                        Type::Timestamp => "timestamp",
                    }
                    .to_owned(),
                );
            }

            Schema::Enum {
                definitions,
                metadata,
                nullable,
                enum_,
            } => {
                serde_schema.definitions = Self::definitions_into_serde_schema(definitions);
                serde_schema.metadata = Self::metadata_into_serde_schema(metadata);
                serde_schema.nullable = Self::nullable_into_serde_schema(nullable);
                serde_schema.enum_ = Some(enum_.into_iter().collect());
            }

            Schema::Elements {
                definitions,
                metadata,
                nullable,
                elements,
            } => {
                serde_schema.definitions = Self::definitions_into_serde_schema(definitions);
                serde_schema.metadata = Self::metadata_into_serde_schema(metadata);
                serde_schema.nullable = Self::nullable_into_serde_schema(nullable);
                serde_schema.elements = Some(Box::new(elements.into_serde_schema()));
            }

            Schema::Properties {
                definitions,
                metadata,
                nullable,
                properties,
                optional_properties,
                properties_is_present,
                additional_properties,
            } => {
                serde_schema.definitions = Self::definitions_into_serde_schema(definitions);
                serde_schema.metadata = Self::metadata_into_serde_schema(metadata);
                serde_schema.nullable = Self::nullable_into_serde_schema(nullable);

                if properties_is_present {
                    serde_schema.properties = Some(
                        properties
                            .into_iter()
                            .map(|(k, v)| (k, v.into_serde_schema()))
                            .collect(),
                    );
                }

                if !optional_properties.is_empty() {
                    serde_schema.optional_properties = Some(
                        optional_properties
                            .into_iter()
                            .map(|(k, v)| (k, v.into_serde_schema()))
                            .collect(),
                    );
                }

                if additional_properties {
                    serde_schema.additional_properties = Some(additional_properties);
                }
            }

            Schema::Values {
                definitions,
                metadata,
                nullable,
                values,
            } => {
                serde_schema.definitions = Self::definitions_into_serde_schema(definitions);
                serde_schema.metadata = Self::metadata_into_serde_schema(metadata);
                serde_schema.nullable = Self::nullable_into_serde_schema(nullable);
                serde_schema.values = Some(Box::new(values.into_serde_schema()));
            }

            Schema::Discriminator {
                definitions,
                metadata,
                nullable,
                discriminator,
                mapping,
            } => {
                serde_schema.definitions = Self::definitions_into_serde_schema(definitions);
                serde_schema.metadata = Self::metadata_into_serde_schema(metadata);
                serde_schema.nullable = Self::nullable_into_serde_schema(nullable);
                serde_schema.discriminator = Some(discriminator);
                serde_schema.mapping = Some(
                    mapping
                        .into_iter()
                        .map(|(k, v)| (k, v.into_serde_schema()))
                        .collect(),
                );
            }
        }

        serde_schema
    }

    fn definitions_into_serde_schema(
        definitions: Definitions,
    ) -> Option<BTreeMap<String, SerdeSchema>> {
        if definitions.is_empty() {
            None
        } else {
            Some(
                definitions
                    .into_iter()
                    .map(|(k, v)| (k, v.into_serde_schema()))
                    .collect(),
            )
        }
    }

    fn metadata_into_serde_schema(metadata: Metadata) -> Option<BTreeMap<String, Value>> {
        if metadata.is_empty() {
            None
        } else {
            Some(metadata)
        }
    }

    fn nullable_into_serde_schema(nullable: bool) -> Option<bool> {
        if nullable {
            Some(true)
        } else {
            None
        }
    }

    /// Constructs a [`Schema`] from a [`SerdeSchema`].
    ///
    /// ```
    /// use jtd::{Schema, SerdeSchema, Type};
    ///
    /// assert_eq!(
    ///     Schema::Type {
    ///         definitions: Default::default(),
    ///         metadata: Default::default(),
    ///         nullable: false,
    ///         type_: Type::Uint8,
    ///     },
    ///     Schema::from_serde_schema(SerdeSchema {
    ///         type_: Some("uint8".to_owned()),
    ///         ..Default::default()
    ///     }).unwrap(),
    /// );
    /// ```
    ///
    /// See the documentation for [`FromSerdeSchemaError`] for examples of how
    /// this function may return an error.
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

    /// Ensures a [`Schema`] is well-formed.
    ///
    /// ```
    /// use jtd::{Schema, Type};
    ///
    /// let schema = Schema::Type {
    ///     definitions: Default::default(),
    ///     metadata: Default::default(),
    ///     nullable: false,
    ///     type_: Type::Uint8,
    /// };
    ///
    /// schema.validate().expect("Invalid schema");
    /// ```
    ///
    /// See the documentation for [`SchemaValidateError`] for examples of how
    /// this function may return an error.
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

    /// Gets the schema's definitions.
    ///
    /// ```
    /// use jtd::{Definitions, Schema};
    ///
    /// assert_eq!(
    ///     &vec![(
    ///         "foo".to_owned(),
    ///         Schema::Empty {
    ///             definitions: Default::default(),
    ///             metadata: Default::default(),
    ///         },
    ///     )].into_iter().collect::<Definitions>(),
    ///
    ///      Schema::Empty {
    ///          definitions: vec![(
    ///             "foo".to_owned(),
    ///             Schema::Empty {
    ///                 definitions: Default::default(),
    ///                 metadata: Default::default(),
    ///             },
    ///         )].into_iter().collect(),
    ///          metadata: Default::default(),
    ///      }.definitions(),
    /// );
    /// ```
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

    /// Gets the schema's metadata.
    ///
    /// ```
    /// use jtd::{Metadata, Schema};
    /// use serde_json::json;
    ///
    /// assert_eq!(
    ///     &vec![(
    ///         "foo".to_owned(),
    ///         json!("bar"),
    ///     )].into_iter().collect::<Metadata>(),
    ///
    ///     Schema::Empty {
    ///         definitions: Default::default(),
    ///         metadata: vec![(
    ///            "foo".to_owned(),
    ///            json!("bar"),
    ///        )].into_iter().collect(),
    ///     }.metadata(),
    /// );
    /// ```
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

    /// Gets whether the schema is nullable.
    ///
    /// For [`Schema::Empty`], this always returns true. For all other forms,
    /// this fetches the `nullable` property.
    ///
    /// ```
    /// use jtd::{Schema, Type};
    ///
    /// assert!(
    ///     Schema::Empty {
    ///         definitions: Default::default(),
    ///         metadata: Default::default(),
    ///     }.nullable(),
    /// );
    ///
    /// assert!(
    ///     !Schema::Type {
    ///         definitions: Default::default(),
    ///         metadata: Default::default(),
    ///         nullable: false,
    ///         type_: Type::Uint8,
    ///     }.nullable(),
    /// );
    /// ```
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
