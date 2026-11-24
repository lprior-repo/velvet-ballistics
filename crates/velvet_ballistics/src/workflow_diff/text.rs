#![forbid(unsafe_code)]
//! Line-level textual diff of workflow YAML sources.

/// Largest LCS table (cells) built for the differing middle of two files.
/// Beyond it the middle is reported as one wholesale replacement.
const MAX_TABLE_CELLS: usize = 4_000_000;

/// One added or removed source line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LineChange {
    pub(super) added: bool,
    /// 1-based line number in the new file (added) or old file (removed).
    pub(super) line: usize,
    pub(super) text: String,
}

/// Result of a line diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LineDiff {
    pub(super) changes: Vec<LineChange>,
    /// True when the differing region exceeded the LCS bound and was not aligned.
    pub(super) coarse: bool,
}

impl LineDiff {
    pub(super) fn count(&self, added: bool) -> usize {
        self.changes.iter().filter(|c| c.added == added).count()
    }
}

struct Table {
    width: usize,
    cells: Vec<u32>,
}

impl Table {
    fn new(rows: usize, cols: usize) -> Option<Self> {
        let (height, width) = (rows.checked_add(1)?, cols.checked_add(1)?);
        let total = height.checked_mul(width)?;
        (total <= MAX_TABLE_CELLS).then(|| Self {
            width,
            cells: vec![0; total],
        })
    }

    fn index(&self, row: usize, col: usize) -> usize {
        row.saturating_mul(self.width).saturating_add(col)
    }

    fn get(&self, row: usize, col: usize) -> u32 {
        self.cells.get(self.index(row, col)).copied().unwrap_or(0)
    }

    fn set(&mut self, row: usize, col: usize, value: u32) {
        let index = self.index(row, col);
        if let Some(cell) = self.cells.get_mut(index) {
            *cell = value;
        }
    }
}

/// Diffs two sources line by line.
pub(super) fn diff_lines(old: &str, new: &str) -> LineDiff {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let prefix = old_lines
        .iter()
        .zip(&new_lines)
        .take_while(|(a, b)| a == b)
        .count();
    let limit = old_lines.len().min(new_lines.len()).saturating_sub(prefix);
    let suffix = old_lines
        .iter()
        .rev()
        .zip(new_lines.iter().rev())
        .take(limit)
        .take_while(|(a, b)| a == b)
        .count();
    let old_mid = middle(&old_lines, prefix, suffix);
    let new_mid = middle(&new_lines, prefix, suffix);
    match Table::new(old_mid.len(), new_mid.len()) {
        Some(table) => LineDiff {
            changes: walk(
                old_mid,
                new_mid,
                &lcs_table(table, old_mid, new_mid),
                prefix,
            ),
            coarse: false,
        },
        None => LineDiff {
            changes: replace_all(old_mid, new_mid, prefix),
            coarse: true,
        },
    }
}

fn middle<'a>(lines: &'a [&'a str], prefix: usize, suffix: usize) -> &'a [&'a str] {
    let end = lines.len().saturating_sub(suffix);
    lines.get(prefix..end).unwrap_or(&[])
}

/// `table[i][j]` = LCS length of `old[i..]` and `new[j..]`.
fn lcs_table(mut table: Table, old: &[&str], new: &[&str]) -> Table {
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            let value = if old.get(i) == new.get(j) {
                table
                    .get(i.saturating_add(1), j.saturating_add(1))
                    .saturating_add(1)
            } else {
                table
                    .get(i.saturating_add(1), j)
                    .max(table.get(i, j.saturating_add(1)))
            };
            table.set(i, j, value);
        }
    }
    table
}

fn walk(old: &[&str], new: &[&str], table: &Table, offset: usize) -> Vec<LineChange> {
    let (mut i, mut j) = (0_usize, 0_usize);
    let mut out = Vec::new();
    while i < old.len() || j < new.len() {
        let same = i < old.len() && j < new.len() && old.get(i) == new.get(j);
        let take_old = j >= new.len()
            || (i < old.len()
                && table.get(i.saturating_add(1), j) >= table.get(i, j.saturating_add(1)));
        if same {
            i = i.saturating_add(1);
            j = j.saturating_add(1);
        } else if take_old {
            out.push(line_change(false, offset, i, old.get(i)));
            i = i.saturating_add(1);
        } else {
            out.push(line_change(true, offset, j, new.get(j)));
            j = j.saturating_add(1);
        }
    }
    out
}

fn line_change(added: bool, offset: usize, index: usize, text: Option<&&str>) -> LineChange {
    LineChange {
        added,
        line: offset.saturating_add(index).saturating_add(1),
        text: text.map_or_else(String::new, |line| (*line).to_owned()),
    }
}

fn replace_all(old: &[&str], new: &[&str], offset: usize) -> Vec<LineChange> {
    let removed = old
        .iter()
        .enumerate()
        .map(|(i, line)| line_change(false, offset, i, Some(line)));
    let added = new
        .iter()
        .enumerate()
        .map(|(j, line)| line_change(true, offset, j, Some(line)));
    removed.chain(added).collect()
}
