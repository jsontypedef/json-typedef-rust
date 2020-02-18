pub mod form;
pub mod schema;
pub mod serde;

pub use form::Form;
pub use schema::Schema;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
