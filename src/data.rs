use std::{
    cmp,
    collections::BTreeMap,
    fmt::{Display, Write},
    fs,
    str::FromStr,
};

use Outcome::{Almost, Correct, Wrong};
use anyhow::anyhow;
use rand::{RngExt, rng};

use crate::strings::{Reuse, distance};

#[derive(Debug, PartialEq, Eq)]
pub struct Data {
    pub very_recent: usize,
    pub recent: usize,
    pub prompts: Vec<String>,
    pub questions: Vec<Item>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Item {
    pub prompt_index: usize,
    pub question: String,
    pub answers: String,
    pub attempts: usize,
    pub errors: usize,
    pub last_attempt: usize,
    pub streak: usize,
}

pub enum Outcome {
    Correct,
    Almost,
    Wrong,
}

impl Item {
    pub fn check(&self, answer: &str) -> Outcome {
        let mut almost = false;
        for correct in self.answers.split('/') {
            let correct = correct.to_lowercase();
            if correct == answer {
                return Correct;
            }
            let bound = u8::try_from(correct.chars().count().div_ceil(5)).unwrap();
            if distance(&correct, answer, &mut Reuse::new()) <= bound {
                almost = true;
            }
        }
        if almost { Almost } else { Wrong }
    }

    // TODO
    #[allow(unused)]
    const fn score(&self) -> Score {
        Score {
            attempts: self.attempts,
            errors: self.errors,
        }
    }

    fn weight(&self, very_recent: usize, recent: usize) -> f32 {
        assert!(very_recent <= recent);
        if self.last_attempt <= very_recent {
            return 0.;
        }

        let mut output = 1.;

        let err = if self.attempts == 0 {
            1.
        } else {
            self.errors as f32 / self.attempts as f32
        };

        assert!(err.is_finite());

        output += err.sqrt() * 10.;

        if self.last_attempt < recent {
            output *= ((recent - self.last_attempt) as f32 / (recent - very_recent) as f32).powi(2);
        }

        if self.errors * 20 < self.attempts {
            output /= self.attempts as f32;
        }

        output /= self.streak as f32 + 1.;

        assert!(output.is_finite());

        output
    }

    pub fn prompt_string(&self, prompt: &str) -> String {
        let mut output = String::new();
        let mut first = true;
        for piece in prompt.split('$') {
            if !first {
                write!(output, "{}", self.question).unwrap();
            }
            first = false;
            write!(output, "{piece}").unwrap();
        }
        output
    }
}

impl Data {
    pub fn get_prompt(&self, i: usize) -> String {
        let item = &self.questions[i];
        let prompt = &self.prompts[item.prompt_index];
        item.prompt_string(prompt)
    }

    pub fn get_mut_item(&mut self, i: usize) -> &mut Item {
        &mut self.questions[i]
    }

    pub fn default() -> Self {
        let str = include_str!("data.quiz");

        Self::deserialize(str).unwrap()
    }

    pub fn serialize(&self) -> String {
        let mut output = String::new();
        writeln!(
            &mut output,
            "// Very recent: {}, Recent: {}",
            self.very_recent, self.recent
        )
        .unwrap();
        let mut last_prompt = None;
        for q in &self.questions {
            if last_prompt != Some(q.prompt_index) {
                writeln!(output, "//?{}", self.prompts[q.prompt_index]).unwrap();
                last_prompt = Some(q.prompt_index);
            }
            writeln!(
                output,
                "{},{},{},{},{},{}",
                q.question, q.answers, q.attempts, q.errors, q.last_attempt, q.streak
            )
            .unwrap();
        }
        output
    }

    pub fn deserialize(input: &str) -> anyhow::Result<Self> {
        let mut lines = input.lines();
        let recent_line = lines.next().ok_or_else(|| anyhow!("File is empty"))?;
        let strip_beginning = recent_line
            .strip_prefix("// Very recent: ")
            .ok_or_else(|| anyhow!("Expect first line to start with '// Very recent: '"))?;
        let (very_recent, recent) = strip_beginning
            .split_once(", Recent: ")
            .ok_or_else(|| anyhow!("Expect first line to contain ', Recent: '"))?;
        let very_recent = very_recent
            .parse()
            .map_err(|err| anyhow!("Failed to parse {very_recent}: {err}"))?;
        let recent = recent
            .parse()
            .map_err(|err| anyhow!("Failed to parse {recent}: {err}"))?;
        let mut prompts = vec![];
        let mut questions = vec![];
        for line in lines {
            if let Some(prompt) = line.strip_prefix("//?") {
                prompts.push(prompt.to_owned());
            } else {
                let mut pieces = line.split(',');

                let question = pieces
                    .next()
                    .ok_or_else(|| anyhow!("Line ends unexpectedly:\n{line}"))?;
                let answers = pieces
                    .next()
                    .ok_or_else(|| anyhow!("Line ends unexpectedly:\n{line}"))?;
                let attempts = parse_one_or(&mut pieces, 0)?;
                let errors = parse_one_or(&mut pieces, 0)?;
                let last_attempt = parse_one_or(&mut pieces, recent)?;
                let streak = parse_one_or(&mut pieces, if errors == 0 { attempts } else { 0 })?;

                questions.push(Item {
                    prompt_index: prompts
                        .len()
                        .checked_sub(1)
                        .ok_or_else(|| anyhow!("Found a question without prompt"))?,
                    question: question.into(),
                    answers: answers.into(),
                    attempts,
                    errors,
                    last_attempt,
                    streak,
                });
            }
        }

        Ok(Self {
            very_recent,
            recent,
            prompts,
            questions,
        })
    }

