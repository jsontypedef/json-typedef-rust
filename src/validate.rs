use crate::{Schema, Type};
use chrono::DateTime;
use serde_json::Value;
use std::borrow::Cow;
use thiserror::Error;

/// Options you can pass to [`validate()`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValidateOptions {
    max_depth: usize,
    max_errors: usize,
}

impl ValidateOptions {
    /// Construct a new set of options with all default values.
    ///
    /// Equivalent to [`Default::default()`] or calling `with_max_depth(0)` and
    /// `with_max_errors(0)`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum "depth" of references to following in [`validate()`].
    ///
    /// This option exists to handle the possibility of an infinite loop in a
    /// schema. For instance, this is a valid schema:
    ///
    /// ```json
    /// { "ref": "loop", "definitions": { "loop": { "ref": "loop" }}}
    /// ```
    ///
    /// There are good reasons to sometimes have self-referential schemas -- for
    /// instance, to describe a recursive data structure. What `with_max_depth`
    /// does is limit how many recursive `ref` nodes will be followed before
    /// [`validate()`] errors with [`ValidateError::MaxDepthExceeded`].
    ///
    /// The default max depth of `0` indicates that no max depth should be
    /// implemented. An infinite `ref` loop will eventually overflow the stack
    /// during [`validate()`].
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Sets the maximum number of validation errors to return from
    /// [`validate()`].
    ///
    /// This option exists as an optimization for [`validate()`]. If all you
    /// care about is whether an input is valid, then consider using
    /// `set_max_errors(1)` to have [`validate()`] immediately return after
    /// finding a validation error.
    ///
    /// The default max errors of `0` indicates that all errors will be
    /// returned.
    pub fn with_max_errors(mut self, max_errors: usize) -> Self {
        self.max_errors = max_errors;
        self
    }
}

/// Errors that may arise from [`validate()`].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ValidateError {
    /// The maximum depth, as specified by [`ValidateOptions::with_max_depth`],
    /// was exceeded.
    ///
    /// ```
    /// use serde_json::json;
    /// use jtd::{Schema, ValidateError, ValidateOptions};
    ///
    /// let schema = Schema::from_serde_schema(
    ///     serde_json::from_value(json!({
    ///         "definitions": {
    ///             "loop": { "ref": "loop" },
    ///         },
    ///         "ref": "loop",
    ///     }))
    ///     .unwrap(),
    /// )
    /// .unwrap();
    ///
    /// assert_eq!(
    ///     ValidateError::MaxDepthExceeded,
    ///     jtd::validate(
    ///         &schema,
    ///         &json!(null),
    ///         ValidateOptions::new().with_max_depth(3)
    ///     )
    ///     .unwrap_err()
    /// )
    /// ```
    #[error("max depth exceeded")]
    MaxDepthExceeded,
}

/// A single validation error returned by [`validate()`].
///
/// This type has *Indicator* at the end of its name to emphasize that it is
/// *not* a Rust error. It is an ordinary struct, and corresponds to the concept
/// of a validation error indicator in the JSON Typedef specification. See
/// [RFC8927, Section 3.2](https://tools.ietf.org/html/rfc8927#section-3.2).
///
/// In order to avoid unncessary allocations, this struct uses
/// [`std::borrow::Cow`] instead of [`String`] directly. If you would prefer not
/// to have to deal with that, and are OK with copying all the data out of this
/// struct, then use
/// [`into_owned_paths`][`ValidationErrorIndicator::into_owned_paths`] to
/// convert instances of this type into a pair of plain old `Vec<String>`s.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationErrorIndicator<'a> {
    /// A path to the part of the instance that was rejected.
    pub instance_path: Vec<Cow<'a, str>>,

    /// A path to the part of the schema that rejected the instance.
    pub schema_path: Vec<Cow<'a, str>>,
}

impl<'a> ValidationErrorIndicator<'a> {
    /// Converts this struct into a `instance_path` and `schema_path` pair.
    ///
    /// This is a convenience function for those who don't want to manipulate
    /// [`std::borrow::Cow`].
    ///
    /// ```
    /// use std::borrow::Cow;
    ///
    /// let indicator = jtd::ValidationErrorIndicator {
    ///     instance_path: vec![Cow::Borrowed("foo")],
    ///     schema_path: vec![Cow::Owned("bar".to_owned())],
    /// };
    ///
    /// let (instance_path, schema_path) = indicator.into_owned_paths();
    /// assert_eq!(vec!["foo".to_owned()], instance_path);
    /// assert_eq!(vec!["bar".to_owned()], schema_path);
    /// ```
    pub fn into_owned_paths(self) -> (Vec<String>, Vec<String>) {
        (
            self.instance_path
                .into_iter()
                .map(|c| c.into_owned())
                .collect(),
            self.schema_path
                .into_iter()
                .map(|c| c.into_owned())
                .collect(),
        )
    }
}

