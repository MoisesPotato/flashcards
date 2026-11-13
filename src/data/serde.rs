use std::{iter::Peekable, str::CharIndices};

#[cfg(test)]
use pretty_assertions::assert_eq;

use crate::data::{Data, Item};

#[derive(Debug)]
pub struct Error {
    pub msg: String,
    pub byte_index: (usize, usize),
}

impl Data {
    pub fn deserialize(input: &str) -> (Self, Vec<Error>) {
        let mut output = Self::default();

        let mut errors = vec![];

        let mut chars = input.char_indices().peekable();

        skip_whitespace(&mut chars);

        loop {
            match chars.peek().copied() {
                None => {
                    for q in &output.questions {
                        debug_assert_eq!(
                            q.question,
                            q.question.trim(),
                            "Questions have whitespace"
                        );
                        debug_assert_eq!(q.answers, q.answers.trim(), "Answers have whitespace");
                    }
                    for p in &output.prompts {
                        debug_assert_eq!(p, p.trim(), "Prompts have whitespace");
                    }

                    let mut i = 0;
                    let mut empty_count = 0;
                    while i < output.prompts.len() - empty_count {
                        if output.questions.iter().all(|q| q.prompt_index != i) {
                            output.prompts[i..].rotate_left(1);
                            empty_count += 1;
                            for q in &mut output.questions {
                                if q.prompt_index > i {
                                    q.prompt_index -= 1;
                                }
                            }
                        } else {
                            i += 1;
                        }
                    }

                    return (output, errors);
                }
                Some((slash_b, '/')) => {
                    let _slash = chars.next();
                    match chars.next() {
                        None => errors.push(Error {
                            msg: "Line ended unexpectedly".into(),
                            byte_index: (slash_b, slash_b + '/'.len_utf8()),
                        }),
                        Some((_, '/')) => {
                            skip_comment(&mut chars);
                        }
                        Some((_, '\n')) => {
                            skip_whitespace(&mut chars);
                        }
                        Some((_, '?')) => {
                            let prompt = prompt(input, &mut chars);
                            // println!("Prompt: {prompt:?}");
                            output.prompts.push(prompt);
                        }
                        Some((_, '*')) => {
                            data_field(input, &mut chars, &mut output, &mut errors);
                        }
                        Some((_, c)) => {
                            errors.push(Error {
                                msg: format!("A line cannot start with /{c}: Only //, /? and /* are supported"),
                                byte_index: (slash_b, slash_b + 1 + c.len_utf8())
                            });
                            skip_til_newline(&mut chars);
                        }
                    }
                }
                Some(_) => {
                    if let Some(item) = normal_line(
                        input,
                        &mut chars,
                        output.prompts.len().saturating_sub(1),
                        &mut errors,
                    ) {
                        output.questions.push(item);
                        if output.prompts.is_empty() {
                            output.prompts.push("$".into());
                        }
                    }
                }
            }
        }
    }
}

