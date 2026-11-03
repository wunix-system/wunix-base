use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, PartialEq, Eq)]
pub enum PagerAction {
    Quit,
    DownLine,
    UpLine,
    DownPage,
    UpPage,
    HalfDown,
    HalfUp,
    Top,
    Bottom,
    NextMatch,
    PrevMatch,
    ToggleHelp,
    Search(String),
    None,
}

pub struct PagerState {
    pub lines: Vec<String>,
    pub top_line: usize,
    pub screen_height: usize,
    pub screen_width: usize,
    pub search_query: Option<String>,
    pub search_matches: Vec<usize>,
    pub show_help: bool,
    pub should_quit: bool,
    pub page_name: String,
    pub page_sec: u8,
    pub is_ru: bool,
}

impl PagerState {
    pub fn new(lines: Vec<String>, page_name: &str, page_sec: u8, height: usize, width: usize, is_ru: bool) -> Self {
        Self {
            lines,
            top_line: 0,
            screen_height: height.max(5),
            screen_width: width.max(20),
            search_query: None,
            search_matches: Vec::new(),
            show_help: false,
            should_quit: false,
            page_name: String::from(page_name),
            page_sec,
            is_ru,
        }
    }

    pub fn max_top_line(&self) -> usize {
        let page_size = self.screen_height.saturating_sub(1);
        self.lines.len().saturating_sub(page_size)
    }

    pub fn handle_action(&mut self, action: PagerAction) {
        let page_size = self.screen_height.saturating_sub(1).max(1);
        let max_top = self.max_top_line();

        match action {
            PagerAction::Quit => {
                self.should_quit = true;
            }
            PagerAction::DownLine => {
                if self.top_line < max_top {
                    self.top_line += 1;
                }
            }
            PagerAction::UpLine => {
                self.top_line = self.top_line.saturating_sub(1);
            }
            PagerAction::DownPage => {
                self.top_line = (self.top_line + page_size).min(max_top);
            }
            PagerAction::UpPage => {
                self.top_line = self.top_line.saturating_sub(page_size);
            }
            PagerAction::HalfDown => {
                self.top_line = (self.top_line + (page_size / 2).max(1)).min(max_top);
            }
            PagerAction::HalfUp => {
                self.top_line = self.top_line.saturating_sub((page_size / 2).max(1));
            }
            PagerAction::Top => {
                self.top_line = 0;
            }
            PagerAction::Bottom => {
                self.top_line = max_top;
            }
            PagerAction::ToggleHelp => {
                self.show_help = !self.show_help;
            }
            PagerAction::Search(query) => {
                self.execute_search(&query);
            }
            PagerAction::NextMatch => {
                self.jump_to_next_match();
            }
            PagerAction::PrevMatch => {
                self.jump_to_prev_match();
            }
            PagerAction::None => {}
        }
    }

    pub fn execute_search(&mut self, query: &str) {
        if query.is_empty() {
            return;
        }
        let q_lower = query.to_lowercase();
        let mut matches = Vec::new();
        for (idx, line) in self.lines.iter().enumerate() {
            if line.to_lowercase().contains(&q_lower) {
                matches.push(idx);
            }
        }

        self.search_query = Some(String::from(query));
        self.search_matches = matches;

        // Переходим к первому совпадению на текущем экране или ниже
        self.jump_to_next_match();
    }

