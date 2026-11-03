#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

pub struct History {
    entries: Vec<String>,
    cursor: usize,
    capacity: usize,
}

impl History {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            cursor: 0,
            capacity,
        }
    }

    pub fn add(&mut self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            self.cursor = self.entries.len();
            return;
        }

        // Дедупликация: не дублируем одинаковые последовательные команды
        if let Some(last) = self.entries.last() {
            if last == trimmed {
                self.cursor = self.entries.len();
                return;
            }
        }

        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }

        self.entries.push(String::from(trimmed));
        self.cursor = self.entries.len();
    }

    pub fn prev(&mut self) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        if self.cursor > 0 {
            self.cursor -= 1;
        }
        Some(&self.entries[self.cursor])
    }

    pub fn next(&mut self) -> Option<&str> {
        if self.entries.is_empty() || self.cursor >= self.entries.len() {
            return None;
        }
        self.cursor += 1;
        if self.cursor < self.entries.len() {
            Some(&self.entries[self.cursor])
        } else {
            None
        }
    }

    pub fn reset_cursor(&mut self) {
        self.cursor = self.entries.len();
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_dedup_and_navigation() {
        let mut h = History::new(10);
        h.add("ls");
        h.add("ls");
        assert_eq!(h.entries().len(), 1);

        h.add("pwd");
        assert_eq!(h.entries().len(), 2);

        assert_eq!(h.prev(), Some("pwd"));
        assert_eq!(h.prev(), Some("ls"));
        assert_eq!(h.next(), Some("pwd"));
        assert_eq!(h.next(), None);
    }
}
