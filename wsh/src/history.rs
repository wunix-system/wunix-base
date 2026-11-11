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
    file_path: Option<String>,
}

impl History {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            cursor: 0,
            capacity,
            file_path: None,
        }
    }

    pub fn with_file(capacity: usize, file_path: Option<String>) -> Self {
        Self {
            entries: Vec::new(),
            cursor: 0,
            capacity,
            file_path,
        }
    }

    pub fn file_path(&self) -> Option<&str> {
        self.file_path.as_deref()
    }

    pub fn set_file_path(&mut self, path: Option<String>) {
        self.file_path = path;
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
        while self.entries.len() > self.capacity {
            self.entries.remove(0);
        }
        self.cursor = self.entries.len();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    pub fn add(&mut self, line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            self.cursor = self.entries.len();
            return false;
        }

        // Не дублируем подряд идущие одинаковые команды
        if let Some(last) = self.entries.last() {
            if last == trimmed {
                self.cursor = self.entries.len();
                return false;
            }
        }

        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }

        self.entries.push(String::from(trimmed));
        self.cursor = self.entries.len();
        true
    }

    pub fn remove_at(&mut self, index: usize) -> Option<String> {
        if index < self.entries.len() {
            let item = self.entries.remove(index);
            self.cursor = self.entries.len();
            Some(item)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.cursor = 0;
    }

    pub fn load_from_str(&mut self, content: &str) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(last) = self.entries.last() {
                if last == trimmed {
                    continue;
                }
            }
            if self.entries.len() >= self.capacity {
                self.entries.remove(0);
            }
            self.entries.push(String::from(trimmed));
        }
        self.cursor = self.entries.len();
    }

    pub fn to_file_content(&self) -> String {
        let mut out = String::new();
        for entry in &self.entries {
            out.push_str(entry);
            out.push('\n');
        }
        out
    }

    pub fn load_file(&mut self, path: &str) -> bool {
        if let Some(content) = crate::platform::read_file(path) {
            self.load_from_str(&content);
            true
        } else {
            false
        }
    }

    pub fn save_file(&self, path: &str) -> bool {
        crate::platform::write_file(path, &self.to_file_content())
    }

    pub fn append_entry_to_file(&self, line: &str, path: &str) -> bool {
        crate::platform::append_file_line(path, line)
    }

    pub fn load_default_file(&mut self) -> bool {
        if let Some(path) = self.file_path.clone() {
            self.load_file(&path)
        } else {
            false
        }
    }

    pub fn save_default_file(&self) -> bool {
        if let Some(path) = self.file_path.as_deref() {
            self.save_file(path)
        } else {
            false
        }
    }

    pub fn append_default_file(&self, line: &str) -> bool {
        if let Some(path) = self.file_path.as_deref() {
            self.append_entry_to_file(line, path)
        } else {
            false
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_dedup_and_navigation() {
        let mut h = History::new(10);
        assert!(h.add("ls"));
        assert!(!h.add("ls"));
        assert_eq!(h.entries().len(), 1);

        assert!(h.add("pwd"));
        assert_eq!(h.entries().len(), 2);

        assert_eq!(h.prev(), Some("pwd"));
        assert_eq!(h.prev(), Some("ls"));
        assert_eq!(h.next(), Some("pwd"));
        assert_eq!(h.next(), None);
    }

    #[test]
    fn test_history_reset_cursor() {
        let mut h = History::new(10);
        h.add("cmd1");
        h.add("cmd2");
        assert_eq!(h.prev(), Some("cmd2"));
        assert_eq!(h.prev(), Some("cmd1"));

        h.reset_cursor();
        assert_eq!(h.prev(), Some("cmd2"));
    }

    #[test]
    fn test_history_capacity_limit() {
        let mut h = History::new(3);
        h.add("1");
        h.add("2");
        h.add("3");
        h.add("4");
        assert_eq!(h.entries(), &["2", "3", "4"]);
    }

    #[test]
    fn test_history_remove_and_clear() {
        let mut h = History::new(10);
        h.add("a");
        h.add("b");
        h.add("c");

        assert_eq!(h.remove_at(1), Some(String::from("b")));
        assert_eq!(h.entries(), &["a", "c"]);

        h.clear();
        assert_eq!(h.len(), 0);
        assert!(h.is_empty());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_history_file_roundtrip() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_wsh_hist_persistence.txt");
        let path_str = file_path.to_str().unwrap();

        let mut h1 = History::with_file(10, Some(String::from(path_str)));
        assert!(h1.add("echo alpha"));
        assert!(h1.add("echo beta"));
        assert!(h1.save_default_file());

        let mut h2 = History::with_file(10, Some(String::from(path_str)));
        assert!(h2.load_default_file());
        assert_eq!(h2.entries(), &["echo alpha", "echo beta"]);

        assert!(h2.add("echo gamma"));
        assert!(h2.append_default_file("echo gamma"));

        let mut h3 = History::with_file(10, Some(String::from(path_str)));
        assert!(h3.load_default_file());
        assert_eq!(h3.entries(), &["echo alpha", "echo beta", "echo gamma"]);

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_history_load_and_to_file_content() {
        let mut h = History::new(5);
        let content = "echo 1
echo 2
echo 2

echo 3
";
        h.load_from_str(content);
        assert_eq!(h.entries(), &["echo 1", "echo 2", "echo 3"]);

        let dumped = h.to_file_content();
        assert_eq!(dumped, "echo 1
echo 2
echo 3
");
    }
}
