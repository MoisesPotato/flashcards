use std::{iter::Peekable, str::CharIndices};

#[cfg(test)]
use pretty_assertions::assert_eq;

use crate::data::{Data, Item};

use LineOutput::{Comment, Prompt};

#[derive(Debug, Clone)]
pub struct Error {
    pub msg: String,
    pub line: usize,
    pub byte_index: (usize, usize),
}

impl Error {
    fn from_msg(msg: impl Into<String>, byte_index: (usize, usize)) -> Self {
        Self {
            msg: msg.into(),
            line: 0,
            byte_index,
        }
    }
}

#[derive(Debug)]
pub enum Parameter {
    Recent(usize),
    VeryRecent(usize),
}

#[derive(Debug)]
pub enum LineOutput {
    Comment,
    Prompt {
        contents: String,
        start: usize,
    },
    Parameter {
        value: Parameter,
        key: (usize, usize),
        value_bytes: (usize, usize),
    },
    Item {
        value: Item,
        q_span: (usize, usize),
        a_span: (usize, usize),
    },
}

impl LineOutput {
    pub(crate) fn deserialize(input: &str) -> Result<Self, Error> {
        let mut chars = input.char_indices().peekable();

        skip_whitespace(&mut chars);

        match chars.peek().copied() {
            None => Ok(Self::Comment),
            Some((slash_b, '/')) => {
                let _slash = chars.next();
                match chars.next() {
                    None | Some((_, '/')) => Ok(Self::Comment),
                    Some((_, '?')) => {
                        skip_whitespace(&mut chars);
                        let start = chars.peek().map_or(input.len(), |x| x.0);
                        let contents = input[start..].trim().to_owned();
                        // println!("Prompt: {prompt:?}");
                        Ok(Self::Prompt { contents, start })
                    }
                    Some((_, '*')) => data_field(input, &mut chars),
                    Some((_, c)) => Err(Error::from_msg(
                        format!("A line cannot start with /{c}: Only //, /? and /* are supported"),
                        (slash_b, slash_b + 1 + c.len_utf8()),
                    )),
                }
            }
            Some(_) => normal_line(input, &mut chars),
        }
    }
}

