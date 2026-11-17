//! Модуль интерактивных индикаторов выполнения (Progress Bar) для WPM.
//!
//! Обеспечивает наглядную визуализацию загрузки файлов по сети с вычислением
//! текущей скорости, процента выполнения и очисткой строки в терминале.

extern crate alloc;
use alloc::format;
use alloc::string::String;
use libwunix::{tcgetattr, write, Termios, STDOUT_FILENO};

pub struct ProgressBar {
    title: String,
    total_bytes: Option<u64>,
    current_bytes: u64,
    start_sec: i64,
    start_nsec: i64,
    last_update_sec: i64,
    last_update_nsec: i64,
    is_tty: bool,
    bar_width: usize,
}

impl ProgressBar {
    pub fn new(title: &str, total_bytes: Option<u64>) -> Self {
        let mut tio = Termios::default();
        let is_tty = tcgetattr(STDOUT_FILENO, &mut tio) == 0;
        let ts = libwunix::time::clock_realtime();

        let mut pb = Self {
            title: title.into(),
            total_bytes,
            current_bytes: 0,
            start_sec: ts.tv_sec,
            start_nsec: ts.tv_nsec,
            last_update_sec: ts.tv_sec,
            last_update_nsec: ts.tv_nsec,
            is_tty,
            bar_width: 28,
        };

        if !is_tty {
            let msg = if let Some(tot) = total_bytes {
                format!("  {} (загрузка, {})...\n", title, format_bytes(tot))
            } else {
                format!("  {} (загрузка)...\n", title)
            };
            let _ = write(STDOUT_FILENO, msg.as_bytes());
        } else {
            pb.render(false);
        }

        pb
    }

    pub fn update(&mut self, current: u64) {
        self.current_bytes = current;
        if !self.is_tty {
            return;
        }

        let now = libwunix::time::clock_realtime();
        let delta_ms = (now.tv_sec.saturating_sub(self.last_update_sec) as i128)
            .saturating_mul(1000)
            .saturating_add(((now.tv_nsec.saturating_sub(self.last_update_nsec)) / 1_000_000) as i128);

        // Обновляем терминал не чаще раза в 100 мс во избежание мерцания и торможения I/O
        if delta_ms < 100 {
            return;
        }

        self.last_update_sec = now.tv_sec;
        self.last_update_nsec = now.tv_nsec;
        self.render(false);
    }

    pub fn finish(&mut self, success: bool, message: &str) {
        if self.is_tty {
            self.render(true);
            let _ = write(STDOUT_FILENO, b"\n");
        }
        if !message.is_empty() {
            let status = if success {
                "\x1b[32m[OK]\x1b[0m"
            } else {
                "\x1b[31m[ОШИБКА]\x1b[0m"
            };
            let line = format!("  {} {}\n", status, message);
            let _ = write(STDOUT_FILENO, line.as_bytes());
        }
    }

    fn render(&mut self, _is_final: bool) {
        let now = libwunix::time::clock_realtime();
        let elapsed_ms = (now.tv_sec.saturating_sub(self.start_sec) as i128)
            .saturating_mul(1000)
            .saturating_add(((now.tv_nsec.saturating_sub(self.start_nsec)) / 1_000_000) as i128)
            .max(1) as u64;

        let speed = (self.current_bytes.saturating_mul(1000)) / elapsed_ms;

        let mut line = String::new();
        line.push('\r');
        line.push_str("  ");

        // Отображаем имя файла/задачи (до 20 символов)
        let title_display = if self.title.len() > 20 {
            &self.title[..20]
        } else {
            &self.title
        };
        line.push_str(&format!("{:<20} ", title_display));

        if let Some(total) = self.total_bytes {
            let ratio = if total > 0 {
                (self.current_bytes as f64 / total as f64).min(1.0)
            } else {
                1.0
            };
            let percent = (ratio * 100.0) as u32;
            let filled = ((self.bar_width as f64) * ratio) as usize;
            let empty = self.bar_width.saturating_sub(filled);

            line.push('[');
            line.push_str("\x1b[32m");
            for _ in 0..filled {
                line.push('=');
            }
            line.push_str("\x1b[0m");
            for _ in 0..empty {
                line.push(' ');
            }
            line.push(']');

            line.push_str(&format!(" {:>3}% {}/{} [{}/s]", percent, format_bytes(self.current_bytes), format_bytes(total), format_bytes(speed)));
        } else {
            // Неизвестный размер (например, потоковый ответ без Content-Length)
            let spin_chars = ['|', '/', '-', '\\'];
            let spinner = spin_chars[((elapsed_ms / 200) % 4) as usize];
            line.push_str(&format!("[{}] {} [{}/s]", spinner, format_bytes(self.current_bytes), format_bytes(speed)));
        }

        // Очистка остатка строки от предыдущего длинного вывода
        line.push_str("\x1b[K");

        let _ = write(STDOUT_FILENO, line.as_bytes());
    }
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} Б", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} КиБ", (bytes as f64) / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} МиБ", (bytes as f64) / (1024.0 * 1024.0))
    } else {
        format!("{:.2} ГиБ", (bytes as f64) / (1024.0 * 1024.0 * 1024.0))
    }
}