fn data_field(
    input: &str,
    chars: &mut Peekable<CharIndices<'_>>,
    output: &mut Data,
    errors: &mut Vec<Error>,
) {
    skip_whitespace(chars);
    let Some(ident_begin) = chars.peek().map(|c| c.0) else {
        errors.push(Error {
            msg: "Line starting with /* has nothing after it".into(),
            byte_index: (input.len(), input.len()),
        });
        return;
    };
    let ident = loop {
        let Some((i, c)) = chars.next() else {
            errors.push(Error {
                msg: "Line starting with /* has no semicolon".into(),
                byte_index: (ident_begin, input.len()),
            });
            skip_til_newline(chars);
            return;
        };
        if c == '\n' {
            errors.push(Error {
                msg: "Line starting with /* has no semicolon".into(),
                byte_index: (ident_begin, input.len()),
            });
            skip_whitespace(chars);
            return;
        }
        if c == ':' {
            break &input[ident_begin..i];
        }
    };
    let ident = ident.trim_end();
    skip_whitespace(chars);
    let value_begin = chars.peek().map_or(input.len(), |c| c.0);
    let value = loop {
        let Some((i, c)) = chars.next() else {
            break &input[value_begin..];
        };
        if c == '\n' {
            break &input[value_begin..i];
        }
        if c == ':' {
            errors.push(Error {
                msg: "Line starting with /* has two semicolons".into(),
                byte_index: (i, i + 1),
            });
            skip_til_newline(chars);
            return;
        }
    };
    skip_whitespace(chars);
    match ident {
        "very_recent" => match value.parse() {
            Ok(v) => output.very_recent = v,
            Err(err) => {
                errors.push(Error {
                    msg: format!("Couldn't parse {value:?}: {err}"),
                    byte_index: (value_begin, value_begin + value.len()),
                });
            }
        },
        "recent" => match value.parse() {
            Ok(v) => output.recent = v,
            Err(err) => {
                errors.push(Error {
                    msg: format!("Couldn't parse {value:?}: {err}"),
                    byte_index: (value_begin, value_begin + value.len()),
                });
            }
        },
        _ => {
            errors.push(Error {
                msg: format!(
                    "Unkonwn identifier {ident:?}: Possible values are 'very_recent' and 'recent'"
                ),
                byte_index: (ident_begin, ident_begin + ident.len()),
            });
        }
    }
}

fn prompt(input: &str, chars: &mut Peekable<CharIndices<'_>>) -> String {
    let beginning = chars.peek().map_or(input.len(), |x| x.0);
    loop {
        match chars.next() {
            None => return input[beginning..].trim().into(),
            Some((i, '\n')) => {
                skip_whitespace(chars);
                return input[beginning..i].trim().into();
            }
            Some(_) => {}
        }
    }
}

fn normal_line(
    input: &str,
    chars: &mut Peekable<CharIndices<'_>>,
    current_prompt: usize,
    errors: &mut Vec<Error>,
) -> Option<Item> {
    let beginning = chars.peek()?.0;
    let mut output = Item {
        question: field(chars, errors).or_else(|| {
            skip_whitespace(chars);
            None
        })?,
        answers: {
            let Some(answers) = field(chars, errors) else {
                errors.push(Error {
                    msg: "Question without answer".into(),
                    byte_index: (beginning, chars.peek().map_or(input.len(), |x| x.0)),
                });
                return None;
            };
            answers
        },
        prompt_index: current_prompt,
        ..Item::default()
    };

    output.attempts = field_number(input, chars, errors).unwrap_or_default();
    output.errors = field_number(input, chars, errors).unwrap_or_default();
    output.last_attempt = field_number(input, chars, errors).unwrap_or_default();
    output.streak = field_number(input, chars, errors).unwrap_or(if output.errors == 0 {
        output.attempts
    } else {
        0
    });

    skip_whitespace(chars);

    // println!("{output:#?}");

    Some(output)
}

fn field_number(
    input: &str,
    chars: &mut Peekable<CharIndices<'_>>,
    errors: &mut Vec<Error>,
) -> Option<usize> {
    if chars.peek()?.1 == '\n' {
        return None;
    }
    let beginning = chars.peek()?.0;
    let output = loop {
        let Some((i, c)) = chars.next() else {
            break &input[beginning..];
        };
        if c == ',' || c == '\n' {
            break &input[beginning..i];
        }
    };

    match output.parse() {
        Err(err) => {
            errors.push(Error {
                msg: format!("Failed to parse number {output:?}: {err}"),
                byte_index: (beginning, beginning + output.len()),
            });
            None
        }
        Ok(x) => Some(x),
    }
}

