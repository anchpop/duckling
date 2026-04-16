use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
#[cfg(not(debug_assertions))]
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::document::Document;
use crate::resolve::{Context, Options};
use crate::stash::Stash;
use crate::types::{
    DimensionKind, Entity, Node, PatternItem, Range, RegexMatchData, Rule, TokenData,
};

type RegexMatches = Vec<(Range, Vec<Option<String>>)>;
type SeenKey = (usize, usize, Option<String>, u64);

/// Cache for regex evaluations in match_remaining, keyed by (pattern_string, after_pos).
/// Two-level HashMap so cache hits only need a `&str` borrow (no String allocation).
/// This deduplicates across different Rule objects that share the same regex pattern,
/// which is common (e.g. `\bof|in\b` appears in 7 rules, `\bthe\b` in 7, etc.).
type PositionRegexCache = HashMap<String, HashMap<usize, Option<(Range, Vec<Option<String>>)>>>;

#[derive(Debug, Clone, Copy)]
struct ParseLimits {
    max_regex_matches_per_rule: usize,
    max_rule_results: usize,
    max_new_nodes_per_iteration: usize,
    max_nodes: usize,
    max_iterations: usize,
}

impl ParseLimits {
    fn for_text_len(_text_len: usize) -> Self {
        Self {
            max_regex_matches_per_rule: 256,
            max_rule_results: 256,
            max_new_nodes_per_iteration: 1_024,
            max_nodes: 3_000,
            max_iterations: 24,
        }
    }
}

/// Cached RegexSet built from all unique regex patterns in a rule set.
/// Used as a negative filter: if a pattern doesn't match anywhere in the text,
/// we can skip it everywhere (first-pattern cache + match_remaining).
struct CachedRegexSet {
    set: regex::RegexSet,
    /// Map from pattern string → index in the RegexSet, for O(1) match checks.
    pattern_to_idx: HashMap<String, usize>,
}

/// Global cache of RegexSets, keyed by rules slice pointer.
/// Safe because rules are `&'static [Rule]` (leaked by `lang::rules_for`).
static REGEX_SET_CACHE: Lazy<Mutex<HashMap<usize, &'static CachedRegexSet>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn get_or_build_regex_set(rules: &[Rule]) -> &'static CachedRegexSet {
    let key = rules.as_ptr() as usize;
    {
        let cache = REGEX_SET_CACHE.lock().unwrap();
        if let Some(cached) = cache.get(&key) {
            return cached;
        }
    }

    // Collect unique regex patterns across all rule positions
    let mut patterns: Vec<String> = Vec::new();
    let mut pattern_to_idx: HashMap<String, usize> = HashMap::new();
    for rule in rules {
        for item in &rule.pattern {
            if let PatternItem::Regex(re) = item {
                let pat = re.as_str().to_string();
                pattern_to_idx.entry(pat).or_insert_with_key(|pat| {
                    let idx = patterns.len();
                    patterns.push(pat.clone());
                    idx
                });
            }
        }
    }

    let set = regex::RegexSet::new(&patterns).expect("all patterns should be valid regexes");
    let cached = Box::leak(Box::new(CachedRegexSet {
        set,
        pattern_to_idx,
    }));

    REGEX_SET_CACHE.lock().unwrap().insert(key, cached);
    cached
}

/// Parse text and resolve all entities.
#[allow(dead_code)]
pub fn parse_and_resolve(
    text: &str,
    rules: &[Rule],
    context: &Context,
    options: &Options,
    dims: &[DimensionKind],
) -> Vec<Entity> {
    use crate::dimensions::time::series::{Budget, DEFAULT_WORK_BUDGET};

    let stash = parse_string(text, rules);
    let doc_text = text;
    let mut budget = Budget::new(DEFAULT_WORK_BUDGET);

    let mut entities: Vec<Entity> = Vec::new();
    for node in stash.all_nodes() {
        if let Some(dk) = node.token_data.dimension_kind() {
            if !dims.is_empty() && !dims.contains(&dk) {
                continue;
            }
        } else {
            continue;
        }
        if let Some(entity) = crate::resolve::resolve(node, context, options, doc_text, &mut budget)
        {
            entities.push(entity);
        }
    }

    entities
}

