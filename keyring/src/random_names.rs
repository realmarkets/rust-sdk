// Copyright (c) Mysten Labs, Inc.
// Modifications Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Random `{adjective}-{stone}` aliases for keys added without one, drawn from
//! the same word lists as the reference crate so generated names look the same.

use std::collections::HashSet;

use rand::{RngExt, rng, rngs::ThreadRng};

/// A list of adjectives
const LEFT_NAMES: [&str; 108] = [
    "admiring",
    "adoring",
    "affectionate",
    "agitated",
    "amazing",
    "angry",
    "awesome",
    "beautiful",
    "blissful",
    "bold",
    "boring",
    "brave",
    "busy",
    "charming",
    "clever",
    "compassionate",
    "competent",
    "condescending",
    "confident",
    "cool",
    "cranky",
    "crazy",
    "dazzling",
    "determined",
    "distracted",
    "dreamy",
    "eager",
    "ecstatic",
    "elastic",
    "elated",
    "elegant",
    "eloquent",
    "epic",
    "exciting",
    "fervent",
    "festive",
    "flamboyant",
    "focused",
    "friendly",
    "frosty",
    "funny",
    "gallant",
    "gifted",
    "goofy",
    "gracious",
    "great",
    "happy",
    "hardcore",
    "heuristic",
    "hopeful",
    "hungry",
    "infallible",
    "inspiring",
    "intelligent",
    "interesting",
    "jolly",
    "jovial",
    "keen",
    "kind",
    "laughing",
    "loving",
    "lucid",
    "magical",
    "modest",
    "musing",
    "mystifying",
    "naughty",
    "nervous",
    "nice",
    "nifty",
    "nostalgic",
    "objective",
    "optimistic",
    "peaceful",
    "pedantic",
    "pensive",
    "practical",
    "priceless",
    "quirky",
    "quizzical",
    "recursing",
    "relaxed",
    "reverent",
    "romantic",
    "sad",
    "serene",
    "sharp",
    "silly",
    "sleepy",
    "stoic",
    "strange",
    "stupefied",
    "suspicious",
    "sweet",
    "tender",
    "thirsty",
    "trusting",
    "unruffled",
    "upbeat",
    "vibrant",
    "vigilant",
    "vigorous",
    "wizardly",
    "wonderful",
    "xenodochial",
    "youthful",
    "zealous",
    "zen",
];

/// A list of precious stones
const RIGHT_NAMES: [&str; 53] = [
    "agates",
    "alexandrite",
    "amber",
    "amethyst",
    "apatite",
    "avanturine",
    "axinite",
    "beryl",
    "beryl",
    "carnelian",
    "chalcedony",
    "chrysoberyl",
    "chrysolite",
    "chrysoprase",
    "coral",
    "corundum",
    "crocidolite",
    "cyanite",
    "cymophane",
    "diamond",
    "dichroite",
    "emerald",
    "epidote",
    "euclase",
    "felspar",
    "garnet",
    "heliotrope",
    "hematite",
    "hiddenite",
    "hypersthene",
    "idocrase",
    "jasper",
    "jet",
    "labradorite",
    "malachite",
    "moonstone",
    "obsidian",
    "opal",
    "pearl",
    "phenacite",
    "plasma",
    "prase",
    "quartz",
    "ruby",
    "sapphire",
    "sphene",
    "spinel",
    "spodumene",
    "sunstone",
    "topaz",
    "tourmaline",
    "turquois",
    "zircon",
];

/// Random draws tried before falling back to an exhaustive scan. Far above
/// what a sparse conflict set needs, yet keeps a near-full one bounded.
const RANDOM_ATTEMPTS: usize = 1_000;

/// Return a random name formatted as `adjective-stone` that is not in
/// `conflicts`.
///
/// The reference crate loops forever once every combination is taken. This draws at
/// random first, then scans every combination in order, then appends `-2`,
/// `-3`, … to a combination, so it always returns.
pub fn random_name(conflicts: &HashSet<String>) -> String {
    let mut rng = rng();
    for _ in 0..RANDOM_ATTEMPTS {
        let output = rng.random_alias();
        if !conflicts.contains(&output) {
            return output;
        }
    }

    if let Some(free) = all_combinations().find(|name| !conflicts.contains(name)) {
        return free;
    }

    return (2u64..)
        .map(|suffix| format!("{}-{}-{suffix}", LEFT_NAMES[0], RIGHT_NAMES[0]))
        .find(|name| !conflicts.contains(name))
        .expect("the suffix range is unbounded and a finite set excludes finitely many names");
}

/// Return `output_size` names, unique among themselves and absent from
/// `conflicts`.
pub fn random_names(mut conflicts: HashSet<String>, output_size: usize) -> Vec<String> {
    let mut names = Vec::with_capacity(output_size);
    names.resize_with(output_size, || {
        let name = random_name(&conflicts);
        conflicts.insert(name.clone());
        name
    });
    return names;
}

trait RandomAlias {
    /// One `adjective-stone` pair, each word drawn uniformly from its list.
    fn random_alias(&mut self) -> String;
}

impl RandomAlias for ThreadRng {
    fn random_alias(&mut self) -> String {
        let left = LEFT_NAMES[self.random_range(0..LEFT_NAMES.len())];
        let right = RIGHT_NAMES[self.random_range(0..RIGHT_NAMES.len())];
        return format!("{left}-{right}");
    }
}

fn all_combinations() -> impl Iterator<Item = String> {
    return LEFT_NAMES.iter().flat_map(|left| {
        RIGHT_NAMES
            .iter()
            .map(move |right| format!("{left}-{right}"))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_name_avoids_conflicts() {
        let conflicts: HashSet<String> = all_combinations().skip(1).collect();
        assert_eq!(random_name(&conflicts), "admiring-agates");
    }

    #[test]
    fn random_name_terminates_once_every_combination_is_taken() {
        let conflicts: HashSet<String> = all_combinations().collect();
        assert_eq!(random_name(&conflicts), "admiring-agates-2");
    }

    #[test]
    fn random_names_are_unique_among_themselves() {
        let names = random_names(HashSet::new(), 200);
        let unique: HashSet<&String> = names.iter().collect();
        assert_eq!(unique.len(), 200);
    }

    #[test]
    fn names_match_the_upstream_shape() {
        let name = random_name(&HashSet::new());
        let (left, right) = name.split_once('-').unwrap();
        assert!(
            LEFT_NAMES.contains(&left),
            "{left} is not a listed adjective"
        );
        assert!(
            RIGHT_NAMES.contains(&right),
            "{right} is not a listed stone"
        );
    }

    // the stone list carries a duplicate `beryl`, as upstream does, so 5,616
    // distinct names exist rather than 108 * 53
    #[test]
    fn distinct_combination_count_matches_upstream() {
        let distinct: HashSet<String> = all_combinations().collect();
        assert_eq!(distinct.len(), 5_616);
    }
}