    pub fn jump_to_next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        let max_top = self.max_top_line();
        for &m_line in &self.search_matches {
            if m_line > self.top_line {
                self.top_line = m_line.min(max_top);
                return;
            }
        }
        // Зацикливаемся в начало
        if let Some(&first) = self.search_matches.first() {
            self.top_line = first.min(max_top);
        }
    }

    pub fn jump_to_prev_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        let max_top = self.max_top_line();
        for &m_line in self.search_matches.iter().rev() {
            if m_line < self.top_line {
                self.top_line = m_line.min(max_top);
                return;
            }
        }
        // Зацикливаемся в конец
        if let Some(&last) = self.search_matches.last() {
            self.top_line = last.min(max_top);
        }
    }

    pub fn format_status_bar(&self) -> String {
        let total = self.lines.len();
        let cur = (self.top_line + 1).min(total);
        let pct = if total > 0 { (cur * 100) / total } else { 100 };
        let is_end = self.top_line >= self.max_top_line();

        let end_marker = if is_end {
            if self.is_ru { " (КОНЕЦ)" } else { " (END)" }
        } else {
            ""
        };

        // Запас в 1 колонку исключает автоперенос терминала на последнем знакоместе строки
        let max_len = self.screen_width.saturating_sub(1);

        let full_text = if self.is_ru {
            alloc::format!(
                " Руководство {}({}), строки {}/{} ({}%){} (нажмите h для справки или q для выхода) ",
                self.page_name,
                self.page_sec,
                cur,
                total,
                pct,
                end_marker
            )
        } else {
            alloc::format!(
                " Manual page {}({}) line {}/{} ({}%){} (press h for help or q to quit) ",
                self.page_name,
                self.page_sec,
                cur,
                total,
                pct,
                end_marker
            )
        };

        let text = if full_text.chars().count() <= max_len {
            full_text
        } else {
            let compact_text = if self.is_ru {
                alloc::format!(
                    " {}({}) {}/{} ({}%){} (h - справка, q - выход) ",
                    self.page_name,
                    self.page_sec,
                    cur,
                    total,
                    pct,
                    end_marker
                )
            } else {
                alloc::format!(
                    " {}({}) {}/{} ({}%){} (h: help, q: quit) ",
                    self.page_name,
                    self.page_sec,
                    cur,
                    total,
                    pct,
                    end_marker
                )
            };

            if compact_text.chars().count() <= max_len {
                compact_text
            } else {
                let truncated: String = compact_text.chars().take(max_len).collect();
                truncated
            }
        };

        alloc::format!("\x1b[7m{}\x1b[0m", text)
    }

    pub fn format_help_box(&self) -> Vec<String> {
        let mut help = Vec::new();
        if self.is_ru {
            help.push(String::from("========== Справка по навигации Wunix man =========="));
            help.push(String::from("  q, Q, Ctrl+C     : Выход из справочника"));
            help.push(String::from("  Пробел, f, PgDn  : Прокрутка на страницу вперед"));
            help.push(String::from("  b, PgUp          : Прокрутка на страницу назад"));
            help.push(String::from("  Enter, j, Вниз   : Прокрутка на строку вниз"));
            help.push(String::from("  k, y, Вверх      : Прокрутка на строку вверх"));
            help.push(String::from("  g, <             : Перейти в самое начало документа"));
            help.push(String::from("  G, >             : Перейти в самый конец документа"));
            help.push(String::from("  /шаблон          : Поиск текста по странице"));
            help.push(String::from("  n / N            : Следующее / предыдущее совпадение"));
            help.push(String::from("  h, ?             : Показать / скрыть эту справку"));
            help.push(String::from("===================================================="));
        } else {
            help.push(String::from("============== Wunix man Pager Help =============="));
            help.push(String::from("  q, Q, Ctrl+C     : Exit manual"));
            help.push(String::from("  Space, f, PgDn   : Scroll one page forward"));
            help.push(String::from("  b, PgUp          : Scroll one page backward"));
            help.push(String::from("  Enter, j, Down   : Scroll one line down"));
            help.push(String::from("  k, y, Up         : Scroll one line up"));
            help.push(String::from("  g, <             : Jump to document beginning"));
            help.push(String::from("  G, >             : Jump to document end"));
            help.push(String::from("  /pattern         : Search text in page"));
            help.push(String::from("  n / N            : Next / previous search match"));
            help.push(String::from("  h, ?             : Show / hide this help"));
            help.push(String::from("=================================================="));
        }
        help
    }

    pub fn render_current_view(&self) -> Vec<String> {
        if self.show_help {
            return self.format_help_box();
        }

        let page_size = self.screen_height.saturating_sub(1);
        let mut view = Vec::new();

        for i in 0..page_size {
            let line_idx = self.top_line + i;
            if line_idx < self.lines.len() {
                let line = &self.lines[line_idx];
                view.push(line.clone());
            } else {
                view.push(String::from("~"));
            }
        }

        view
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_pager_navigation() {
        let lines: Vec<String> = (0..50).map(|i| alloc::format!("Line {}", i)).collect();
        let mut pager = PagerState::new(lines, "cat", 1, 24, 80, true);

        assert_eq!(pager.top_line, 0);
        let status = pager.format_status_bar();
        assert!(status.contains("Руководство cat(1)"));

        pager.handle_action(PagerAction::DownLine);
        assert_eq!(pager.top_line, 1);

        pager.handle_action(PagerAction::DownPage);
        assert_eq!(pager.top_line, 24);

        pager.handle_action(PagerAction::UpLine);
        assert_eq!(pager.top_line, 23);

        pager.handle_action(PagerAction::Bottom);
        assert_eq!(pager.top_line, pager.max_top_line());
        let end_status = pager.format_status_bar();
        assert!(end_status.contains("КОНЕЦ"));

        pager.handle_action(PagerAction::Top);
        assert_eq!(pager.top_line, 0);

        pager.handle_action(PagerAction::Quit);
        assert!(pager.should_quit);
    }

    #[test]
    fn test_status_bar_width_and_content() {
        let lines: Vec<String> = (0..77).map(|i| alloc::format!("Line {}", i)).collect();
        let pager = PagerState::new(lines, "man", 1, 25, 80, true);
        let status = pager.format_status_bar();
        assert!(status.contains("Руководство man(1), строки 1/77 (1%)"));
        assert!(status.contains("нажмите h для справки или q для выхода"));

        // Проверяем адаптивное сжатие для узких терминалов
        let narrow_pager = PagerState::new(
            (0..77).map(|i| alloc::format!("Line {}", i)).collect(),
            "very_long_program_name",
            1,
            25,
            40,
            true,
        );
        let narrow_status = narrow_pager.format_status_bar();
        assert!(narrow_status.contains("very_long_program_name"));
    }

    #[test]
    fn test_pager_search() {
        let mut lines = Vec::new();
        for i in 0..30 {
            if i == 5 || i == 15 {
                lines.push(alloc::format!("Line {} target match", i));
            } else {
                lines.push(alloc::format!("Line {}", i));
            }
        }
        let mut pager = PagerState::new(lines, "test", 1, 10, 80, false);
        pager.handle_action(PagerAction::Search(String::from("target")));

        assert_eq!(pager.search_matches, vec![5, 15]);
        assert_eq!(pager.top_line, 5);

        pager.handle_action(PagerAction::NextMatch);
        assert_eq!(pager.top_line, 15);
    }
}