/// Parse text and resolve all entities, returning (Node, Entity) pairs.
/// Used by the training pipeline to access both the parse tree and resolved value.
#[cfg(feature = "train")]
pub fn parse_and_resolve_with_nodes(
    text: &str,
    rules: &[Rule],
    context: &Context,
    options: &Options,
    dims: &[DimensionKind],
) -> Vec<(Node, Entity)> {
    use crate::dimensions::time::series::{Budget, DEFAULT_WORK_BUDGET};

    let stash = parse_string(text, rules);
    let doc_text = text;
    let mut budget = Budget::new(DEFAULT_WORK_BUDGET);

    let mut results: Vec<(Node, Entity)> = Vec::new();
    for node in stash.all_nodes() {
        if let Some(dk) = node.token_data.dimension_kind() {
            if !dims.is_empty() && !dims.contains(&dk) {
                continue;
            }
        } else {
            continue;
        }
        if let Some(entity) = crate::resolve::resolve(node, context, options, doc_text, &mut budget)
        {
            results.push((node.clone(), entity));
        }
    }

    results
}

/// Run the saturation-based parsing loop.
pub fn parse_string(text: &str, rules: &[Rule]) -> Stash {
    let doc = Document::new(text);
    let mut stash = Stash::new();
    let limits = ParseLimits::for_text_len(text.len());

    // Use a cached RegexSet to quickly determine which patterns match anywhere
    // in the text. Patterns that don't match at all can be skipped everywhere.
    let regex_set = get_or_build_regex_set(rules);
    let set_matches = regex_set.set.matches(text);

    // Pre-compute regex matches for all regex-leading rules once.
    // Skip patterns the RegexSet says don't match anywhere.
    // Deduplicate by pattern string so shared patterns only run once.
    let mut pattern_cache: HashMap<String, RegexMatches> = HashMap::new();
    let regex_cache: Vec<Option<RegexMatches>> = rules
        .iter()
        .map(|rule| {
            if rule.pattern.is_empty() {
                return None;
            }
            if let PatternItem::Regex(ref re) = rule.pattern[0] {
                let pat = re.as_str();
                // Skip if RegexSet says this pattern doesn't match anywhere
                if let Some(&idx) = regex_set.pattern_to_idx.get(pat) {
                    if !set_matches.matched(idx) {
                        return Some(Vec::new());
                    }
                }
                let key = pat.to_string();
                Some(
                    pattern_cache
                        .entry(key)
                        .or_insert_with(|| {
                            find_regex_matches(&doc, re, limits.max_regex_matches_per_rule)
                        })
                        .clone(),
                )
            } else {
                None
            }
        })
        .collect();
    drop(pattern_cache);

    // Track seen nodes to deduplicate while preserving alternative parses
    // with different token payloads at the same span/rule.
    let mut seen: HashSet<SeenKey> = HashSet::new();

    // Phase 1: Apply all regex-leading rules to find initial tokens
    let initial = apply_regex_rules(rules, &regex_cache, &limits);
    for node in initial.all_nodes() {
        if seen.len() >= limits.max_nodes {
            break;
        }
        let key = dedup_key(node);
        seen.insert(key);
    }
    stash.merge_from(initial);

    // Cache for regex evaluations in match_remaining so the same regex at the same
    // position is never run twice across different rules or iterations.
    let mut pos_cache = PositionRegexCache::new();

    // Phase 2: Saturation loop - keep applying rules until no new tokens
    let mut iterations = 0usize;
    loop {
        if iterations >= limits.max_iterations || seen.len() >= limits.max_nodes {
            break;
        }
        iterations = iterations.saturating_add(1);
        let new_stash = apply_all_rules(
            &doc,
            rules,
            &stash,
            &regex_cache,
            &seen,
            &limits,
            &mut pos_cache,
        );
        let mut actually_new = Stash::new();
        for node in new_stash.into_nodes() {
            if seen.len() >= limits.max_nodes {
                break;
            }
            let key = dedup_key(&node);
            if seen.insert(key) {
                actually_new.add(node);
            }
        }
        if actually_new.is_empty() {
            break;
        }
        stash.merge_from(actually_new);
    }

    stash
}

fn dedup_key(node: &Node) -> SeenKey {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    // Hash the Debug representation without allocating a String
    struct HashWriter<'a>(&'a mut DefaultHasher);
    impl std::fmt::Write for HashWriter<'_> {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            s.hash(self.0);
            Ok(())
        }
    }
    let _ = std::fmt::Write::write_fmt(
        &mut HashWriter(&mut hasher),
        format_args!("{:?}", node.token_data),
    );
    (
        node.range.start,
        node.range.end,
        node.rule_name.clone(),
        hasher.finish(),
    )
}

