use crate::schema::Schema;
use serde_json::Value;

#[derive(Debug)]
pub struct Validator {
    pub max_depth: Option<usize>,
    pub max_errors: Option<usize>,
}

pub struct ValidationError {
    pub instance_path: Vec<String>,
    pub schema_path: Vec<String>,
}

impl Validator {
    pub fn validate(schema: Schema, instance: Value) -> Vec<ValidationError> {
        vec![]
    }
}
