use crate::SourceId;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    source_id: SourceId,
    start: u32,
    end: u32,
}

impl Span {
    pub const fn new(source_id: SourceId, start: u32, end: u32) -> Self {
        Self {
            source_id,
            start,
            end,
        }
    }

    pub const fn empty(source_id: SourceId, offset: u32) -> Self {
        Self {
            source_id,
            start: offset,
            end: offset,
        }
    }

    pub const fn source_id(self) -> SourceId {
        self.source_id
    }

    pub const fn start(self) -> u32 {
        self.start
    }

    pub const fn end(self) -> u32 {
        self.end
    }

    pub const fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    pub const fn contains(self, offset: u32) -> bool {
        self.start <= offset && offset < self.end
    }

    pub fn checked_new(source_id: SourceId, start: u32, end: u32) -> Option<Self> {
        if start <= end {
            Some(Self::new(source_id, start, end))
        } else {
            None
        }
    }

    pub fn cover(self, other: Self) -> Option<Self> {
        if self.source_id != other.source_id {
            return None;
        }

        Some(Self {
            source_id: self.source_id,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        })
    }
}