fn safe_production(rule: &Rule, nodes: &[&Node]) -> Option<TokenData> {
    #[cfg(debug_assertions)]
    {
        (rule.production)(nodes)
    }

    #[cfg(not(debug_assertions))]
    {
        catch_unwind(AssertUnwindSafe(|| (rule.production)(nodes)))
            .ok()
            .flatten()
    }
}

/// Apply regex-leading rules to the document to find initial tokens.
/// Uses pre-computed regex cache.
fn apply_regex_rules(
    rules: &[Rule],
    regex_cache: &[Option<RegexMatches>],
    limits: &ParseLimits,
) -> Stash {
    let mut stash = Stash::new();

    for (i, rule) in rules.iter().enumerate() {
        if stash.len() >= limits.max_nodes {
            break;
        }
        if rule.pattern.is_empty() {
            continue;
        }
        if let PatternItem::Regex(_) = &rule.pattern[0] {
            let matches = match &regex_cache[i] {
                Some(m) => m,
                None => continue,
            };
            for (range, groups) in matches {
                if stash.len() >= limits.max_nodes {
                    break;
                }
                let regex_node = Node {
                    range: *range,
                    token_data: TokenData::RegexMatch(RegexMatchData {
                        groups: groups.clone(),
                    }),
                    children: Vec::new(),
                    rule_name: None,
                };

                if rule.pattern.len() == 1 {
                    // Single-pattern rule: produce directly
                    if let Some(token_data) = safe_production(rule, &[&regex_node]) {
                        let mut node = Node::new(*range, token_data);
                        node.rule_name = Some(rule.name.clone());
                        node.children = vec![Rc::new(regex_node)];
                        stash.add(node);
                    }
                } else {
                    // Multi-pattern rule: store the regex match for later combination
                    stash.add(regex_node);
                }
            }
        }
    }

    stash
}

/// Apply all rules against the current stash to find new tokens.
/// Skips single-pattern regex rules (already fully handled in phase 1).
fn apply_all_rules(
    doc: &Document,
    rules: &[Rule],
    stash: &Stash,
    regex_cache: &[Option<RegexMatches>],
    seen: &HashSet<SeenKey>,
    limits: &ParseLimits,
    pos_cache: &mut PositionRegexCache,
) -> Stash {
    let mut new_stash = Stash::new();
    let mut local_seen: HashSet<SeenKey> = HashSet::new();

    for (i, rule) in rules.iter().enumerate() {
        if new_stash.len() >= limits.max_new_nodes_per_iteration
            || seen.len().saturating_add(new_stash.len()) >= limits.max_nodes
        {
            break;
        }
        // Skip single-pattern regex rules - already fully handled in phase 1
        if rule.pattern.len() == 1 {
            if let PatternItem::Regex(_) = &rule.pattern[0] {
                continue;
            }
        }
        let cached = regex_cache[i].as_ref();
        let produced = match_rule(doc, rule, stash, cached, limits, pos_cache);
        for node in produced {
            let key = dedup_key(&node);
            if seen.contains(&key) || !local_seen.insert(key) {
                continue;
            }
            new_stash.add(node);
            if new_stash.len() >= limits.max_new_nodes_per_iteration
                || seen.len().saturating_add(new_stash.len()) >= limits.max_nodes
            {
                break;
            }
        }
    }

    new_stash
}