fn field(chars: &mut Peekable<CharIndices<'_>>, errors: &mut Vec<Error>) -> Option<String> {
    let mut output = String::new();
    match *chars.peek()? {
        (_, '\n') => None,
        (beginning, '"') => field_in_quotes(chars, errors, output, beginning),
        (_, _) => loop {
            match chars.peek().copied() {
                None | Some((_, '\n')) => {
                    return Some(trim_right(output));
                }
                Some((_, ',')) => {
                    chars.next();
                    skip_whitespace(chars);
                    return Some(trim_right(output));
                }
                Some((i, '"')) => {
                    chars.next();
                    skip_whitespace(chars);
                    errors.push(Error {
                        msg: "Quotes are not allowed in questions or answers. Use '\\\"'".into(),
                        byte_index: (i, i + 1),
                    });
                    skip_til_newline(chars);
                    return None;
                }
                Some((i, '\\')) => {
                    chars.next();
                    match chars.next() {
                        Some((_, c @ ('"' | '\\'))) => output.push(c),
                        Some((_, 'n')) => {
                            errors.push(Error {
                                msg: "New lines are not allowed in questinos of answers".into(),
                                byte_index: (i, i + 2),
                            });
                            skip_til_newline(chars);
                            return None;
                        }
                        None => {
                            errors.push(Error {
                                msg: "Unfinished escape sequence".into(),
                                byte_index: (i, i + 1),
                            });
                            skip_til_newline(chars);
                            return None;
                        }
                        Some((_, c)) => {
                            errors.push(Error {
                                msg: format!("Unsuppored escape sequence \\{c}"),
                                byte_index: (i, i + 1 + c.len_utf8()),
                            });
                            skip_til_newline(chars);
                            return None;
                        }
                    }
                }
                Some((_, c)) => {
                    chars.next();
                    output.push(c);
                }
            }
        },
    }
}

fn trim_right(mut output: String) -> String {
    while let Some(c) = output.pop() {
        if !c.is_whitespace() {
            output.push(c);
            break;
        }
    }
    output
}

fn field_in_quotes(
    chars: &mut Peekable<CharIndices<'_>>,
    errors: &mut Vec<Error>,
    mut output: String,
    beginning: usize,
) -> Option<String> {
    chars.next();
    skip_whitespace(chars);
    loop {
        let Some((i, c)) = chars.next() else {
            errors.push(Error {
                msg: "Unclosed \"".into(),
                byte_index: (beginning, beginning + 1),
            });
            return None;
        };
        match c {
            '"' => {
                skip_whitespace(chars);
                match chars.peek() {
                    None => {}
                    Some((_, ',')) => {
                        chars.next();
                        skip_whitespace(chars);
                    }
                    Some(&(i, c)) => {
                        errors.push(Error {
                            msg: "Expected ',' or new line".into(),
                            byte_index: (i, i + c.len_utf8()),
                        });
                        skip_til_newline(chars);
                        return None;
                    }
                }
                return Some(trim_right(output));
            }
            '\n' => {
                errors.push(Error {
                    msg: "Questions and answers must be a single line".into(),
                    byte_index: (beginning + 1, i),
                });
                skip_whitespace(chars);
                return None;
            }
            '\\' => match chars.next() {
                Some((i, 'n')) => {
                    errors.push(Error {
                        msg: "Questions and answers must be a single line".into(),
                        byte_index: (beginning + 1, i),
                    });
                    skip_til_newline(chars);
                    return None;
                }
                Some((_, c @ ('\\' | '"' | '/'))) => {
                    output.push(c);
                }
                Some((_, c)) => {
                    errors.push(Error {
                        msg: format!("Unsupported escape sequence \\{c}"),
                        byte_index: (i, i + 1 + c.len_utf8()),
                    });
                    skip_til_newline(chars);
                    return None;
                }
                None => {
                    errors.push(Error {
                        msg: "Expected a character after '\\'".into(),
                        byte_index: (i, i + 1),
                    });
                    return None;
                }
            },
            _ => {
                output.push(c);
            }
        }
    }
}