impl Data {
    pub fn deserialize(input: &str) -> (Self, Vec<Error>) {
        let mut output = Self::default();
        let mut errors = vec![];

        for (i, line) in input.lines().enumerate() {
            match LineOutput::deserialize(line) {
                Err(mut err) => {
                    err.line = i;
                    errors.push(err);
                }
                Ok(LineOutput::Parameter {
                    value: Parameter::VeryRecent(x),
                    ..
                }) => {
                    output.very_recent = x;
                }
                Ok(LineOutput::Parameter {
                    value: Parameter::Recent(x),
                    ..
                }) => {
                    output.recent = x;
                }
                Ok(Prompt { contents, .. }) => {
                    output.prompts.push(contents);
                }
                Ok(LineOutput::Item { value: mut i, .. }) => {
                    if output.prompts.is_empty() {
                        output.prompts.push("$".into());
                    }
                    i.prompt_index = output.prompts.len() - 1;
                    output.questions.push(i);
                }
                Ok(Comment) => {}
            }
        }

        output.total_asked = 0;

        for q in &output.questions {
            debug_assert_eq!(q.question, q.question.trim(), "Questions have whitespace");
            debug_assert_eq!(q.answers, q.answers.trim(), "Answers have whitespace");
            output.total_asked += q.attempts;
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

        // return (output, errors);
        (output, errors)
    }
}

fn data_field(input: &str, chars: &mut Peekable<CharIndices<'_>>) -> Result<LineOutput, Error> {
    skip_whitespace(chars);
    let Some(ident_begin) = chars.peek().map(|c| c.0) else {
        return Err(Error::from_msg(
            "Line starting with /* has nothing after it",
            (input.len(), input.len()),
        ));
    };
    let ident = loop {
        let Some((i, c)) = chars.next() else {
            return Err(Error::from_msg(
                "Line starting with /* has no semicolon",
                (ident_begin, input.len()),
            ));
        };
        if c == '\n' {
            return Err(Error::from_msg(
                "Line starting with /* has no semicolon",
                (ident_begin, input.len()),
            ));
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
            return Err(Error::from_msg(
                "Line starting with /* has two semicolons",
                (i, i + 1),
            ));
        }
    };
    skip_whitespace(chars);
    match ident {
        "very_recent" => match value.parse() {
            Ok(v) => Ok(LineOutput::Parameter {
                value: Parameter::VeryRecent(v),
                key: (ident_begin, ident_begin + ident.len()),
                value_bytes: (value_begin, value_begin + value.len()),
            }),
            Err(err) => Err(Error::from_msg(
                format!("Couldn't parse {value:?}: {err}"),
                (value_begin, value_begin + value.len()),
            )),
        },
        "recent" => match value.parse() {
            Ok(v) => Ok(LineOutput::Parameter {
                value: Parameter::Recent(v),
                key: (ident_begin, ident_begin + ident.len()),
                value_bytes: (value_begin, value_begin + value.len()),
            }),
            Err(err) => Err(Error::from_msg(
                format!("Couldn't parse {value:?}: {err}"),
                (value_begin, value_begin + value.len()),
            )),
        },
        _ => Err(Error::from_msg(
            format!("Unkonwn identifier {ident:?}: Possible values are 'very_recent' and 'recent'"),
            (ident_begin, ident_begin + ident.len()),
        )),
    }
}

fn normal_line(input: &str, chars: &mut Peekable<CharIndices<'_>>) -> Result<LineOutput, Error> {
    skip_whitespace(chars);
    let q_start = chars.peek().expect("Can it be empty here?").0;
    let (question, q_end) = field(chars)?.expect("Can it be empty here?");
    skip_whitespace(chars);
    let a_start = chars
        .peek()
        .ok_or_else(|| Error::from_msg("Question without answer", (q_start, q_end)))?
        .0;

    let (answers, a_end) = field(chars)?.expect("Checked empty above");
    let mut output = Item {
        question,
        answers,
        ..Item::default()
    };

    output.attempts = field_number(input, chars)?.unwrap_or_default();
    output.errors = field_number(input, chars)?.unwrap_or_default();
    output.last_attempt = field_number(input, chars)?.and_then(|x| x.try_into().ok());
    output.streak = field_number(input, chars)?.unwrap_or(if output.errors == 0 {
        output.attempts
    } else {
        0
    });

    // println!("{output:#?}");

    Ok(LineOutput::Item {
        value: output,
        q_span: (q_start, q_end),
        a_span: (a_start, a_end),
    })
}

fn field_number(
    input: &str,
    chars: &mut Peekable<CharIndices<'_>>,
) -> Result<Option<usize>, Error> {
    let Some(&(beginning, _)) = chars.peek() else {
        return Ok(None);
    };
    let output = loop {
        let Some(&(i, c)) = chars.peek() else {
            break &input[beginning..];
        };
        if c == ',' {
            chars.next();
            break &input[beginning..i];
        }
        if c == '\n' {
            break &input[beginning..i];
        }
        chars.next();
    };

    match output.parse() {
        Err(err) => Err(Error::from_msg(
            format!("Failed to parse number {output:?}: {err}"),
            (beginning, beginning + output.len()),
        )),
        Ok(x) => Ok(Some(x)),
    }
}

fn field(chars: &mut Peekable<CharIndices<'_>>) -> Result<Option<(String, usize)>, Error> {
    let mut output = String::new();
    let mut last = 0;
    match chars.peek().copied() {
        None => Ok(None),
        Some((beginning, '"')) => field_in_quotes(chars, beginning),
        Some((_, _)) => loop {
            match chars.peek().copied() {
                None => {
                    return Ok(Some((trim_end(output), last)));
                }
                Some((i, ',')) => {
                    chars.next();
                    skip_whitespace(chars);
                    return Ok(Some((trim_end(output), i)));
                }
                Some((i, '"')) => {
                    return Err(Error::from_msg(
                        "Quotes are not allowed in questions or answers. Use '\\\"'",
                        (i, i + 1),
                    ));
                }
                Some((i, '\\')) => {
                    chars.next();
                    match chars.next() {
                        Some((i, c @ ('"' | '\\'))) => {
                            last = i;
                            output.push(c);
                        }
                        Some((_, 'n')) => {
                            return Err(Error::from_msg(
                                "New lines are not allowed in questions or answers",
                                (i, i + 2),
                            ));
                        }
                        None => {
                            return Err(Error::from_msg("Unfinished escape sequence", (i, i + 1)));
                        }
                        Some((_, c)) => {
                            return Err(Error::from_msg(
                                format!("Unsuppored escape sequence \\{c}"),
                                (i, i + 1 + c.len_utf8()),
                            ));
                        }
                    }
                }
                Some((_, c)) if c.is_whitespace() => {
                    chars.next();
                    output.push(c);
                }
                Some((i, c)) => {
                    chars.next();
                    last = i;
                    output.push(c);
                }
            }
        },
    }
}

fn trim_end(mut output: String) -> String {
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
    beginning: usize,
) -> Result<Option<(String, usize)>, Error> {
    let mut output = String::new();
    chars.next();
    skip_whitespace(chars);

    loop {
        let Some((i, c)) = chars.next() else {
            return Err(Error::from_msg("Unclosed \"", (beginning, beginning + 1)));
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
                        return Err(Error::from_msg(
                            "Expected ',' or new line",
                            (i, i + c.len_utf8()),
                        ));
                    }
                }
                return Ok(Some((trim_end(output), i + 1)));
            }
            '\n' => {
                return Err(Error::from_msg(
                    "Questions and answers must be a single line",
                    (beginning + 1, i),
                ));
            }
            '\\' => match chars.next() {
                Some((i, 'n')) => {
                    return Err(Error::from_msg(
                        "Questions and answers must be a single line",
                        (beginning + 1, i),
                    ));
                }
                Some((_, c @ ('\\' | '"' | '/'))) => {
                    output.push(c);
                }
                Some((_, c)) => {
                    return Err(Error::from_msg(
                        format!("Unsupported escape sequence \\{c})"),
                        (i, i + 1 + c.len_utf8()),
                    ));
                }
                None => {
                    return Err(Error::from_msg(
                        "Expected a character after '\\'",
                        (i, i + 1),
                    ));
                }
            },
            _ => {
                output.push(c);
            }
        }
    }
}

