use crate::form;
use crate::schema::Schema;
use chrono::DateTime;
use serde_json::Value;

#[derive(Debug)]
pub struct Validator {
    pub max_depth: Option<usize>,
    pub max_errors: Option<usize>,
}

#[derive(Debug)]
pub struct ValidationError {
    pub instance_path: Vec<String>,
    pub schema_path: Vec<String>,
}

#[derive(Debug)]
pub enum ValidateError {
    MaxDepthExceeded,
}

impl Validator {
    pub fn validate(
        &self,
        schema: &Schema,
        instance: &Value,
    ) -> Result<Vec<ValidationError>, ValidateError> {
        let mut vm = Vm {
            max_depth: self.max_depth,
            max_errors: self.max_errors,
            instance_tokens: vec![],
            schema_tokens: vec![vec![]],
            errors: vec![],
        };

        match vm.validate(schema, schema, None, instance) {
            Ok(()) | Err(VmValidateError::MaxErrorsReached) => Ok(vm.errors),
            Err(VmValidateError::MaxDepthExceeded) => Err(ValidateError::MaxDepthExceeded),
        }
    }
}

struct Vm {
    pub max_depth: Option<usize>,
    pub max_errors: Option<usize>,
    pub instance_tokens: Vec<String>,
    pub schema_tokens: Vec<Vec<String>>,
    pub errors: Vec<ValidationError>,
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
        match &schema.form {
            form::Form::Empty => {}
            form::Form::Ref(form::Ref {
                nullable,
                definition,
            }) => {
                if !*nullable || !instance.is_null() {
                    self.schema_tokens
                        .push(vec!["definitions".to_owned(), definition.clone()]);
                    self.validate(root, &root.definitions[definition], None, instance)?;
                    self.schema_tokens.pop();
                }
            }
            form::Form::Type(form::Type {
                nullable,
                type_value,
            }) => {
                if !*nullable || !instance.is_null() {
                    self.push_schema_token("type");

                    match type_value {
                        form::TypeValue::Boolean => {
                            if !instance.is_boolean() {
                                self.push_error()?;
                            }
                        }
                        form::TypeValue::Float32 | form::TypeValue::Float64 => {
                            if !instance.is_f64() && !instance.is_i64() {
                                self.push_error()?;
                            }
                        }
                        form::TypeValue::Int8 => self.validate_int(instance, -128.0, 127.0)?,
                        form::TypeValue::Uint8 => self.validate_int(instance, 0.0, 255.0)?,
                        form::TypeValue::Int16 => self.validate_int(instance, -32768.0, 32767.0)?,
                        form::TypeValue::Uint16 => self.validate_int(instance, 0.0, 65535.0)?,
                        form::TypeValue::Int32 => {
                            self.validate_int(instance, -2147483648.0, 2147483647.0)?
                        }
                        form::TypeValue::Uint32 => {
                            self.validate_int(instance, 0.0, 4294967295.0)?
                        }
                        form::TypeValue::String => {
                            if !instance.is_string() {
                                self.push_error()?;
                            }
                        }
                        form::TypeValue::Timestamp => {
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
            }
            form::Form::Elements(form::Elements { nullable, schema }) => {
                if !*nullable || !instance.is_null() {
                    self.push_schema_token("elements");

                    if let Some(arr) = instance.as_array() {
                        for (i, sub_instance) in arr.iter().enumerate() {
                            self.push_instance_token(&i.to_string());
                            self.validate(root, schema, None, sub_instance)?;
                            self.pop_instance_token();
                        }
                    } else {
                        self.push_error()?;
                    }

                    self.pop_schema_token();
                }
            }
            form::Form::Properties(form::Properties {
                nullable,
                required,
                optional,
                additional,
                has_required,
            }) => {
                if !*nullable || !instance.is_null() {
                    if let Some(obj) = instance.as_object() {
                        self.push_schema_token("properties");
                        for (name, sub_schema) in required {
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
                        for (name, sub_schema) in optional {
                            self.push_schema_token(name);
                            if let Some(sub_instance) = obj.get(name) {
                                self.push_instance_token(name);
                                self.validate(root, sub_schema, None, sub_instance)?;
                                self.pop_instance_token();
                            }
                            self.pop_schema_token();
                        }
                        self.pop_schema_token();

                        if !*additional {
                            self.push_schema_token(if *has_required {
                                "properties"
                            } else {
                                "optionalProperties"
                            });

                            for name in obj.keys() {
                                if parent_tag != Some(name)
                                    && !required.contains_key(name)
                                    && !optional.contains_key(name)
                                {
                                    self.push_instance_token(name);
                                    self.push_error()?;
                                    self.pop_instance_token();
                                }
                            }

                            self.pop_schema_token();
                        }
                    } else {
                        self.push_schema_token(if *has_required {
                            "properties"
                        } else {
                            "optionalProperties"
                        });
                        self.push_error()?;
                        self.pop_schema_token();
                    }
                }
            }
            form::Form::Values(form::Values { nullable, schema }) => {
                if !*nullable || !instance.is_null() {
                    self.push_schema_token("values");

                    if let Some(obj) = instance.as_object() {
                        for (name, sub_instance) in obj {
                            self.push_instance_token(name);
                            self.validate(root, schema, None, sub_instance)?;
                            self.pop_instance_token();
                        }
                    } else {
                        self.push_error()?;
                    }

                    self.pop_schema_token();
                }
            }
            form::Form::Discriminator(form::Discriminator {
                nullable,
                discriminator,
                mapping,
            }) => {
                if !*nullable || !instance.is_null() {
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
            }
            _ => unimplemented!(),
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
        self.errors.push(ValidationError {
            instance_path: self.instance_tokens.clone(),
            schema_path: self.schema_tokens.last().unwrap().clone(),
        });

        Ok(())
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
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::collections::{HashMap, HashSet};
    use std::convert::TryInto;

    #[test]
    fn spec_validation_suite() {
        #[derive(Serialize, Deserialize)]
        struct TestCase {
            schema: crate::serde::Schema,
            instance: Value,
            errors: Vec<TestCaseError>,
        }

        #[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
        #[serde(rename_all = "camelCase")]
        struct TestCaseError {
            instance_path: Vec<String>,
            schema_path: Vec<String>,
        }

        let test_cases: HashMap<String, TestCase> =
            serde_json::from_str(include_str!("../json-typedef-spec/tests/validation.json"))
                .unwrap();

        for (name, test_case) in test_cases {
            let schema = test_case
                .schema
                .try_into()
                .expect(&format!("parsing schema: {}", name));

            let validator = Validator {
                max_depth: None,
                max_errors: None,
            };

            let errors: HashSet<_> = validator
                .validate(&schema, &test_case.instance)
                .expect(&format!("validating: {}", name))
                .into_iter()
                .map(|err| TestCaseError {
                    instance_path: err.instance_path,
                    schema_path: err.schema_path,
                })
                .collect();

            assert_eq!(
                test_case.errors.into_iter().collect::<HashSet<_>>(),
                errors,
                "wrong set of errors returned for test case: {}",
                name
            );
        }
    }
}