/// Try to match a rule against the document and stash, producing new nodes.
/// Uses cached regex matches when available.
fn match_rule(
    doc: &Document,
    rule: &Rule,
    stash: &Stash,
    cached_regex: Option<&RegexMatches>,
    limits: &ParseLimits,
    pos_cache: &mut PositionRegexCache,
) -> Vec<Node> {
    let mut results = Vec::new();

    if rule.pattern.is_empty() {
        return results;
    }

    match &rule.pattern[0] {
        PatternItem::Regex(_) => {
            // Use cached regex matches
            let matches = match cached_regex {
                Some(m) => m,
                None => return results,
            };
            for (range, groups) in matches {
                if results.len() >= limits.max_rule_results {
                    break;
                }
                let regex_node = Node {
                    range: *range,
                    token_data: TokenData::RegexMatch(RegexMatchData {
                        groups: groups.clone(),
                    }),
                    children: Vec::new(),
                    rule_name: None,
                };

                if rule.pattern.len() == 1 {
                    if let Some(token_data) = safe_production(rule, &[&regex_node]) {
                        let mut node = Node::new(*range, token_data);
                        node.rule_name = Some(rule.name.clone());
                        node.children = vec![Rc::new(regex_node)];
                        results.push(node);
                    }
                } else {
                    // Continue matching remaining patterns
                    let continuations = match_remaining(
                        doc,
                        rule,
                        stash,
                        1,
                        range.end,
                        vec![Rc::new(regex_node)],
                        limits,
                        pos_cache,
                    );
                    extend_with_limit(&mut results, continuations, limits.max_rule_results);
                }
            }
        }
        PatternItem::Dimension(dim) => {
            // Start with dimension matches from the stash
            for node in stash.all_nodes() {
                if results.len() >= limits.max_rule_results {
                    break;
                }
                if node.token_data.dimension_kind() == Some(*dim) {
                    if rule.pattern.len() == 1 {
                        if let Some(token_data) = safe_production(rule, &[node]) {
                            let mut new_node = Node::new(node.range, token_data);
                            new_node.rule_name = Some(rule.name.clone());
                            new_node.children = vec![Rc::new(node.clone())];
                            results.push(new_node);
                        }
                    } else {
                        let continuations = match_remaining(
                            doc,
                            rule,
                            stash,
                            1,
                            node.range.end,
                            vec![Rc::new(node.clone())],
                            limits,
                            pos_cache,
                        );
                        extend_with_limit(&mut results, continuations, limits.max_rule_results);
                    }
                }
            }
        }
        PatternItem::Predicate(pred) => {
            for node in stash.all_nodes() {
                if results.len() >= limits.max_rule_results {
                    break;
                }
                if pred(&node.token_data) {
                    if rule.pattern.len() == 1 {
                        if let Some(token_data) = safe_production(rule, &[node]) {
                            let mut new_node = Node::new(node.range, token_data);
                            new_node.rule_name = Some(rule.name.clone());
                            new_node.children = vec![Rc::new(node.clone())];
                            results.push(new_node);
                        }
                    } else {
                        let continuations = match_remaining(
                            doc,
                            rule,
                            stash,
                            1,
                            node.range.end,
                            vec![Rc::new(node.clone())],
                            limits,
                            pos_cache,
                        );
                        extend_with_limit(&mut results, continuations, limits.max_rule_results);
                    }
                }
            }
        }
    }

    results
}

fn extend_with_limit<T>(dst: &mut Vec<T>, mut src: Vec<T>, limit: usize) {
    let remaining = limit.saturating_sub(dst.len());
    if remaining == 0 {
        return;
    }
    if src.len() > remaining {
        src.truncate(remaining);
    }
    dst.extend(src);
}

