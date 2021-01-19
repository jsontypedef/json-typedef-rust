use crate::{Schema, Type};
use chrono::DateTime;
use serde_json::Value;
use thiserror::Error;

#[derive(Default)]
pub struct ValidateOptions {
    max_depth: usize,
    max_errors: usize,
}

impl ValidateOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub fn with_max_errors(mut self, max_errors: usize) -> Self {
        self.max_errors = max_errors;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ValidateError {
    #[error("max depth exceeded")]
    MaxDepthExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationErrorIndicator {
    pub instance_path: Vec<String>,
    pub schema_path: Vec<String>,
}

pub fn validate(
    schema: &Schema,
    instance: &Value,
    options: ValidateOptions,
) -> Result<Vec<ValidationErrorIndicator>, ValidateError> {
    let mut vm = Vm {
        max_depth: options.max_depth,
        max_errors: options.max_errors,
        instance_tokens: vec![],
        schema_tokens: vec![vec![]],
        errors: vec![],
    };

    match vm.validate(schema, schema, None, instance) {
        Ok(()) | Err(VmValidateError::MaxErrorsReached) => Ok(vm.errors),
        Err(VmValidateError::MaxDepthExceeded) => Err(ValidateError::MaxDepthExceeded),
    }
}

struct Vm {
    pub max_depth: usize,
    pub max_errors: usize,
    pub instance_tokens: Vec<String>,
    pub schema_tokens: Vec<Vec<String>>,
    pub errors: Vec<ValidationErrorIndicator>,
}

enum VmValidateError {
    MaxErrorsReached,
    MaxDepthExceeded,
}

impl Vm {
    pub fn validate(
        &mut self,
        root: &Schema,
        schema: &Schema,
        parent_tag: Option<&str>,
        instance: &Value,
    ) -> Result<(), VmValidateError> {
        if instance.is_null() && schema.nullable() {
            return Ok(());
        }

        match schema {
            Schema::Empty { .. } => {}
            Schema::Ref { ref_, .. } => {
                self.schema_tokens
                    .push(vec!["definitions".to_owned(), ref_.clone()]);
                if self.schema_tokens.len() == self.max_depth {
                    return Err(VmValidateError::MaxDepthExceeded);
                }

                self.validate(root, &root.definitions()[ref_], None, instance)?;
                self.schema_tokens.pop();
            }
            Schema::Type { type_, .. } => {
                self.push_schema_token("type");

                match type_ {
                    Type::Boolean => {
                        if !instance.is_boolean() {
                            self.push_error()?;
                        }
                    }
                    Type::Float32 | Type::Float64 => {
                        if !instance.is_f64() && !instance.is_i64() {
                            self.push_error()?;
                        }
                    }
                    Type::Int8 => self.validate_int(instance, -128.0, 127.0)?,
                    Type::Uint8 => self.validate_int(instance, 0.0, 255.0)?,
                    Type::Int16 => self.validate_int(instance, -32768.0, 32767.0)?,
                    Type::Uint16 => self.validate_int(instance, 0.0, 65535.0)?,
                    Type::Int32 => self.validate_int(instance, -2147483648.0, 2147483647.0)?,
                    Type::Uint32 => self.validate_int(instance, 0.0, 4294967295.0)?,
                    Type::String => {
                        if !instance.is_string() {
                            self.push_error()?;
                        }
                    }
                    Type::Timestamp => {
                        if let Some(s) = instance.as_str() {
                            if DateTime::parse_from_rfc3339(s).is_err() {
                                self.push_error()?;
                            }
                        } else {
                            self.push_error()?;
                        }
                    }
                };

                self.pop_schema_token();
            }
            Schema::Enum { enum_, .. } => {
                self.push_schema_token("enum");
                if let Some(s) = instance.as_str() {
                    if !enum_.contains(s) {
                        self.push_error()?;
                    }
                } else {
                    self.push_error()?;
                }
                self.pop_schema_token();
            }
            Schema::Elements { elements, .. } => {
                self.push_schema_token("elements");

                if let Some(arr) = instance.as_array() {
                    for (i, sub_instance) in arr.iter().enumerate() {
                        self.push_instance_token(&i.to_string());
                        self.validate(root, elements, None, sub_instance)?;
                        self.pop_instance_token();
                    }
                } else {
                    self.push_error()?;
                }

                self.pop_schema_token();
            }
            Schema::Properties {
                properties,
                optional_properties,
                properties_is_present,
                additional_properties,
                ..
            } => {
                if let Some(obj) = instance.as_object() {
                    self.push_schema_token("properties");
                    for (name, sub_schema) in properties {
                        self.push_schema_token(name);
                        if let Some(sub_instance) = obj.get(name) {
                            self.push_instance_token(name);
                            self.validate(root, sub_schema, None, sub_instance)?;
                            self.pop_instance_token();
                        } else {
                            self.push_error()?;
                        }
                        self.pop_schema_token();
                    }
                    self.pop_schema_token();

                    self.push_schema_token("optionalProperties");
                    for (name, sub_schema) in optional_properties {
                        self.push_schema_token(name);
                        if let Some(sub_instance) = obj.get(name) {
                            self.push_instance_token(name);
                            self.validate(root, sub_schema, None, sub_instance)?;
                            self.pop_instance_token();
                        }
                        self.pop_schema_token();
                    }
                    self.pop_schema_token();

                    if !*additional_properties {
                        for name in obj.keys() {
                            if parent_tag != Some(name)
                                && !properties.contains_key(name)
                                && !optional_properties.contains_key(name)
                            {
                                self.push_instance_token(name);
                                self.push_error()?;
                                self.pop_instance_token();
                            }
                        }
                    }
                } else {
                    self.push_schema_token(if *properties_is_present {
                        "properties"
                    } else {
                        "optionalProperties"
                    });
                    self.push_error()?;
                    self.pop_schema_token();
                }
            }
            Schema::Values { values, .. } => {
                self.push_schema_token("values");

                if let Some(obj) = instance.as_object() {
                    for (name, sub_instance) in obj {
                        self.push_instance_token(name);
                        self.validate(root, values, None, sub_instance)?;
                        self.pop_instance_token();
                    }
                } else {
                    self.push_error()?;
                }

                self.pop_schema_token();
            }
            Schema::Discriminator {
                discriminator,
                mapping,
                ..
            } => {
                if let Some(obj) = instance.as_object() {
                    if let Some(tag) = obj.get(discriminator) {
                        if let Some(tag) = tag.as_str() {
                            if let Some(schema) = mapping.get(tag) {
                                self.push_schema_token("mapping");
                                self.push_schema_token(tag);
                                self.validate(root, schema, Some(discriminator), instance)?;
                                self.pop_schema_token();
                                self.pop_schema_token();
                            } else {
                                self.push_schema_token("mapping");
                                self.push_instance_token(discriminator);
                                self.push_error()?;
                                self.pop_instance_token();
                                self.pop_schema_token();
                            }
                        } else {
                            self.push_schema_token("discriminator");
                            self.push_instance_token(discriminator);
                            self.push_error()?;
                            self.pop_instance_token();
                            self.pop_schema_token();
                        }
                    } else {
                        self.push_schema_token("discriminator");
                        self.push_error()?;
                        self.pop_schema_token();
                    }
                } else {
                    self.push_schema_token("discriminator");
                    self.push_error()?;
                    self.pop_schema_token();
                }
            }
        };

        Ok(())
    }

    fn validate_int(
        &mut self,
        instance: &Value,
        min: f64,
        max: f64,
    ) -> Result<(), VmValidateError> {
        if let Some(val) = instance.as_f64() {
            if val.fract() != 0.0 || val < min || val > max {
                self.push_error()
            } else {
                Ok(())
            }
        } else {
            self.push_error()
        }
    }

    fn push_error(&mut self) -> Result<(), VmValidateError> {
        self.errors.push(ValidationErrorIndicator {
            instance_path: self.instance_tokens.clone(),
            schema_path: self.schema_tokens.last().unwrap().clone(),
        });

        if self.max_errors == self.errors.len() {
            Err(VmValidateError::MaxErrorsReached)
        } else {
            Ok(())
        }
    }

    fn push_schema_token(&mut self, token: &str) {
        self.schema_tokens
            .last_mut()
            .unwrap()
            .push(token.to_owned());
    }

    fn pop_schema_token(&mut self) {
        self.schema_tokens.last_mut().unwrap().pop().unwrap();
    }

    fn push_instance_token(&mut self, token: &str) {
        self.instance_tokens.push(token.to_owned());
    }

    fn pop_instance_token(&mut self) {
        self.instance_tokens.pop().unwrap();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn max_depth() {
        use serde_json::json;

        let schema = crate::Schema::from_serde_schema(
            serde_json::from_value(json!({
                "definitions": {
                    "loop": { "ref": "loop" },
                },
                "ref": "loop",
            }))
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            super::ValidateError::MaxDepthExceeded,
            super::validate(
                &schema,
                &json!(null),
                super::ValidateOptions::new().with_max_depth(3)
            )
            .unwrap_err()
        )
    }

    #[test]
    fn max_errors() {
        use serde_json::json;

        let schema = crate::Schema::from_serde_schema(
            serde_json::from_value(json!({
                "elements": { "type": "string" }
            }))
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            3,
            super::validate(
                &schema,
                &json!([null, null, null, null, null]),
                super::ValidateOptions::new().with_max_errors(3)
            )
            .unwrap()
            .len()
        )
    }

    #[test]
    fn validation_spec() {
        use std::collections::{BTreeMap, HashSet};

        #[derive(serde::Deserialize, PartialEq, Debug, Eq, Hash)]
        struct TestCaseError {
            #[serde(rename = "instancePath")]
            instance_path: Vec<String>,

            #[serde(rename = "schemaPath")]
            schema_path: Vec<String>,
        }

        #[derive(serde::Deserialize)]
        struct TestCase {
            schema: crate::SerdeSchema,
            instance: serde_json::Value,
            errors: Vec<TestCaseError>,
        }

        let test_cases: BTreeMap<String, TestCase> =
            serde_json::from_str(include_str!("../json-typedef-spec/tests/validation.json"))
                .expect("parse validation.json");

        for (test_case_name, test_case) in test_cases {
            let schema = crate::Schema::from_serde_schema(test_case.schema).expect(&test_case_name);
            schema.validate().expect(&test_case_name);

            let errors: HashSet<_> =
                super::validate(&schema, &test_case.instance, super::ValidateOptions::new())
                    .expect(&test_case_name)
                    .into_iter()
                    .map(|err| TestCaseError {
                        instance_path: err.instance_path,
                        schema_path: err.schema_path,
                    })
                    .collect();

            let test_case_errors: HashSet<_> = test_case.errors.into_iter().collect();

            assert_eq!(
                test_case_errors, errors,
                "wrong validation errors returned: {}",
                &test_case_name
            );
        }
    }
}
