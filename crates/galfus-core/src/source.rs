use crate::{SourceId, Span};

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SourceId, Span};

    fn source(text: &str) -> SourceFile {
        SourceFile::new(SourceId::new(0), "main.gfs".to_string(), text.to_string())
    }

    #[test]
    fn source_file_stores_basic_data() {
        let source = source("fn main(): null {}");

        assert_eq!(source.id(), SourceId::new(0));
        assert_eq!(source.name(), "main.gfs");
        assert_eq!(source.text(), "fn main(): null {}");
        assert_eq!(source.len(), "fn main(): null {}".len() as u32);
        assert!(!source.is_empty());
    }

    #[test]
    fn source_file_can_be_empty() {
        let source = source("");

        assert_eq!(source.len(), 0);
        assert!(source.is_empty());
    }

    #[test]
    fn row_col_for_empty_source_accepts_eof_offset() {
        let source = source("");

        let pos = source.row_col(0).unwrap();

        assert_eq!(pos.row, 1);
        assert_eq!(pos.column, 1);
    }

    #[test]
    fn row_col_in_single_row_source() {
        let source = source("abc");

        let start = source.row_col(0).unwrap();
        let middle = source.row_col(1).unwrap();
        let eof = source.row_col(3).unwrap();

        assert_eq!(start.row, 1);
        assert_eq!(start.column, 1);

        assert_eq!(middle.row, 1);
        assert_eq!(middle.column, 2);

        assert_eq!(eof.row, 1);
        assert_eq!(eof.column, 4);
    }

    #[test]
    fn row_col_in_multiple_rows_source() {
        let source = source("abc\ndef");

        let row_1 = source.row_col(0).unwrap();
        let row_2 = source.row_col(4).unwrap();
        let eof = source.row_col(7).unwrap();

        assert_eq!(row_1.row, 1);
        assert_eq!(row_1.column, 1);

        assert_eq!(row_2.row, 2);
        assert_eq!(row_2.column, 1);

        assert_eq!(eof.row, 2);
        assert_eq!(eof.column, 4);
    }

    #[test]
    fn row_col_handles_trailing_newline() {
        let source = source("abc\n");

        let after_newline = source.row_col(4).unwrap();

        assert_eq!(after_newline.row, 2);
        assert_eq!(after_newline.column, 1);
    }

    #[test]
    fn row_col_handles_multiple_empty_rows() {
        let source = source("\n\n");

        let row_1 = source.row_col(0).unwrap();
        let row_2 = source.row_col(1).unwrap();
        let row_3 = source.row_col(2).unwrap();

        assert_eq!(row_1.row, 1);
        assert_eq!(row_1.column, 1);

        assert_eq!(row_2.row, 2);
        assert_eq!(row_2.column, 1);

        assert_eq!(row_3.row, 3);
        assert_eq!(row_3.column, 1);
    }

    #[test]
    fn row_col_returns_none_when_offset_is_after_eof() {
        let source = source("abc");

        assert_eq!(source.row_col(4), None);
    }

    #[test]
    fn offset_converts_row_col_back_to_byte_offset() {
        let source = source("abc\ndef");

        let first = source.row_col(0).unwrap();
        let second_row_start = source.row_col(4).unwrap();
        let eof = source.row_col(7).unwrap();

        assert_eq!(source.offset(&first), Some(0));
        assert_eq!(source.offset(&second_row_start), Some(4));
        assert_eq!(source.offset(&eof), Some(7));
    }

    #[test]
    fn offset_returns_none_for_invalid_row() {
        let source = source("abc");

        let invalid = RowCol { row: 2, column: 1 };

        assert_eq!(source.offset(&invalid), None);
    }

    #[test]
    fn offset_returns_none_for_zero_row_or_zero_column() {
        let source = source("abc");

        let zero_row = RowCol { row: 0, column: 1 };

        let zero_column = RowCol { row: 1, column: 0 };

        assert_eq!(source.offset(&zero_row), None);
        assert_eq!(source.offset(&zero_column), None);
    }

    #[test]
    fn slice_returns_text_inside_span() {
        let source = source("hello world");

        let span = Span::new(SourceId::new(0), 0, 5);

        assert_eq!(source.slice(span), Some("hello"));
    }

    #[test]
    fn slice_returns_none_for_different_source() {
        let source = source("hello world");

        let span = Span::new(SourceId::new(1), 0, 5);

        assert_eq!(source.slice(span), None);
    }

    #[test]
    fn slice_returns_none_for_invalid_range() {
        let source = source("hello world");

        let span = Span::new(SourceId::new(0), 0, 100);

        assert_eq!(source.slice(span), None);
    }

    #[test]
    fn slice_returns_none_when_range_breaks_utf8_boundary() {
        let source = source("ação");

        // "ç" has more than one byte in UTF-8.
        // This span intentionally cuts inside a UTF-8 character.
        let span = Span::new(SourceId::new(0), 1, 2);

        assert_eq!(source.slice(span), None);
    }
}
