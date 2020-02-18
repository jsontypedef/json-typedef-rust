use crate::form::Form;
use std::collections::HashMap;

pub struct Schema {
    pub definitions: HashMap<String, Schema>,
    pub form: Form,
}
