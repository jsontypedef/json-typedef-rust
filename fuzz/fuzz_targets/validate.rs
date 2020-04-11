#![no_main]
use libfuzzer_sys::fuzz_target;

use serde_json;

fuzz_target!(|schema_and_instance: (jtd::schema::Schema, Vec<u8>)| {
    let validator = jtd::validator::Validator {
        max_errors: None,
        max_depth: None,
    };

    // We're only interested in fuzzing against valid schemas.
    if schema_and_instance.0.validate().is_err() {
        return;
    }

    if let Ok(instance) = serde_json::from_slice(&schema_and_instance.1) {
        let _ = validator.validate(&schema_and_instance.0, &instance);
    }
});