fn skip_til_newline(chars: &mut Peekable<CharIndices<'_>>) {
    for (_, c) in chars.by_ref() {
        if c == '\n' {
            skip_whitespace(chars);
            return;
        }
    }
}
fn skip_whitespace(chars: &mut Peekable<CharIndices<'_>>) {
    loop {
        match chars.peek().copied() {
            Some((_, c)) if c.is_whitespace() => {
                chars.next();
            }
            Some(_) | None => return,
        }
    }
}
fn skip_comment(chars: &mut Peekable<CharIndices<'_>>) {
    loop {
        match chars.next() {
            None => return,
            Some((_, '\n')) => {
                skip_whitespace(chars);
                return;
            }
            Some(_) => {}
        }
    }
}

#[test]
fn reject_newlines() {
    let tests = [
        r#"/?What is the capital of $?
Afghanistan,"kabul
Afghanistan",6,0,150,6
"#,
        r#"/?What is the capital of $?
Afghanistan,"kabul\nAfghanistan",6,0,150,6
"#,
        r"/?What is the capital of $?
Afgha
// Nothing here
",
        r#"/?What is the capital of $?
"Afgha
// Nothing here
"#,
        r#"/?What is the capital of $?
"Afgha\nistan","kabul",6,0,150,6
"#,
    ];

    for test in tests {
        let (data, errors) = Data::deserialize(test);
        assert!(
            !errors.is_empty(),
            "Got no errors: Shouldn't accept newlines in\n{test}"
        );
        assert!(
            data.questions.is_empty(),
            "Parsing\n{test}\nShould be rejected because of the newline, got\n{data:#?}\nErrors\n{errors:#?}"
        );
    }
}

#[test]
fn fuzzed_serialized() {
    let inputs = [
        Data {
            very_recent: 0,
            recent: 0,
            prompts: vec![String::new()],
            questions: vec![
                Item {
                    prompt_index: 0,
                    question: String::new(),
                    answers: "\\\0".into(),
                    attempts: 13_589_282_577_500,
                    errors: 6_081_631_320_437,
                    last_attempt: 0,
                    streak: 0,
                },
                Item {
                    prompt_index: 0,
                    question: String::new(),
                    answers: String::new(),
                    attempts: 0,
                    errors: 0,
                    last_attempt: 0,
                    streak: 0,
                },
            ],
        },
        Data {
            prompts: vec![String::new()],
            questions: vec![Item {
                question: "/".into(),
                answers: "/".into(),
                ..Item::default()
            }],
            ..Data::default()
        },
    ];

    for data in inputs {
        let serialized = data.serialize();
        let (deserialized, errors) = Data::deserialize(&serialized);
        assert!(
            errors.is_empty(),
            "Deserializing yielded errors:\n{errors:#?}\nInput\n{data:#?}\nSerialized:\n{serialized:?}"
        );
        assert_eq!(data, deserialized);
    }
}

#[test]
fn fuzzed_deserialize() {
    const INPUTS: &[&str] = &[
        ",\"/\n /,\"\"",
        "/*\u{3}\n /,=",
        "/u{3}\n \u{16},/",
        "\n/?1\n/?2\nq,a",
        ",\" \"",
        ",\"\u{b}\"",
        "q ,a",
        "~\u{b},/",
        "/? ",
        "/?\u{b}",
        "\"\", ",
        "\"\",\u{b}",
        "\"\\#",
        "\"\\",
        "\"\"j",
        "/?",
        "\u{19}",
    ];

    for input in INPUTS {
        let (data, errors) = Data::deserialize(input);

        let serialized = data.serialize();

        eprintln!("Before:\n{input:?}\nErrors:{errors:#?}\nAfter:\n{serialized:?}");

        let (deserialized, errors) = Data::deserialize(&serialized);

        assert!(errors.is_empty());
        assert_eq!(data, deserialized);
    }
}
