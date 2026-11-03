#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use libwunix::{
    current_language, exit, Language, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO,
};
use man::args::{CliArgs, ManMode};
use man::db::{FoundPage, ManDatabase};
use man::format::TerminalFormatter;
use man::pager::{PagerAction, PagerState};
use man::roff::RoffParser;

const MAN_HEAP_SIZE: usize = 2 * 1024 * 1024; // 2 МиБ кучи для AST roff, форматирования строк и пейджера

struct SimpleBumpAllocator {
    heap: core::cell::UnsafeCell<[u8; MAN_HEAP_SIZE]>,
    offset: core::sync::atomic::AtomicUsize,
}

unsafe impl Sync for SimpleBumpAllocator {}

unsafe impl core::alloc::GlobalAlloc for SimpleBumpAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();
        let mut cur = self.offset.load(core::sync::atomic::Ordering::Relaxed);
        loop {
            let aligned = (cur + align - 1) & !(align - 1);
            let next = aligned + size;
            if next > MAN_HEAP_SIZE {
                return core::ptr::null_mut();
            }
            match self.offset.compare_exchange_weak(
                cur,
                next,
                core::sync::atomic::Ordering::SeqCst,
                core::sync::atomic::Ordering::Relaxed,
            ) {
                Ok(_) => {
                    let ptr = self.heap.get() as *mut u8;
                    return unsafe { ptr.add(aligned) };
                }
                Err(actual) => cur = actual,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: SimpleBumpAllocator = SimpleBumpAllocator {
    heap: core::cell::UnsafeCell::new([0u8; MAN_HEAP_SIZE]),
    offset: core::sync::atomic::AtomicUsize::new(0),
};

fn cstr_from_ptr(ptr: *const u8) -> &'static str {
    if ptr.is_null() {
        return "";
    }
    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 && len < 4096 {
            len += 1;
        }
        let slice = core::slice::from_raw_parts(ptr, len);
        core::str::from_utf8(slice).unwrap_or("")
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut args: Vec<String> = Vec::new();
    if argc > 0 && !argv.is_null() {
        for i in 0..argc {
            let p = unsafe { *argv.add(i as usize) };
            args.push(String::from(cstr_from_ptr(p)));
        }
    } else {
        args.push(String::from("man"));
    }

    let parsed = match CliArgs::parse(args) {
        Ok(a) => a,
        Err(_) => {
            libwunix::write(
                STDERR_FILENO,
                libwunix::t(
                    "man: ошибка разбора аргументов командной строки\n",
                    "man: command line argument parsing error\n",
                )
                .as_bytes(),
            );
            exit(1);
        }
    };

    let is_ru = current_language() == Language::Russian;

    match parsed.mode {
        ManMode::Help => {
            let help_text = libwunix::t(
                "Использование: man [ОПЦИИ...] [РАЗДЕЛ] СТРАНИЦА...\n\
                 \n\
                 Опции:\n\
                   -s, --section=РАЗДЕЛ   искать только в указанном разделе (1-8)\n\
                   -a, --all              отобразить все найденные страницы по очереди\n\
                   -k, --apropos          поиск по ключевому слову в кратких описаниях\n\
                   -f, --whatis           вывод однострочной справки по имени страницы\n\
                   -w, --where, --path    вывод пути к файлу страницы или каталогов MANPATH\n\
                   -7, --ascii            вывод в простом формате 7-bit ASCII без ANSI-кодов\n\
                   --no-pager             отключить пейджер и вывести страницу целиком в stdout\n\
                   -h, --help             показать эту справку и выйти\n\
                   -V, --version          вывести информацию о версии программы и выйти\n",
                "Usage: man [OPTION...] [SECTION] PAGE...\n\
                 \n\
                 Options:\n\
                   -s, --section=SECTION  look only in the specified section (1-8)\n\
                   -a, --all              display all matching manual pages found\n\
                   -k, --apropos          search short descriptions for keywords\n\
                   -f, --whatis           display short one-line description for page\n\
                   -w, --where, --path    print location of manual page files or MANPATH\n\
                   -7, --ascii            output plain 7-bit ASCII without ANSI escapes\n\
                   --no-pager             do not use interactive pager; write to stdout\n\
                   -h, --help             display this help and exit\n\
                   -V, --version          output version information and exit\n",
            );
            libwunix::write(STDOUT_FILENO, help_text.as_bytes());
            exit(0);
        }
        ManMode::Version => {
            let ver_text = libwunix::t(
                "man (Wunix man-db) 2.12.0\nАвторские права (C) 2026 Команда Wunix OS.\n",
                "man (Wunix man-db) 2.12.0\nCopyright (C) 2026 Wunix OS Team.\n",
            );
            libwunix::write(STDOUT_FILENO, ver_text.as_bytes());
            exit(0);
        }
        ManMode::MissingPage => {
            let msg = libwunix::t(
                "Какую страницу руководства вам нужно?\nПопробуйте, например, «man man».\n",
                "What manual page do you want?\nFor example, try 'man man'.\n",
            );
            libwunix::write(STDERR_FILENO, msg.as_bytes());
            exit(1);
        }
        ManMode::Apropos { keyword } => {
            let matches = ManDatabase::apropos(&keyword, is_ru);
            if matches.is_empty() {
                let msg = libwunix::t(
                    "Ничего подходящего не найдено.\n",
                    "nothing appropriate.\n",
                );
                libwunix::write(STDERR_FILENO, msg.as_bytes());
                exit(16);
            }
            for m in matches {
                let line = alloc::format!("{} ({}) - {}\n", m.name, m.section, m.description);
                libwunix::write(STDOUT_FILENO, line.as_bytes());
            }
            exit(0);
        }
        ManMode::Whatis { names } => {
            let mut any_found = false;
            for name in names {
                let matches = ManDatabase::whatis(&name, is_ru);
                if matches.is_empty() {
                    let msg = alloc::format!(
                        "{}: {}\n",
                        name,
                        libwunix::t("ничего подходящего", "nothing appropriate")
                    );
                    libwunix::write(STDERR_FILENO, msg.as_bytes());
                } else {
                    any_found = true;
                    for m in matches {
                        let line = alloc::format!("{} ({}) - {}\n", m.name, m.section, m.description);
                        libwunix::write(STDOUT_FILENO, line.as_bytes());
                    }
                }
            }
            exit(if any_found { 0 } else { 16 });
        }
        ManMode::Where { section, pages } => {
            if pages.is_empty() {
                let mut manpath = String::new();
                for (idx, p) in ManDatabase::SEARCH_PATHS.iter().enumerate() {
                    if idx > 0 {
                        manpath.push(':');
                    }
                    manpath.push_str(p);
                }
                manpath.push('\n');
                libwunix::write(STDOUT_FILENO, manpath.as_bytes());
                exit(0);
            }

            let mut all_ok = true;
            for page in pages {
                if let Some(found) = ManDatabase::find_page(&page, section, is_ru) {
                    let line = alloc::format!("{}\n", found.path);
                    libwunix::write(STDOUT_FILENO, line.as_bytes());
                } else {
                    all_ok = false;
                    let msg = if let Some(sec) = section {
                        if is_ru {
                            alloc::format!("man: Нет руководства для {} в разделе {}\n", page, sec)
                        } else {
                            alloc::format!("man: No manual entry for {} in section {}\n", page, sec)
                        }
                    } else {
                        if is_ru {
                            alloc::format!("man: Нет руководства для {}\n", page)
                        } else {
                            alloc::format!("man: No entry for {} in manual\n", page)
                        }
                    };
                    libwunix::write(STDERR_FILENO, msg.as_bytes());
                }
            }
            exit(if all_ok { 0 } else { 16 });
        }
        ManMode::Display {
            section,
            pages,
            all,
            ascii,
            no_pager,
        } => {
            let mut exit_code = 0;
            for page_name in &pages {
                let found_pages = if all {
                    ManDatabase::find_all_matching(page_name, is_ru)
                } else if let Some(p) = ManDatabase::find_page(page_name, section, is_ru) {
                    alloc::vec![p]
                } else {
                    alloc::vec![]
                };

                if found_pages.is_empty() {
                    let msg = if let Some(sec) = section {
                        if is_ru {
                            alloc::format!("man: Нет руководства для {} в разделе {}\n", page_name, sec)
                        } else {
                            alloc::format!("man: No manual entry for {} in section {}\n", page_name, sec)
                        }
                    } else {
                        if is_ru {
                            alloc::format!("man: Нет руководства для {}\n", page_name)
                        } else {
                            alloc::format!("man: No entry for {} in manual\n", page_name)
                        }
                    };
                    libwunix::write(STDERR_FILENO, msg.as_bytes());
                    exit_code = 16;
                    continue;
                }

                let total_pages = found_pages.len();
                for (idx, page) in found_pages.into_iter().enumerate() {
                    display_manual_page(&page, ascii, no_pager, is_ru);

                    if all && idx + 1 < total_pages {
                        let prompt = libwunix::t(
                            "\n-- Нажмите Enter для перехода к следующей странице (или 'q' для выхода) --\n",
                            "\n-- Press Enter for next manual page (or 'q' to quit) --\n",
                        );
                        libwunix::write(STDOUT_FILENO, prompt.as_bytes());
                        let mut b = [0u8; 1];
                        let n = libwunix::read(STDIN_FILENO, &mut b);
                        if n == 1 && (b[0] == b'q' || b[0] == b'Q' || b[0] == 0x03) {
                            break;
                        }
                    }
                }
            }
            exit(exit_code);
        }
    }
}

fn display_manual_page(page: &FoundPage, ascii_only: bool, no_pager: bool, is_ru: bool) {
    let (term_cols, term_rows) = libwunix::get_terminal_size();
    let nodes = RoffParser::parse(&page.roff_content);
    let formatter = TerminalFormatter::new(term_cols, ascii_only, is_ru);
    let lines = formatter.format_page(&nodes);

    if no_pager || lines.len() < term_rows {
        for line in &lines {
            libwunix::write(STDOUT_FILENO, line.as_bytes());
            libwunix::write(STDOUT_FILENO, b"\n");
        }
        return;
    }

    // Интерактивный полноэкранный пейджер
    run_interactive_pager(lines, &page.name, page.section, term_rows, term_cols, is_ru);
}

fn run_interactive_pager(
    lines: Vec<String>,
    name: &str,
    section: u8,
    height: usize,
    width: usize,
    is_ru: bool,
) {
    let mut state = PagerState::new(lines, name, section, height, width, is_ru);

    // Скрываем курсор во время навигации пейджера
    libwunix::write(STDOUT_FILENO, b"\x1b[?25l");

    render_pager_screen(&state);

    let mut buf = [0u8; 1];
    while !state.should_quit {
        let n = libwunix::read(STDIN_FILENO, &mut buf);
        if n == 0 {
            libwunix::yield_now();
            continue;
        }
        if (n as i64) < 0 {
            break;
        }

        let ch = buf[0];
        let action = match ch {
            b'q' | b'Q' | 0x03 => PagerAction::Quit,
            b' ' | b'f' => PagerAction::DownPage,
            b'b' => PagerAction::UpPage,
            b'\n' | b'\r' | b'j' => PagerAction::DownLine,
            b'k' | b'y' => PagerAction::UpLine,
            0x04 | b'd' => PagerAction::HalfDown, // Ctrl+D или 'd'
            0x15 | b'u' => PagerAction::HalfUp,   // Ctrl+U или 'u'
            b'g' | b'<' => PagerAction::Top,
            b'G' | b'>' => PagerAction::Bottom,
            b'h' | b'?' => PagerAction::ToggleHelp,
            b'n' => PagerAction::NextMatch,
            b'N' => PagerAction::PrevMatch,
            b'/' => {
                prompt_search(&mut state);
                PagerAction::None
            }
            0x1b => {
                // Escape-последовательность для стрелок
                handle_escape_sequence()
            }
            _ => PagerAction::None,
        };

        if action != PagerAction::None {
            state.handle_action(action);
            render_pager_screen(&state);
        }
    }

    // Восстанавливаем отображение курсора, очищаем экран и возвращаемся в начало
    libwunix::write(STDOUT_FILENO, b"\x1b[?25h\x1b[2J\x1b[H");
}

fn handle_escape_sequence() -> PagerAction {
    let mut seq = [0u8; 3];
    let n1 = libwunix::read(STDIN_FILENO, &mut seq[0..1]);
    if n1 != 1 || seq[0] != b'[' {
        return PagerAction::None;
    }
    let n2 = libwunix::read(STDIN_FILENO, &mut seq[1..2]);
    if n2 != 1 {
        return PagerAction::None;
    }

    match seq[1] {
        b'A' => PagerAction::UpLine,
        b'B' => PagerAction::DownLine,
        b'H' => PagerAction::Top,
        b'F' => PagerAction::Bottom,
        b'5' => {
            // PageUp (\x1b[5~)
            let _ = libwunix::read(STDIN_FILENO, &mut seq[2..3]);
            PagerAction::UpPage
        }
        b'6' => {
            // PageDown (\x1b[6~)
            let _ = libwunix::read(STDIN_FILENO, &mut seq[2..3]);
            PagerAction::DownPage
        }
        _ => PagerAction::None,
    }
}

fn prompt_search(state: &mut PagerState) {
    // Включаем курсор на время ввода шаблона поиска в нижней строке
    libwunix::write(STDOUT_FILENO, b"\x1b[?25h");
    let move_cursor = alloc::format!("\x1b[{};1H\x1b[K/", state.screen_height);
    libwunix::write(STDOUT_FILENO, move_cursor.as_bytes());
    let mut query = String::new();
    let mut b = [0u8; 1];

    loop {
        let n = libwunix::read(STDIN_FILENO, &mut b);
        if n == 1 {
            let ch = b[0];
            if ch == b'\r' || ch == b'\n' {
                break;
            } else if ch == 0x08 || ch == 0x7f {
                if !query.is_empty() {
                    query.pop();
                    libwunix::write(STDOUT_FILENO, b"\x08 \x08");
                }
            } else if ch == 0x1b || ch == 0x03 {
                // Отмена поиска: снова скрываем курсор
                libwunix::write(STDOUT_FILENO, b"\x1b[?25l");
                return;
            } else if ch >= 32 && ch < 127 {
                query.push(ch as char);
                libwunix::write(STDOUT_FILENO, &[ch]);
            }
        }
    }

    libwunix::write(STDOUT_FILENO, b"\x1b[?25l");
    if !query.is_empty() {
        state.handle_action(PagerAction::Search(query));
    }
}

fn render_pager_screen(state: &PagerState) {
    // Перемещаем курсор в левый верхний угол и очищаем экран
    libwunix::write(STDOUT_FILENO, b"\x1b[H\x1b[2J");

    let view = state.render_current_view();
    for line in view {
        libwunix::write(STDOUT_FILENO, line.as_bytes());
        libwunix::write(STDOUT_FILENO, b"\n");
    }

    // Выводим статус-бар точно на нижней строке терминала
    let move_cursor = alloc::format!("\x1b[{};1H\x1b[K", state.screen_height);
    libwunix::write(STDOUT_FILENO, move_cursor.as_bytes());
    let status_bar = state.format_status_bar();
    libwunix::write(STDOUT_FILENO, status_bar.as_bytes());
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    libwunix::write(STDERR_FILENO, b"panic in man\n");
    exit(1);
}
