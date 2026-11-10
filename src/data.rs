use std::{cmp, collections::BTreeMap, fmt::Write, fs};

use Outcome::{Almost, Correct, Wrong};
use rand::{RngExt, rng};

use crate::strings::{Reuse, distance};

#[cfg(test)]
use pretty_assertions::assert_eq;

mod serde;

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Data {
    pub very_recent: usize,
    pub recent: usize,
    pub prompts: Vec<String>,
    pub questions: Vec<Item>,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
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
    pub(crate) fn check(&self, answer: &str) -> Outcome {
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
        item.prompt_string(&self.prompts[item.prompt_index])
    }

    pub fn get_item(&self, i: usize) -> &Item {
        &self.questions[i]
    }

    pub fn get_mut_item(&mut self, i: usize) -> &mut Item {
        &mut self.questions[i]
    }

    pub fn capitals() -> Self {
        let str = include_str!("data.quiz");

        let (output, errors) = Self::deserialize(str);
        debug_assert!(errors.is_empty());

        output
    }

    pub fn serialize(&self) -> String {
        let mut output = String::new();
        writeln!(
            &mut output,
            "/* very_recent: {}\n/* recent: {}",
            self.very_recent, self.recent
        )
        .unwrap();
        let mut last_prompt = None;
        for q in &self.questions {
            if last_prompt != Some(q.prompt_index) {
                writeln!(output, "/?{}", self.prompts[q.prompt_index]).unwrap();
                last_prompt = Some(q.prompt_index);
            }
            write_or_escape(&mut output, &q.question);
            write!(output, ",").unwrap();
            write_or_escape(&mut output, &q.answers);
            write!(output, ",").unwrap();
            writeln!(
                output,
                "{},{},{},{}",
                q.attempts, q.errors, q.last_attempt, q.streak
            )
            .unwrap();
        }
        output
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
            if i.last_attempt < self.recent {
                i.last_attempt += 1;
            }
        }
    }
}

fn write_or_escape(output: &mut String, item: &str) {
    if item.chars().any(|c| matches!(c, '"' | ',' | '\\' | '/')) {
        output.push('"');
        for c in item.chars() {
            match c {
                c @ ('"' | '\\' | '/') => {
                    output.push('\\');
                    output.push(c);
                }
                c => output.push(c),
            }
        }
        output.push('"');
    } else {
        output.push_str(item);
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

    let (deserialized, errors) = Data::deserialize(&serialized);

    assert!(
        errors.is_empty(),
        "Got errors parsing the following:\n{serialized}\nErrors:{errors:#?}"
    );

    assert_eq!(test, deserialized);

    let test_2 = r#"/* very_recent: 75
/* recent: 150
/?What is the capital of $?
Afghanistan,kabul,6,0,150,6
Albania,tirana,2,0,150,2
Comma country,"comma,city",1,1,147,1
/?Name the flag $:
🇦🇫,Afghanistan,9,1,84,2
🇦🇱,Albania,3,0,127,3
🇫🇮,Finland,2,0,150,2
🇿🇼,Zimbabwe,6,1,32,0
"#;

    let (deserialized, errors) = Data::deserialize(test_2);

    assert!(errors.is_empty(), "Got errors:\n{errors:#?}");

    assert_eq!(test_2, deserialized.serialize());
}
