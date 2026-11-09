use std::{iter::Peekable, str::CharIndices};

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
                None => return (output, errors),
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
                        Some((_, '?')) => {
                            let prompt = prompt(input, &mut chars);
                            // println!("Prompt: {prompt:?}");
                            output.prompts.push(prompt);
                        }
                        Some((_, '*')) => {
                            data_field(input, &mut chars, &mut output, &mut errors);
                        }
                        Some((_, c)) => todo!("Line starts with /{c}"),
                    }
                }
                Some(_) => {
                    if output.prompts.is_empty() {
                        output.prompts.push("$".into());
                    }
                    if let Some(item) =
                        normal_line(input, &mut chars, output.prompts.len() - 1, &mut errors)
                    {
                        output.questions.push(item);
                    }
                }
            }
        }

        // let mut lines = input.lines();
        // let recent_line = lines.next().ok_or_else(|| anyhow!("File is empty"))?;
        // let strip_beginning = recent_line
        //     .strip_prefix("// Very recent: ")
        //     .ok_or_else(|| anyhow!("Expect first line to start with '// Very recent: '"))?;
        // let (very_recent, recent) = strip_beginning
        //     .split_once(", Recent: ")
        //     .ok_or_else(|| anyhow!("Expect first line to contain ', Recent: '"))?;
        // let very_recent = very_recent
        //     .parse()
        //     .map_err(|err| anyhow!("Failed to parse {very_recent}: {err}"))?;
        // let recent = recent
        //     .parse()
        //     .map_err(|err| anyhow!("Failed to parse {recent}: {err}"))?;
        // let mut prompts = vec![];
        // let mut questions = vec![];
        // for line in lines {
        //     if let Some(prompt) = line.strip_prefix("//?") {
        //         prompts.push(prompt.to_owned());
        //     } else {
        //         let mut pieces = line.split(',');

        //         let question = pieces
        //             .next()
        //             .ok_or_else(|| anyhow!("Line ends unexpectedly:\n{line}"))?;
        //         let answers = pieces
        //             .next()
        //             .ok_or_else(|| anyhow!("Line ends unexpectedly:\n{line}"))?;
        //         let attempts = parse_one_or(&mut pieces, 0)?;
        //         let errors = parse_one_or(&mut pieces, 0)?;
        //         let last_attempt = parse_one_or(&mut pieces, recent)?;
        //         let streak = parse_one_or(&mut pieces, if errors == 0 { attempts } else { 0 })?;

        //         questions.push(Item {
        //             prompt_index: prompts
        //                 .len()
        //                 .checked_sub(1)
        //                 .ok_or_else(|| anyhow!("Found a question without prompt"))?,
        //             question: question.into(),
        //             answers: answers.into(),
        //             attempts,
        //             errors,
        //             last_attempt,
        //             streak,
        //         });
        //     }
        // }

        // Ok(Self {
        //     very_recent,
        //     recent,
        //     prompts,
        //     questions,
        // })
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
            return;
        };
        if c == '\n' {
            errors.push(Error {
                msg: "Line starting with /* has no semicolon".into(),
                byte_index: (ident_begin, input.len()),
            });
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
            None => return input[beginning..].into(),
            Some((i, '\n')) => {
                skip_whitespace(chars);
                return input[beginning..i].into();
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
        question: field(input, chars, errors).unwrap(),
        answers: {
            let Some(answers) = field(input, chars, errors) else {
                errors.push(Error {
                    msg: "Question without answer".into(),
                    byte_index: (beginning, chars.peek().map_or(input.len(), |x| x.0)),
                });
                return None;
            };
            answers
        },
        prompt_index: current_prompt,
        // attempts: todo!(),
        // errors: todo!(),
        // last_attempt: todo!(),
        // streak: todo!()
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

fn field(
    input: &str,
    chars: &mut Peekable<CharIndices<'_>>,
    errors: &mut Vec<Error>,
) -> Option<String> {
    let mut output = String::new();
    match *chars.peek()? {
        (_, '\n') => None,
        (beginning, '"') => {
            chars.next();
            loop {
                let Some((_, c)) = chars.next() else {
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
                            }
                            x => todo!("Error: found {x:?} after close quotes. Parsed {output:?}"),
                        }
                        return Some(output);
                    }
                    '\\' => todo!("Escape"),
                    _ => {
                        output.push(c);
                    }
                }
            }
        }
        (beginning, _) => loop {
            match chars.next() {
                None => {
                    return Some(output);
                }
                Some((_, ',')) => {
                    skip_whitespace(chars);
                    return Some(output);
                }
                Some((_, '\\')) => todo!("Escape"),
                Some((_, c)) => {
                    output.push(c);
                }
            }
        },
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
