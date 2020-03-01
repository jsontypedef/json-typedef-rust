pub mod form;
pub mod schema;
pub mod serde;
pub mod validator;

pub use crate::serde::Schema as SerdeSchema;
pub use form::Form;
pub use schema::Schema;
pub use validator::{ValidationError, Validator};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