/// Validates a schema against an instance, returning a set of error indicators.
///
/// In keeping with the conventions of RFC8927, the "input" JSON -- the second
/// argument to this function -- is called an *instance*.
///
/// The set of error indicators returned is specified by the JSON Typedef
/// specification. The ordering of those errors is not defined by the JSON
/// Typedef specification, and is subject to change in a future version of this
/// crate.
///
/// ```
/// use jtd::{Schema, ValidationErrorIndicator, ValidateOptions};
/// use serde_json::json;
///
/// let schema = Schema::from_serde_schema(
///     serde_json::from_value(json!({
///         "elements": {
///             "type": "uint8"
///         }
///     })).unwrap()).unwrap();
///
/// let instance = serde_json::json!([ "a", "b", "c" ]);
///
/// // By default, jtd::validate() will return all errors in the input.
/// let validate_options = ValidateOptions::new();
/// let errors = jtd::validate(&schema, &instance, validate_options).unwrap();
/// assert_eq!(
///     vec![
///         ValidationErrorIndicator {
///             instance_path: vec!["0".to_owned().into()],
///             schema_path: vec!["elements".into(), "type".into()],
///         },
///         ValidationErrorIndicator {
///             instance_path: vec!["1".to_owned().into()],
///             schema_path: vec!["elements".into(), "type".into()],
///         },
///         ValidationErrorIndicator {
///             instance_path: vec!["2".to_owned().into()],
///             schema_path: vec!["elements".into(), "type".into()],
///         },
///     ],
///     errors,
/// );
///
/// // If you don't care about validation errors beyond a certain amount of
/// // errors, use with_max_errors on the ValidateOptions you pass to validate.
/// let validate_options = ValidateOptions::new().with_max_errors(1);
/// let errors = jtd::validate(&schema, &instance, validate_options).unwrap();
/// assert_eq!(
///     vec![
///         ValidationErrorIndicator {
///             instance_path: vec!["0".to_owned().into()],
///             schema_path: vec!["elements".into(), "type".into()],
///         },
///     ],
///     errors,
/// );
/// ```
///
/// # Security considerations
///
/// (This note is copied from [the top-level documentation][`crate`], because
/// it's important.)
///
/// If you're running [`validate()`] with untrusted schemas (untrusted inputs is
/// fine), then be aware of this security consideration from RFC 8927:
///
/// > Implementations that evaluate user-inputted schemas SHOULD implement
/// > mechanisms to detect and abort circular references that might cause a
/// > naive implementation to go into an infinite loop.  Without such
/// > mechanisms, implementations may be vulnerable to denial-of-service
/// > attacks.
///
/// This crate supports that "detect and abort" mechanism via
/// [`ValidateOptions::with_max_depth`]. Please see that documentation if you're
/// validating data against untrusted schemas.
pub fn validate<'a>(
    schema: &'a Schema,
    instance: &'a Value,
    options: ValidateOptions,
) -> Result<Vec<ValidationErrorIndicator<'a>>, ValidateError> {
    let mut vm = Vm::new(schema, options);

    match vm.validate(schema, None, instance) {
        Ok(()) | Err(VmValidateError::MaxErrorsReached) => Ok(vm.into_errors()),
        Err(VmValidateError::MaxDepthExceeded) => Err(ValidateError::MaxDepthExceeded),
    }
}

struct Vm<'a> {
    root: &'a Schema,
    options: ValidateOptions,
    instance_tokens: Vec<Cow<'a, str>>,
    schema_tokens: Vec<Vec<Cow<'a, str>>>,
    errors: Vec<ValidationErrorIndicator<'a>>,
}

enum VmValidateError {
    MaxErrorsReached,
    MaxDepthExceeded,
}

impl<'a> Vm<'a> {
    pub fn new(schema: &'a Schema, options: ValidateOptions) -> Self {
        Self {
            root: schema,
            options,
            instance_tokens: vec![],
            schema_tokens: vec![vec![]],
            errors: vec![],
        }
    }

    pub fn into_errors(self) -> Vec<ValidationErrorIndicator<'a>> {
        self.errors
    }

    pub fn validate(
        &mut self,
        schema: &'a Schema,
        parent_tag: Option<&'a str>,
        instance: &'a Value,
    ) -> Result<(), VmValidateError> {
        if instance.is_null() && schema.nullable() {
            return Ok(());
        }

        match schema {
            Schema::Empty { .. } => {}
            Schema::Ref { ref_, .. } => {
                self.schema_tokens
                    .push(vec!["definitions".into(), ref_.into()]);

                if self.schema_tokens.len() == self.options.max_depth {
                    return Err(VmValidateError::MaxDepthExceeded);
                }

                self.validate(&self.root.definitions()[ref_], None, instance)?;
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
                        // This is the only case where we push a non-Borrowed
                        // instance token. We handle pushing to instance_tokens
                        // manually here, to keep push_instance_token simpler.
                        self.instance_tokens.push(Cow::Owned(i.to_string()));

                        self.validate(elements, None, sub_instance)?;
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
                            self.validate(sub_schema, None, sub_instance)?;
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
                            self.validate(sub_schema, None, sub_instance)?;
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
                        self.validate(values, None, sub_instance)?;
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
                                self.validate(schema, Some(discriminator), instance)?;
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

        if self.options.max_errors == self.errors.len() {
            Err(VmValidateError::MaxErrorsReached)
        } else {
            Ok(())
        }
    }

    fn push_schema_token(&mut self, token: &'a str) {
        self.schema_tokens.last_mut().unwrap().push(token.into());
    }

    fn pop_schema_token(&mut self) {
        self.schema_tokens.last_mut().unwrap().pop().unwrap();
    }

    fn push_instance_token(&mut self, token: &'a str) {
        self.instance_tokens.push(token.into());
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
                    .map(|err| err.into_owned_paths())
                    .map(|(instance_path, schema_path)| TestCaseError {
                        instance_path,
                        schema_path,
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
