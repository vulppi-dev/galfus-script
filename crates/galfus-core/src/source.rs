use crate::{SourceId, Span};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowCol {
    pub row: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceFile {
    id: SourceId,
    name: String,
    text: String,
    row_starts: Vec<u32>,
}

impl SourceFile {
    pub fn new(id: SourceId, name: String, text: String) -> Self {
        let mut row_starts = Vec::new();

        row_starts.push(0);

        for (byte_index, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                row_starts.push((byte_index + 1) as u32);
            }
        }

        Self {
            id,
            name,
            text,
            row_starts,
        }
    }

    pub fn id(&self) -> SourceId {
        self.id
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn text(&self) -> &str {
        self.text.as_str()
    }

    pub fn len(&self) -> u32 {
        self.text.len() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.text.len() == 0
    }

    pub fn row_col(&self, offset: u32) -> Option<RowCol> {
        let len = self.len();

        if offset > len {
            return None;
        }

        let index = match self.row_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(0) => return None,
            Err(i) => i - 1,
        };
        let start = self.row_starts[index];

        Some(RowCol {
            row: (index as u32) + 1,
            column: offset - start + 1,
        })
    }

    pub fn offset(&self, line_col: &RowCol) -> Option<u32> {
        let row_index = line_col.row.checked_sub(1)? as usize;
        let column_offset = line_col.column.checked_sub(1)?;
        let row_start = *self.row_starts.get(row_index)?;
        Some(row_start + column_offset)
    }

    pub fn slice(&self, span: Span) -> Option<&str> {
        if span.source_id() != self.id {
            return None;
        }

        self.text.get(span.start() as usize..span.end() as usize)
    }

    pub fn span(&self) -> Span {
        Span::new(self.id, 0, self.len())
    }
}
