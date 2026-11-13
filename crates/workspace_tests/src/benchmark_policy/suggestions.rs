pub(super) fn suggest(argument: &str) -> Option<&'static str> {
    let stripped = argument.strip_prefix("--")?;
    let name = stripped.split_once('=').map_or(stripped, |(name, _)| name);
    let candidates = ["budget", "evidence", "help"];
    let best = candidates
        .iter()
        .filter_map(|c| jaro(name, c).map(|s| (s, *c)))
        .fold(None, |best: Option<(f64, &str)>, (score, cand)| {
            if let Some((prev_score, _)) = best {
                if score >= prev_score {
                    Some((score, cand))
                } else {
                    best
                }
            } else {
                Some((score, cand))
            }
        });
    best.and_then(|(score, cand)| if score > 0.7 { Some(cand) } else { None })
}

fn jaro(value: &str, candidate: &str) -> Option<f64> {
    let a_len = value.chars().count();
    let b_len = candidate.chars().count();
    if a_len == 0 && b_len == 0 {
        return Some(1.0);
    }
    if a_len == 0 || b_len == 0 {
        return Some(0.0);
    }
    let range = (a_len.max(b_len) / 2).saturating_sub(1);
    let mut occupied = [false; 8];
    let chars: Vec<char> = candidate.chars().collect();
    let pairs = find_matches(value, &chars, &mut occupied, range);
    jaro_score(&pairs, a_len, b_len)
}

fn jaro_score(pairs: &[(char, usize)], a_len: usize, b_len: usize) -> Option<f64> {
    if pairs.is_empty() {
        return Some(0.0);
    }
    let m = pairs.len();
    let t = count_transpositions(pairs);
    let matches = f64::from(u32::try_from(m).ok()?);
    let a = matches / f64::from(u32::try_from(a_len).ok()?);
    let b = matches / f64::from(u32::try_from(b_len).ok()?);
    let c = f64::from(u32::try_from(m.saturating_sub(t)).ok()?) / matches;
    Some((a + b + c) / 3.0)
}

fn count_transpositions(pairs: &[(char, usize)]) -> usize {
    let mut sorted = pairs.to_vec();
    sorted.sort_by_key(|pair| pair.1);
    pairs
        .iter()
        .zip(sorted.iter())
        .filter(|(a, b)| a.0 != b.0)
        .count()
        / 2
}

fn find_matches(
    value: &str,
    candidate: &[char],
    occupied: &mut [bool; 8],
    range: usize,
) -> Vec<(char, usize)> {
    value
        .chars()
        .enumerate()
        .filter_map(|(index, ch)| {
            let low = index.saturating_sub(range);
            let high = index.saturating_add(range).saturating_add(1);
            let found = candidate
                .iter()
                .enumerate()
                .take(high)
                .skip(low)
                .find(|(pos, cand_ch)| {
                    **cand_ch == ch && occupied.get(*pos).is_some_and(|used| !used)
                });
            let (pos, _) = found?;
            *occupied.get_mut(pos)? = true;
            Some((ch, pos))
        })
        .collect()
}
