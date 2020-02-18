use crate::schema::Schema;
use std::collections::HashMap;
use std::collections::HashSet;

pub enum Form {
    Empty(Empty),
    Type(Type),
    Enum(Enum),
    Elements(Elements),
    Properties(Properties),
    Values(Values),
    Discriminator(Discriminator),
}

pub struct Empty {}

pub struct Type {
    pub nullable: bool,
    pub type_value: TypeValue,
}

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

pub struct Enum {
    pub nullable: bool,
    pub values: HashSet<String>,
}

pub struct Elements {
    pub nullable: bool,
    pub schema: Box<Schema>,
}

pub struct Properties {
    pub nullable: bool,
    pub required: HashMap<String, Schema>,
    pub optional: HashMap<String, Schema>,
    pub additional: bool,
    pub has_required: bool,
}

pub struct Values {
    pub nullable: bool,
    pub schema: Box<Schema>,
}

pub struct Discriminator {
    pub nullable: bool,
    pub tag: String,
    pub mapping: HashMap<String, Schema>,
}
