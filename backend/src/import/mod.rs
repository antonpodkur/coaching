//! Parser for workout plans Dasha used to send as Telegram messages. It is only
//! used by the one-off import when a client moves into the app; new plans are
//! built in the builder.
//!
//! Format: an exercise name on its own line, then one line per set, e.g.
//! `79 кг х 12`, `79 кг х 8-10`, `х 17` or just `17` for bodyweight. A set line
//! may end with words such as `на кожну руку`. A line in parentheses is a note
//! for the exercise above it; parentheses after a name are a note too.

use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedPlan {
    pub exercises: Vec<ParsedExercise>,
    /// Lines that could not be read, shown to the coach to fix.
    pub unparsed: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedExercise {
    pub name: String,
    /// Her own wording, e.g. `на кожну руку`, when every set carries it.
    pub per_side_label: Option<String>,
    pub note: Option<String>,
    pub sets: Vec<ParsedSet>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedSet {
    /// `None` for bodyweight.
    pub kg: Option<f64>,
    pub reps_min: u32,
    pub reps_max: u32,
}

// `х` is Cyrillic, `x` Latin; she uses both, and `×` appears when copied from the app.
static WEIGHTED_SET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(\d+(?:[.,]\d+)?)\s*кг\s*[xх×]\s*(\d+)(?:\s*-\s*(\d+))?(?:\s+(.*))?$")
        .expect("valid regex")
});
static REPS_ONLY_SET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:[xх×]\s*)?(\d+)(?:\s*-\s*(\d+))?(?:\s+(.*))?$").expect("valid regex")
});
static NOTE_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\((.*)\)$").expect("valid regex"));
static NAME_WITH_NOTE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.*?)\s*(?:\((.*)\))?$").expect("valid regex"));

/// Lower-cased with single spaces, for matching names against the library.
pub fn normalize_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub fn parse_plan(text: &str) -> ParsedPlan {
    let mut drafts: Vec<Draft> = Vec::new();
    let mut unparsed = Vec::new();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some(caps) = NOTE_LINE.captures(line) {
            match drafts.last_mut() {
                Some(draft) => draft.notes.push(caps[1].trim().to_owned()),
                None => unparsed.push(line.to_owned()),
            }
            continue;
        }
        if let Some(set) = parse_set(line) {
            match (drafts.last_mut(), set) {
                (Some(draft), Some(set)) => draft.sets.push(set),
                _ => unparsed.push(line.to_owned()),
            }
            continue;
        }
        let caps = NAME_WITH_NOTE
            .captures(line)
            .expect("pattern matches any line");
        drafts.push(Draft {
            name: caps[1].to_owned(),
            notes: caps
                .get(2)
                .map(|note| vec![note.as_str().trim().to_owned()])
                .unwrap_or_default(),
            sets: Vec::new(),
        });
    }

    ParsedPlan {
        exercises: drafts.into_iter().map(Draft::finish).collect(),
        unparsed,
    }
}

struct Draft {
    name: String,
    notes: Vec<String>,
    sets: Vec<(ParsedSet, String)>,
}

impl Draft {
    fn finish(self) -> ParsedExercise {
        let extras: Vec<&str> = self.sets.iter().map(|(_, extra)| extra.as_str()).collect();
        let shared = extras
            .first()
            .filter(|first| !first.is_empty() && extras.iter().all(|extra| extra == *first))
            .map(|first| (*first).to_owned());

        let mut notes = self.notes;
        let per_side_label = match shared {
            Some(extra) if extra.to_lowercase().contains("кожн") => Some(extra),
            Some(extra) => {
                notes.insert(0, extra);
                None
            }
            None => {
                // Different words on different sets: keep them, numbered.
                for (index, extra) in extras.iter().enumerate() {
                    if !extra.is_empty() {
                        notes.push(format!("підхід {}: {extra}", index + 1));
                    }
                }
                None
            }
        };

        ParsedExercise {
            name: self.name,
            per_side_label,
            note: (!notes.is_empty()).then(|| notes.join(" · ")),
            sets: self.sets.into_iter().map(|(set, _)| set).collect(),
        }
    }
}

