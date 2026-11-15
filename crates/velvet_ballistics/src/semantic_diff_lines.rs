use serde_json::{Value, json};

const MAX_LINE_DIFF_CELLS: usize = 1_048_576;
type EditState = (usize, usize, Vec<Value>);

enum EditKind {
    Equal,
    Insert,
    Delete,
    Done,
}

pub(super) fn line_diff(before: &[u8], after: &[u8]) -> Value {
    match aligned(before, after) {
        Ok(lines) => json!({"status": "available", "changed": before != after, "lines": lines}),
        Err(reason) => {
            json!({"status": "unavailable", "changed": before != after, "reason": reason})
        }
    }
}

fn aligned(before: &[u8], after: &[u8]) -> Result<Vec<Value>, &'static str> {
    let before = std::str::from_utf8(before).map_err(|_| "baseline source is not UTF8")?;
    let after = std::str::from_utf8(after).map_err(|_| "current source is not UTF8")?;
    let before: Vec<_> = before.lines().collect();
    let after: Vec<_> = after.lines().collect();
    let mut alignment = Alignment::new(&before, &after)?;
    alignment.fill()?;
    alignment.edits()
}

struct Alignment<'a> {
    before: &'a [&'a str],
    after: &'a [&'a str],
    columns: usize,
    matrix: Vec<usize>,
}

impl<'a> Alignment<'a> {
    fn new(before: &'a [&'a str], after: &'a [&'a str]) -> Result<Self, &'static str> {
        let (columns, matrix) = scratch(before.len(), after.len())?;
        Ok(Self {
            before,
            after,
            columns,
            matrix,
        })
    }

    fn fill(&mut self) -> Result<(), &'static str> {
        (0..self.before.len()).rev().try_for_each(|row| {
            (0..self.after.len())
                .rev()
                .try_for_each(|column| self.fill_cell(row, column))
        })
    }

    fn fill_cell(&mut self, row: usize, column: usize) -> Result<(), &'static str> {
        let (next_row, next_column) = successors(row, column)?;
        let value = if self.before.get(row) == self.after.get(column) {
            self.get(next_row, next_column)?
                .checked_add(1)
                .ok_or("line diff size overflow")?
        } else {
            self.get(next_row, column)?.max(self.get(row, next_column)?)
        };
        let index = index(self.columns, row, column)?;
        *self
            .matrix
            .get_mut(index)
            .ok_or("line diff scratch bounds")? = value;
        Ok(())
    }

    fn get(&self, row: usize, column: usize) -> Result<usize, &'static str> {
        self.matrix
            .get(index(self.columns, row, column)?)
            .copied()
            .ok_or("line diff scratch bounds")
    }

    fn edits(&self) -> Result<Vec<Value>, &'static str> {
        let bound = self
            .before
            .len()
            .checked_add(self.after.len())
            .ok_or("line diff size overflow")?;
        (0..bound)
            .try_fold((0, 0, Vec::new()), |state, _| self.advance(state))
            .map(|(_, _, edits)| edits)
    }

    fn edit_kind(&self, row: usize, column: usize) -> Result<EditKind, &'static str> {
        let old = self.before.get(row).copied();
        let new = self.after.get(column).copied();
        let (next_row, next_column) = successors(row, column)?;
        if old.is_some() && old == new {
            return Ok(EditKind::Equal);
        }
        if new.is_some()
            && (old.is_none() || self.get(row, next_column)? >= self.get(next_row, column)?)
        {
            return Ok(EditKind::Insert);
        }
        Ok(if old.is_some() {
            EditKind::Delete
        } else {
            EditKind::Done
        })
    }

    fn advance(&self, (row, column, mut edits): EditState) -> Result<EditState, &'static str> {
        let (next_row, next_column) = successors(row, column)?;
        match self.edit_kind(row, column)? {
            EditKind::Equal => Ok((next_row, next_column, edits)),
            EditKind::Insert => {
                edits.push(json!({"kind": "insert", "before_line": null, "after_line": next_column, "before": null, "after": self.after.get(column).copied()}));
                Ok((row, next_column, edits))
            }
            EditKind::Delete => {
                edits.push(json!({"kind": "delete", "before_line": next_row, "after_line": null, "before": self.before.get(row).copied(), "after": null}));
                Ok((next_row, column, edits))
            }
            EditKind::Done => Ok((row, column, edits)),
        }
    }
}

fn scratch(before: usize, after: usize) -> Result<(usize, Vec<usize>), &'static str> {
    let columns = after.checked_add(1).ok_or("line diff size overflow")?;
    let cells = before
        .checked_add(1)
        .and_then(|rows| rows.checked_mul(columns))
        .ok_or("line diff size overflow")?;
    if cells > MAX_LINE_DIFF_CELLS {
        return Err("line diff exceeds bounded scratch capacity");
    }
    let mut matrix = Vec::new();
    matrix
        .try_reserve_exact(cells)
        .map_err(|_| "line diff scratch allocation failed")?;
    matrix.resize(cells, 0usize);
    Ok((columns, matrix))
}

fn successors(row: usize, column: usize) -> Result<(usize, usize), &'static str> {
    Ok((
        row.checked_add(1).ok_or("line diff size overflow")?,
        column.checked_add(1).ok_or("line diff size overflow")?,
    ))
}

fn index(columns: usize, row: usize, column: usize) -> Result<usize, &'static str> {
    row.checked_mul(columns)
        .and_then(|offset| offset.checked_add(column))
        .ok_or("line diff size overflow")
}
