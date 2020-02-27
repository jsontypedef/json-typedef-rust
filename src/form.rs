use crate::schema::Schema;
use std::collections::HashMap;
use std::collections::HashSet;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
pub struct Ref {
    pub nullable: bool,
    pub definition: String,
}

impl Default for Form {
    fn default() -> Self {
        Form::Empty
    }
}

#[derive(Debug, PartialEq)]
pub struct Type {
    pub nullable: bool,
    pub type_value: TypeValue,
}

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
pub struct Enum {
    pub nullable: bool,
    pub values: HashSet<String>,
}

#[derive(Debug, PartialEq)]
pub struct Elements {
    pub nullable: bool,
    pub schema: Box<Schema>,
}

#[derive(Debug, PartialEq)]
pub struct Properties {
    pub nullable: bool,
    pub required: HashMap<String, Schema>,
    pub optional: HashMap<String, Schema>,
    pub additional: bool,
    pub has_required: bool,
}

#[derive(Debug, PartialEq)]
pub struct Values {
    pub nullable: bool,
    pub schema: Box<Schema>,
}

#[derive(Debug, PartialEq)]
pub struct Discriminator {
    pub nullable: bool,
    pub tag: String,
    pub mapping: HashMap<String, Schema>,
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
