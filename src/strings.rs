use std::collections::HashMap;

pub struct Reuse {
    characters: Box<[u8; 256]>,
    nonascii: HashMap<char, u8>,
    distances: Vec<u8>,
    a_len: u8,
}

impl Reuse {
    fn dist(&mut self, x: isize, y: isize) -> &mut u8 {
        let i = usize::try_from(x + 1).unwrap()
            + (2 + usize::from(self.a_len)) * usize::try_from(y + 1).unwrap();
        &mut self.distances[i]
    }

    fn get(&self, c: char) -> u8 {
        if c.is_ascii() {
            self.characters[c as usize]
        } else {
            self.nonascii.get(&c).copied().unwrap_or_default()
        }
    }

    fn set(&mut self, c: char, i: u8) {
        if c.is_ascii() {
            self.characters[c as usize] = i;
        } else {
            self.nonascii.insert(c, i);
        }
    }

    pub fn new() -> Self {
        Self {
            characters: Box::new([0; 256]),
            nonascii: HashMap::new(),
            distances: vec![],
            a_len: 0,
        }
    }

    fn clear(&mut self, a_len: u8) {
        for c in &mut self.characters {
            *c = 0;
        }
        self.nonascii.clear();
        self.distances.clear();
        self.a_len = a_len;
    }
}

/// <https://en.wikipedia.org/wiki/Damerau%E2%80%93Levenshtein_distance>
pub fn distance(a: &str, b: &str, reuse: &mut Reuse) -> u8 {
    let a_len = u8::try_from(a.chars().count()).unwrap_or(u8::MAX);
    let b_len = u8::try_from(b.chars().count()).unwrap_or(u8::MAX);

    reuse.clear(a_len);

    reuse.distances.clear();
    for _ in 0..(usize::from(a_len) + 2) * (usize::from(b_len) + 2) {
        reuse.distances.push(0);
    }

    let maxdist = a_len.saturating_add(b_len);

    *reuse.dist(-1, -1) = maxdist;

    for i in 0..=a_len {
        *reuse.dist(i.into(), -1) = maxdist;
        *reuse.dist(i.into(), 0) = i;
    }
    for j in 0..=b_len {
        *reuse.dist(-1, j.into()) = maxdist;
        *reuse.dist(0, j.into()) = j;
    }

    for (i, a_i) in a
        .char_indices()
        .map(|(i, c)| (u8::try_from(i).unwrap() + 1, c))
        .take((u8::MAX - 1).into())
    {
        let mut last_b_eq_a_i = 0_u8;
        for (j, b_j) in b
            .char_indices()
            .map(|(j, c)| (u8::try_from(j).unwrap() + 1, c))
            .take((u8::MAX - 1).into())
        {
            let last_a_eq_b_j = reuse.get(b_j);
            let prev_b_eq_a_i = last_b_eq_a_i;
            let cost = if a_i == b_j {
                last_b_eq_a_i = j;
                0
            } else {
                1
            };

            let i_2 = isize::from(i);
            let j_2 = isize::from(j);

            debug_assert!(i > last_a_eq_b_j);
            debug_assert!(j > prev_b_eq_a_i);

            let to_min = [
                reuse.dist(i_2 - 1, j_2 - 1).saturating_add(cost), // Sub
                reuse.dist(i_2, j_2 - 1).saturating_add(1),        // Insert
                reuse.dist(i_2 - 1, j_2).saturating_add(1),        // Delete
                reuse
                    .dist(
                        isize::from(last_a_eq_b_j) - 1,
                        isize::from(prev_b_eq_a_i) - 1,
                    )
                    .saturating_add(i - last_a_eq_b_j - 1)
                    .saturating_add(cost)
                    .saturating_add(j - prev_b_eq_a_i - 1), // Flip
            ];

            let d = to_min.into_iter().min().unwrap();

            // This is wrong
            // if d >= bound {
            //     return bound;
            // }

            // println!(
            //     "
            // Distance between
            // {:?} and {:?}
            // is {d}. Options were
            // {to_min:?}",
            //     a.chars().take(i.into()).collect::<String>(),
            //     b.chars().take(j.into()).collect::<String>(),
            // );

            *reuse.dist(i_2, j_2) = d;
        }

        reuse.set(a_i, i);
    }

    *reuse.dist(a_len.into(), b_len.into())
}

#[test]
fn test() {
    let cases = &[
        ("italy", "hungary", 5),
        ("abc", "bca", 2),
        ("abcd", "bcda", 2),
        ("kiev", "keiv", 1),
        ("a", "b", 1),
        ("a", "a", 0),
        ("ab", "ba", 1),
        ("abc", "cba", 2),
        (
            "st vincent and the grenadines",
            "saint vincent and the grenadines",
            3,
        ),
    ];

    let mut reuse = Reuse::new();

    for (a, b, result) in cases {
        assert_eq!(
            distance(a, b, &mut reuse),
            *result,
            "Computing d({a:?}, {b:?}, {result})",
        );
    }
}