/// `None` if the line is not a set line; `Some(None)` if it looks like one but
/// the numbers make no sense (e.g. `12-8` or `0`).
fn parse_set(line: &str) -> Option<Option<(ParsedSet, String)>> {
    let (kg, min, max, extra) = if let Some(caps) = WEIGHTED_SET.captures(line) {
        (
            Some(caps.get(1)?.as_str()),
            caps.get(2)?.as_str(),
            caps.get(3).map(|m| m.as_str()),
            caps.get(4).map(|m| m.as_str()),
        )
    } else {
        let caps = REPS_ONLY_SET.captures(line)?;
        (
            None,
            caps.get(1)?.as_str(),
            caps.get(2).map(|m| m.as_str()),
            caps.get(3).map(|m| m.as_str()),
        )
    };

    let set = (|| {
        let reps_min: u32 = min.parse().ok()?;
        let reps_max: u32 = max.map_or(Some(reps_min), |max| max.parse().ok())?;
        if reps_min == 0 || reps_min > reps_max {
            return None;
        }
        let kg = match kg {
            Some(kg) => Some(kg.replace(',', ".").parse::<f64>().ok()?),
            None => None,
        };
        let set = ParsedSet {
            kg,
            reps_min,
            reps_max,
        };
        Some((set, extra.unwrap_or_default().trim().to_owned()))
    })();
    Some(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(kg: Option<f64>, reps_min: u32, reps_max: u32) -> ParsedSet {
        ParsedSet {
            kg,
            reps_min,
            reps_max,
        }
    }

    #[test]
    fn reads_her_back_workout() {
        let plan = parse_plan(include_str!("../../tests/fixtures/example-back.txt"));
        assert!(plan.unparsed.is_empty(), "unparsed: {:?}", plan.unparsed);

        let names: Vec<_> = plan.exercises.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Підтягування",
                "Вертикальна тяга вузьким хватом",
                "Вертикальна тяга широким хватом",
                "Тяга гантелі в нахилі",
                "Горизонтальна тяга прямим хватом",
                "Зворотній метелик",
                "Згинання штанги на біцепс",
            ]
        );
        let total_sets: usize = plan.exercises.iter().map(|e| e.sets.len()).sum();
        assert_eq!(total_sets, 20);

        let pull_ups = &plan.exercises[0];
        assert_eq!(
            pull_ups.sets,
            [set(None, 17, 17), set(None, 14, 14), set(None, 12, 12)]
        );

        let narrow = &plan.exercises[1];
        assert_eq!(narrow.sets[0], set(Some(45.0), 15, 15));

        let wide = &plan.exercises[2];
        assert!(wide.sets.iter().all(|s| *s == set(Some(79.0), 8, 10)));

        let rows = &plan.exercises[3];
        assert_eq!(rows.per_side_label.as_deref(), Some("на кожну руку"));
        assert_eq!(rows.sets[2], set(Some(28.0), 15, 15));
        assert_eq!(rows.note, None);

        let fly = &plan.exercises[5];
        assert!(
            fly.note
                .as_deref()
                .unwrap()
                .starts_with("Зворотній метелик і горизонтальна тяга")
        );

        let curls = &plan.exercises[6];
        assert_eq!(
            curls.note.as_deref(),
            Some("зігнутий гриф +10 кг з кожної сторони")
        );
        assert_eq!(curls.sets, [set(None, 17, 17), set(None, 17, 17)]);
    }

    #[test]
    fn accepts_latin_x_decimal_comma_and_no_spaces() {
        let plan = parse_plan("Жим ногами\n120,5кг x10\n100 КГ × 12-15");
        assert_eq!(
            plan.exercises[0].sets,
            [set(Some(120.5), 10, 10), set(Some(100.0), 12, 15)]
        );
    }

    #[test]
    fn a_name_starting_with_a_digit_is_still_a_name() {
        let plan = parse_plan("3D-тяга\n10");
        assert_eq!(plan.exercises[0].name, "3D-тяга");
        assert_eq!(plan.exercises[0].sets, [set(None, 10, 10)]);
    }

    #[test]
    fn reports_lines_it_cannot_place() {
        let plan = parse_plan("(нотатка без вправи)\n12\nПрисідання\n12-8\n60 кг х 8");
        assert_eq!(plan.unparsed, ["(нотатка без вправи)", "12", "12-8"]);
        assert_eq!(plan.exercises[0].sets, [set(Some(60.0), 8, 8)]);
    }

    #[test]
    fn keeps_differing_set_words_as_numbered_notes() {
        let plan = parse_plan("Випади\n14 кг х 10 ліва нога попереду\n14 кг х 10");
        assert_eq!(plan.exercises[0].per_side_label, None);
        assert_eq!(
            plan.exercises[0].note.as_deref(),
            Some("підхід 1: ліва нога попереду")
        );
    }

    #[test]
    fn normalizes_names_for_matching() {
        assert_eq!(
            normalize_name("  Тяга  гантелі в НАХИЛІ "),
            "тяга гантелі в нахилі"
        );
    }
}
