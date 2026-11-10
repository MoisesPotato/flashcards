#![no_main]

use libfuzzer_sys::fuzz_target;

use flashcards::Data;

fuzz_target!(|input: &str| {
    let (data, _errors) = Data::deserialize(input);
    let serialized = data.serialize();
    let (deserialized, errors) = Data::deserialize(&serialized);
    assert!(errors.is_empty());
    assert_eq!(data, deserialized);
});