    pub fn _show_stats(&self) {
        const N_WORST: usize = 10;

        let mut worst: BTreeMap<(Score, &str), &Item> = BTreeMap::new();

        for item in &self.questions {
            if worst
                .iter()
                .nth(N_WORST)
                .is_some_and(|tenth| tenth.0.0 > item.score())
            {
                continue;
            }
            worst.insert((item.score(), &item.question), item);
            if worst.len() > N_WORST {
                worst.pop_last();
            }
        }

        println!("\nHere are your 10 worst questions");

        for item in worst.values() {
            println!(
                "{:>10}: {}/{} ({:.0}%)",
                item.question,
                item.attempts - item.errors,
                item.attempts,
                100. * (item.attempts - item.errors) as f32 / item.attempts as f32
            );
        }

        let mut counts: BTreeMap<Score, usize> = BTreeMap::new();

        let mut max_attempts = 0;

        for item in &self.questions {
            *counts.entry(item.score()).or_default() += 1;
            max_attempts = cmp::max(max_attempts, item.attempts);
        }

        let max_n = counts.values().max().copied().unwrap_or(1);

        let max_radius = 1. / (max_n as f32).sqrt();

        let mut graph = String::from(
            r"\documentclass[crop,tikz]{standalone}

\usepackage{tikz}

\begin{document}
\begin{tikzpicture}[>=stealth, fill = blue!20, draw = blue!50!black]
",
        );
        writeln!(
            &mut graph,
            "   \\def\\r{{{:.2}}}
   \\draw[line width = 0.1pt] (0,0) -- ({},{});\n",
            max_radius,
            max_attempts + 1,
            max_attempts + 1
        )
        .unwrap();

        for (score, n) in counts {
            let radius = (n as f32).sqrt();
            writeln!(
                &mut graph,
                "\\filldraw ({},{}) circle ({:.2} * \\r);",
                score.attempts,
                score.attempts - score.errors,
                radius,
            )
            .unwrap();
            // println!(
            //     "{}/{} ({:.0}%): {n}",
            //     score.attempts - score.errors,
            //     score.attempts,
            //     100. * (score.attempts - score.errors) as f32 / score.attempts as f32
            // );
        }

        writeln!(
            &mut graph,
            "
    \\draw[->] (0,0) -- ({},0);
    \\draw[->] (0,0) -- (0,{});

\\foreach \\x in {{1,...,{}}}
        \\draw (\\x,0.1) -- (\\x,-0.1);
    \\foreach \\y in {{1,...,{}}}
        \\draw (0.1,\\y) -- (-0.1,\\y);

\\end{{tikzpicture}}
\\end{{document}}",
            max_attempts + 1,
            max_attempts + 1,
            max_attempts + 1,
            max_attempts + 1
        )
        .unwrap();

        if let Err(err) = fs::write("graph.tex", graph) {
            println!("Error saving the graph: {err}");
        }
    }

    pub fn pick_one(&self) -> usize {
        let total_weight: f32 = self
            .questions
            .iter()
            .map(|i| i.weight(self.very_recent, self.recent))
            .sum();

        let chosen = rng().random_range(0. ..total_weight);

        let mut so_far = 0.;

        for (i, c) in self.questions.iter().enumerate() {
            so_far += c.weight(self.very_recent, self.recent);
            if so_far >= chosen {
                return i;
            }
        }

        unreachable!("Weight too big")
    }

    pub(crate) fn step_all(&mut self) {
        for i in &mut self.questions {
            if i.attempts < self.recent {
                i.attempts += 1;
            }
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
struct Score {
    attempts: usize,
    errors: usize,
}

impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Score {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (other.errors * self.attempts)
            .cmp(&(self.errors * other.attempts))
            .then(self.attempts.cmp(&other.attempts))
    }
}

#[test]
fn ser_deser() {
    let test = Data {
        very_recent: 10,
        recent: 20,
        prompts: vec!["question $?".into(), "Or $?".into()],
        questions: vec![
            Item {
                prompt_index: 0,
                question: "Afghanistan".into(),
                answers: "Kabul/kabul".into(),
                attempts: 1,
                errors: 0,
                last_attempt: 3,
                streak: 1,
            },
            Item {
                prompt_index: 1,
                question: "Ireland".into(),
                answers: "Dublin/Dopolin".into(),
                attempts: 3,
                errors: 2,
                last_attempt: 5,
                streak: 0,
            },
        ],
    };

    let serialized = test.serialize();

    let deserialized = Data::deserialize(&serialized).unwrap();

    assert_eq!(test, deserialized);

    let test_2 = "// Very recent: 75, Recent: 150
//?What is the capital of $?
Afghanistan,kabul,6,0,150,6
Albania,tirana,2,0,150,2
//?Name the flag $:
🇦🇫,Afghanistan,9,1,84,2
🇦🇱,Albania,3,0,127,3
🇫🇮,Finland,2,0,150,2
🇿🇼,Zimbabwe,6,1,32,0
";

    assert_eq!(test_2, Data::deserialize(test_2).unwrap().serialize());
}

fn parse_one_or<'s, T: FromStr>(
    pieces: &mut impl Iterator<Item = &'s str>,
    default: T,
) -> anyhow::Result<T>
where
    T::Err: Display,
{
    let Some(piece) = pieces.next() else {
        return Ok(default);
    };
    if piece.is_empty() {
        return Ok(default);
    }
    piece
        .parse()
        .map_err(|err| anyhow!("Failed to parse {piece:?}: {err}"))
}
