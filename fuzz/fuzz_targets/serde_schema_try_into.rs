#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|serde_schema: jtd::serde::Schema| {
    use std::convert::TryInto;
    let _: Result<jtd::Schema, jtd::schema::SerdeConvertError> = serde_schema.try_into();
});
