#![no_main]

use libfuzzer_sys::{arbitrary::*, fuzz_target};

use flashcards::{Data, Item};

fn arbitrary_data(data: &mut Unstructured) -> Result<Data, Error> {
    let prompts_len = 1 + data.arbitrary_len::<usize>()?;

    let mut prompts = Vec::with_capacity(prompts_len);

    for _ in 0..prompts_len {
        prompts.push(String::arbitrary(data)?);
    }

    let questions_len = 1 + data.arbitrary_len::<usize>()?;

    let mut questions = Vec::with_capacity(questions_len);

    for _ in 0..questions_len {
        let attempts = usize::arbitrary(data)?;
        let errors = data.int_in_range(0..=attempts)?;
        questions.push(Item {
            answers: data.arbitrary()?,
            attempts,
            errors,
            last_attempt: data.arbitrary()?,
            prompt_index: data.int_in_range(0..=prompts_len - 1)?,
            question: data.arbitrary()?,
            streak: if errors == 0 {
                attempts
            } else {
                data.int_in_range(0..=attempts)?
            },
        });
    }

    questions.sort_unstable_by_key(|q| q.prompt_index);

    // pub very_recent: usize,
    // pub recent: usize,
    // pub prompts: Vec<String>,
    // pub questions: Vec<Item>,

    let recent = data.int_in_range(0..=questions_len)?;
    let very_recent = data.int_in_range(0..=recent)?;

    Ok(Data {
        prompts,
        questions,
        recent,
        very_recent,
    })
}

#[derive(Debug)]
struct MyData(Data);

impl Arbitrary<'_> for MyData {
    fn arbitrary(data: &mut Unstructured<'_>) -> Result<Self, Error> {
        Ok(Self(arbitrary_data(data)?))
    }
}

fuzz_target!(|my_data: MyData| {
    let data = my_data.0;
    let serialized = data.serialize();
    let (deserialized, errors) = Data::deserialize(&serialized);
    assert!(errors.is_empty());
    assert_eq!(data, deserialized);
});
