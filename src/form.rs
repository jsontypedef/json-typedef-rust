use crate::schema::Schema;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
pub enum Form {
    Empty,
    Ref(Ref),
    Type(Type),
    Enum(Enum),
    Elements(Elements),
    Properties(Properties),
    Values(Values),
    Discriminator(Discriminator),
}

impl Default for Form {
    fn default() -> Self {
        Form::Empty
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
pub struct Ref {
    pub nullable: bool,
    pub definition: String,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
pub struct Type {
    pub nullable: bool,
    pub type_value: TypeValue,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
pub enum TypeValue {
    Boolean,
    Float32,
    Float64,
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    String,
    Timestamp,
}

impl FromStr for TypeValue {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "boolean" => Ok(Self::Boolean),
            "float32" => Ok(Self::Float32),
            "float64" => Ok(Self::Float64),
            "int8" => Ok(Self::Int8),
            "uint8" => Ok(Self::Uint8),
            "int16" => Ok(Self::Int16),
            "uint16" => Ok(Self::Uint16),
            "int32" => Ok(Self::Int32),
            "uint32" => Ok(Self::Uint32),
            "string" => Ok(Self::String),
            "timestamp" => Ok(Self::Timestamp),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
pub struct Enum {
    pub nullable: bool,
    pub values: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
pub struct Elements {
    pub nullable: bool,
    pub schema: Box<Schema>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
pub struct Properties {
    pub nullable: bool,
    pub required: BTreeMap<String, Schema>,
    pub optional: BTreeMap<String, Schema>,
    pub additional: bool,
    pub has_required: bool,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
pub struct Values {
    pub nullable: bool,
    pub schema: Box<Schema>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
pub struct Discriminator {
    pub nullable: bool,
    pub discriminator: String,
    pub mapping: BTreeMap<String, Schema>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_value_from_str() {
        assert_eq!(Err(()), "Boolean".parse::<TypeValue>());
        assert_eq!(Ok(TypeValue::Boolean), "boolean".parse());
        assert_eq!(Ok(TypeValue::Float32), "float32".parse());
        assert_eq!(Ok(TypeValue::Float64), "float64".parse());
        assert_eq!(Ok(TypeValue::Int8), "int8".parse());
        assert_eq!(Ok(TypeValue::Uint8), "uint8".parse());
        assert_eq!(Ok(TypeValue::Int16), "int16".parse());
        assert_eq!(Ok(TypeValue::Uint16), "uint16".parse());
        assert_eq!(Ok(TypeValue::Int32), "int32".parse());
        assert_eq!(Ok(TypeValue::Uint32), "uint32".parse());
        assert_eq!(Ok(TypeValue::String), "string".parse());
        assert_eq!(Ok(TypeValue::Timestamp), "timestamp".parse());
    }
}