fn skip_whitespace(chars: &mut Peekable<CharIndices<'_>>) {
    loop {
        match chars.peek().copied() {
            Some((_, '\n')) => return,
            Some((_, c)) if c.is_whitespace() => {
                chars.next();
            }
            Some(_) | None => return,
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
            total_asked: 13_589_282_577_500,
            prompts: vec![String::new()],
            questions: vec![
                Item {
                    prompt_index: 0,
                    question: String::new(),
                    answers: "\\\0".into(),
                    attempts: 13_589_282_577_500,
                    errors: 6_081_631_320_437,
                    ..Default::default()
                },
                Item::default(),
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
        "q\n 0,,",
        "\n q,a",
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

#[test]
fn early_line_end() {
    const INPUT: &str = "q1,a
q2,a,
q3,a,0
q4,a,0,
q5,a,0,0
q6,a,0,0,0
q7,a,0,0,0,0
q8,a,0,0,0,0,
";

    let (data, errors) = Data::deserialize(INPUT);

    assert!(
        errors.is_empty(),
        "Got errors\n{errors:#?},\nStuff:\n{data:#?}"
    );

    assert_eq!(
        data,
        Data {
            very_recent: 0,
            recent: 0,
            total_asked: 0,
            prompts: vec!["$".into()],
            questions: vec![
                Item::new("q1", "a"),
                Item::new("q2", "a"),
                Item::new("q3", "a"),
                Item::new("q4", "a"),
                Item::new("q5", "a"),
                Item::new("q6", "a"),
                Item::new("q7", "a"),
                Item::new("q8", "a"),
            ]
        }
    );
}

#[derive(Debug)]
pub enum PromptPiece {
    Normal(usize, usize),
    Dollar,
}

struct PromptIter<'i> {
    input: &'i str,
    at: usize,
}

impl Iterator for PromptIter<'_> {
    type Item = PromptPiece;

    fn next(&mut self) -> Option<Self::Item> {
        let chars = self.input[self.at..].char_indices();
        let mut escaping = false;

        if '$' == self.input[self.at..].chars().next()? {
            self.at += 1;
            return Some(PromptPiece::Dollar);
        }

        for (i, c) in chars {
            match (c, escaping) {
                ('$', false) => {
                    self.at += i;
                    return Some(PromptPiece::Normal(self.at - i, self.at));
                }
                (_, true) => {
                    escaping = false;
                }
                ('\\', false) => escaping = true,
                (_, false) => {}
            }
        }
        let start = self.at;
        self.at = self.input.len();
        Some(PromptPiece::Normal(start, self.at))
    }
}

pub fn parse_prompt(prompt: &str) -> impl Iterator<Item = PromptPiece> + '_ {
    PromptIter {
        input: prompt,
        at: 0,
    }
}