/// Continue matching the remaining pattern items after a partial match.
/// Uses position-filtered stash iteration for efficiency.
/// Uses `pos_cache` to memoize regex evaluations by (pattern, position).
#[allow(clippy::too_many_arguments)]
fn match_remaining(
    doc: &Document,
    rule: &Rule,
    stash: &Stash,
    pattern_idx: usize,
    after_pos: usize,
    mut matched_so_far: Vec<Rc<Node>>,
    limits: &ParseLimits,
    pos_cache: &mut PositionRegexCache,
) -> Vec<Node> {
    let mut results = Vec::new();

    if pattern_idx >= rule.pattern.len() {
        // All patterns matched - produce the result
        let refs: Vec<&Node> = matched_so_far.iter().map(|rc| rc.as_ref()).collect();
        if let Some(token_data) = safe_production(rule, &refs) {
            let start = matched_so_far.first().unwrap().range.start;
            let end = matched_so_far.last().unwrap().range.end;
            let mut node = Node::new(Range::new(start, end), token_data);
            node.rule_name = Some(rule.name.clone());
            node.children = matched_so_far;
            results.push(node);
        }
        return results;
    }

    match &rule.pattern[pattern_idx] {
        PatternItem::Regex(re) => {
            // Use position cache to avoid re-running the same regex at the same position.
            // Lookup uses &str borrow (no allocation); only inserts allocate.
            let pattern = re.as_str();
            let cached = pos_cache
                .get(pattern)
                .and_then(|inner| inner.get(&after_pos))
                .cloned();
            let result = match cached {
                Some(r) => r,
                None => {
                    let text = doc.text();
                    let r = if after_pos <= text.len() {
                        let search_text = &text[after_pos..];
                        re.captures(search_text).map(|caps| {
                            let m = caps.get(0).unwrap();
                            let abs_start = after_pos.saturating_add(m.start());
                            let abs_end = after_pos.saturating_add(m.end());
                            let groups = extract_groups_with_offset(&caps, text, after_pos);
                            (Range::new(abs_start, abs_end), groups)
                        })
                    } else {
                        None
                    };
                    pos_cache
                        .entry(pattern.to_string())
                        .or_default()
                        .insert(after_pos, r.clone());
                    r
                }
            };
            if let Some((range, groups)) = result {
                if doc.is_adjacent(after_pos, range.start) {
                    let regex_node = Node {
                        range,
                        token_data: TokenData::RegexMatch(RegexMatchData { groups }),
                        children: Vec::new(),
                        rule_name: None,
                    };
                    matched_so_far.push(Rc::new(regex_node));
                    let cont = match_remaining(
                        doc,
                        rule,
                        stash,
                        pattern_idx.saturating_add(1),
                        range.end,
                        matched_so_far,
                        limits,
                        pos_cache,
                    );
                    extend_with_limit(&mut results, cont, limits.max_rule_results);
                }
            }
        }
        PatternItem::Dimension(dim) => {
            // Collect matching nodes to allow move on the last one
            let matching: Vec<&Node> = stash
                .nodes_starting_from(after_pos)
                .filter(|node| {
                    node.token_data.dimension_kind() == Some(*dim)
                        && doc.is_adjacent(after_pos, node.range.start)
                })
                .collect();
            let last_idx = matching.len().saturating_sub(1);
            for (i, node) in matching.into_iter().enumerate() {
                if results.len() >= limits.max_rule_results {
                    break;
                }
                let mut next_matched = if i == last_idx {
                    std::mem::take(&mut matched_so_far)
                } else {
                    matched_so_far.clone()
                };
                next_matched.push(Rc::new(node.clone()));
                let cont = match_remaining(
                    doc,
                    rule,
                    stash,
                    pattern_idx.saturating_add(1),
                    node.range.end,
                    next_matched,
                    limits,
                    pos_cache,
                );
                extend_with_limit(&mut results, cont, limits.max_rule_results);
            }
        }
        PatternItem::Predicate(pred) => {
            // Collect matching nodes to allow move on the last one
            let matching: Vec<&Node> = stash
                .nodes_starting_from(after_pos)
                .filter(|node| {
                    pred(&node.token_data) && doc.is_adjacent(after_pos, node.range.start)
                })
                .collect();
            let last_idx = matching.len().saturating_sub(1);
            for (i, node) in matching.into_iter().enumerate() {
                if results.len() >= limits.max_rule_results {
                    break;
                }
                let mut next_matched = if i == last_idx {
                    std::mem::take(&mut matched_so_far)
                } else {
                    matched_so_far.clone()
                };
                next_matched.push(Rc::new(node.clone()));
                let cont = match_remaining(
                    doc,
                    rule,
                    stash,
                    pattern_idx.saturating_add(1),
                    node.range.end,
                    next_matched,
                    limits,
                    pos_cache,
                );
                extend_with_limit(&mut results, cont, limits.max_rule_results);
            }
        }
    }

    results
}

/// Find all regex matches in the document.
/// Matches against the original text and extracts captured groups from the
/// original text to preserve case (important for email, URL, etc.).
fn find_regex_matches(
    doc: &Document,
    re: &Regex,
    max_matches: usize,
) -> Vec<(Range, Vec<Option<String>>)> {
    let original = doc.text();
    let mut matches = Vec::new();

    for caps in re.captures_iter(original).take(max_matches) {
        let m = caps.get(0).unwrap();
        let range = Range::new(m.start(), m.end());
        let groups = extract_groups(&caps, original);
        matches.push((range, groups));
    }

    matches
}

fn extract_groups(caps: &regex::Captures, original: &str) -> Vec<Option<String>> {
    let mut groups = Vec::new();
    for i in 0..caps.len() {
        groups.push(caps.get(i).map(|m| {
            // Extract text from original (preserves case) using byte offsets
            original[m.start()..m.end()].to_string()
        }));
    }
    groups
}

/// Extract capture groups from a regex match on a substring, mapping offsets back to the
/// full text for text extraction.
fn extract_groups_with_offset(
    caps: &regex::Captures,
    full_text: &str,
    offset: usize,
) -> Vec<Option<String>> {
    let mut groups = Vec::new();
    for i in 0..caps.len() {
        groups.push(caps.get(i).map(|m| {
            let s = offset.saturating_add(m.start());
            let e = offset.saturating_add(m.end());
            full_text[s..e].to_string()
        }));
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimensions::numeral::NumeralData;
    use crate::dimensions::ordinal::OrdinalData;
    use crate::pattern::{predicate, regex};
    use crate::resolve::{Context, Options};
    use crate::types::DimensionValue;

    #[test]
    fn saturation_is_fixpoint_not_hard_capped() {
        let rules = vec![
            Rule {
                name: "seed".to_string(),
                pattern: vec![regex(r"x")],
                production: Box::new(|_| Some(TokenData::Numeral(NumeralData::new(0.0)))),
            },
            Rule {
                name: "inc".to_string(),
                pattern: vec![predicate(|td| matches!(td, TokenData::Numeral(_)))],
                production: Box::new(|nodes| match &nodes[0].token_data {
                    TokenData::Numeral(n) if n.value < 12.0 => {
                        Some(TokenData::Numeral(NumeralData::new(n.value + 1.0)))
                    }
                    _ => None,
                }),
            },
        ];

        let entities = parse_and_resolve(
            "x",
            &rules,
            &Context::default(),
            &Options { with_latent: false },
            &[DimensionKind::Numeral],
        );
        let has_12 = entities
            .iter()
            .any(|e| matches!(&e.value, DimensionValue::Numeral(v) if (*v - 12.0).abs() < 0.01));
        assert!(
            has_12,
            "Expected saturation to produce 12, got: {:?}",
            entities
        );
    }

    #[test]
    fn dedup_keeps_alternative_payloads_for_same_span_and_rule() {
        let rules = vec![
            Rule {
                name: "seed".to_string(),
                pattern: vec![regex(r"x")],
                production: Box::new(|_| Some(TokenData::Numeral(NumeralData::new(1.0)))),
            },
            Rule {
                name: "seed".to_string(),
                pattern: vec![regex(r"x")],
                production: Box::new(|_| Some(TokenData::Numeral(NumeralData::new(2.0)))),
            },
            Rule {
                name: "select-two".to_string(),
                pattern: vec![predicate(
                    |td| matches!(td, TokenData::Numeral(n) if (n.value - 2.0).abs() < 0.01),
                )],
                production: Box::new(|_| Some(TokenData::Ordinal(OrdinalData::new(2)))),
            },
        ];

        let entities = parse_and_resolve(
            "x",
            &rules,
            &Context::default(),
            &Options { with_latent: false },
            &[DimensionKind::Ordinal],
        );
        let has_ordinal_2 = entities
            .iter()
            .any(|e| matches!(&e.value, DimensionValue::Ordinal(2)));
        assert!(
            has_ordinal_2,
            "Expected ordinal(2) from alternate derivation, got: {:?}",
            entities
        );
    }

    #[test]
    fn unicode_casefold_expansion_does_not_panic_for_regex_cache_matches() {
        let rules = vec![Rule {
            name: "seed".to_string(),
            pattern: vec![regex(r".")],
            production: Box::new(|_| Some(TokenData::Numeral(NumeralData::new(1.0)))),
        }];

        let stash = parse_string("İ", &rules);
        assert!(
            !stash.is_empty(),
            "Expected at least one match for Unicode input"
        );
    }

    #[test]
    fn unicode_casefold_expansion_does_not_panic_in_match_remaining_regex() {
        let rules = vec![Rule {
            name: "two-regex".to_string(),
            pattern: vec![regex(r"a"), regex(r".")],
            production: Box::new(|_| Some(TokenData::Numeral(NumeralData::new(1.0)))),
        }];

        let stash = parse_string("aİ", &rules);
        assert!(
            stash
                .all_nodes()
                .any(|n| n.rule_name.as_deref() == Some("two-regex")),
            "Expected multi-pattern regex rule to produce a node"
        );
    }
}
