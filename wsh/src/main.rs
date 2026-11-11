#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::string::String;

#[cfg(feature = "std")]
use std::string::String;

use wsh::builtins::Builtins;
use wsh::editor::LineEditor;
use wsh::history::History;
use wsh::parser::Parser;
use wsh::platform;
use wsh::prompt::Prompt;

#[cfg(feature = "std")]
fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Обработка запуска скрипта или команды через -c: wsh -c "команда"
    if args.len() >= 3 && args[1] == "-c" {
        let cmd_str = &args[2];
        let exit_code = run_command_line(cmd_str, &mut History::new(10), 0, &mut false);
        std::process::exit(exit_code);
    }

    if args.len() >= 2 && !args[1].starts_with('-') {
        if let Ok(content) = std::fs::read_to_string(&args[1]) {
            let mut history = History::new(100);
            let mut last_status = 0;
            let mut should_exit = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                last_status = run_command_line(trimmed, &mut history, last_status, &mut should_exit);
                if should_exit {
                    break;
                }
            }
            std::process::exit(last_status);
        } else {
            eprintln!("wsh: cannot open script: {}", args[1]);
            std::process::exit(1);
        }
    }

    run_interactive_shell();
}

#[cfg(not(feature = "std"))]
const WSH_HEAP_SIZE: usize = 2 * 1024 * 1024; // 2 МиБ кучи для строк, истории и парсера шелла

#[cfg(all(not(feature = "std"), not(test)))]
struct SimpleBumpAllocator {
    heap: core::cell::UnsafeCell<[u8; WSH_HEAP_SIZE]>,
    offset: core::sync::atomic::AtomicUsize,
}

#[cfg(all(not(feature = "std"), not(test)))]
unsafe impl Sync for SimpleBumpAllocator {}

#[cfg(all(not(feature = "std"), not(test)))]
unsafe impl core::alloc::GlobalAlloc for SimpleBumpAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();
        let mut cur = self.offset.load(core::sync::atomic::Ordering::Relaxed);
        loop {
            let aligned = (cur + align - 1) & !(align - 1);
            let next = aligned + size;
            if next > WSH_HEAP_SIZE {
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

#[cfg(all(not(feature = "std"), not(test)))]
#[global_allocator]
static ALLOCATOR: SimpleBumpAllocator = SimpleBumpAllocator {
    heap: core::cell::UnsafeCell::new([0u8; WSH_HEAP_SIZE]),
    offset: core::sync::atomic::AtomicUsize::new(0),
};

#[cfg(all(not(feature = "std"), not(test)))]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    run_interactive_shell();
    libwunix::exit(0);
}

fn get_history_file_path() -> String {
    if let Some(histfile) = platform::get_env("HISTFILE") {
        let trimmed = histfile.trim();
        if !trimmed.is_empty() {
            return String::from(trimmed);
        }
    }

    if let Some(home) = platform::get_env("HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            if trimmed == "/" {
                return String::from("/.wsh_history");
            } else if trimmed.ends_with('/') {
                let mut p = String::from(trimmed);
                p.push_str(".wsh_history");
                return p;
            } else {
                let mut p = String::from(trimmed);
                p.push_str("/.wsh_history");
                return p;
            }
        }
    }

    #[cfg(not(feature = "std"))]
    let is_root = libwunix::geteuid() == 0;
    #[cfg(feature = "std")]
    let is_root = false;

    if is_root {
        String::from("/root/.wsh_history")
    } else {
        String::from("/home/.wsh_history")
    }
}

fn run_interactive_shell() {
    let hist_capacity = platform::get_env("HISTSIZE")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1000);
    let hist_path = get_history_file_path();
    let mut history = History::with_file(hist_capacity, Some(hist_path));
    history.load_default_file();
    let mut last_status = 0;
    let mut should_exit = false;

    let euid = {
        #[cfg(not(feature = "std"))]
        {
            libwunix::geteuid()
        }
        #[cfg(feature = "std")]
        {
            0
        }
    };
    let is_root = euid == 0;

    // Переходим в домашний каталог пользователя при старте в корне
    if platform::get_cwd() == "/" {
        let default_home = if is_root { "/root" } else { "/home" };
        let home = platform::get_env("HOME").unwrap_or_else(|| String::from(default_home));
        let _ = platform::set_cwd(&home);
    }

    // Локализованная подсказка командной строки Wunix
    #[cfg(not(feature = "std"))]
    let banner = libwunix::t(
        "\x1b[37mВведите \x1b[1;32m'help'\x1b[0m\x1b[37m для списка команд или \x1b[1;32m'exit'\x1b[0m\x1b[37m для выхода.\x1b[0m\n\n",
        "\x1b[37mType \x1b[1;32m'help'\x1b[0m\x1b[37m for available commands or \x1b[1;32m'exit'\x1b[0m\x1b[37m to quit.\x1b[0m\n\n",
    );
    #[cfg(feature = "std")]
    let banner = "\x1b[37mType \x1b[1;32m'help'\x1b[0m\x1b[37m for available commands or \x1b[1;32m'exit'\x1b[0m\x1b[37m to quit.\x1b[0m\n\n";

    print_out(banner);

    loop {
        let cwd = platform::get_cwd();
        let euid = {
            #[cfg(not(feature = "std"))]
            {
                libwunix::geteuid()
            }
            #[cfg(feature = "std")]
            {
                0
            }
        };
        let is_root = euid == 0;
        let default_user = if is_root { "root" } else { "user" };
        let default_home = if is_root { "/root" } else { "/home" };
        let user = platform::get_env("USER").unwrap_or_else(|| String::from(default_user));
        let host = platform::get_env("HOSTNAME").unwrap_or_else(|| String::from("wunix"));
        let home = platform::get_env("HOME").unwrap_or_else(|| String::from(default_home));

        let prompt = Prompt::build(&user, &host, &cwd, &home, is_root, last_status);

        let line = match LineEditor::read_line(&prompt, &mut history) {
            Some(l) => l,
            None => {
                // Получен EOF (Ctrl+D)
                print_out("exit\n");
                break;
            }
        };

        if line == "\x03" {
            history.reset_cursor();
            last_status = 130;
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            history.reset_cursor();
            last_status = 0;
            continue;
        }

        if history.add(trimmed) {
            history.append_default_file(trimmed);
        }
        last_status = run_command_line(trimmed, &mut history, last_status, &mut should_exit);

        if should_exit {
            break;
        }
    }

    history.save_default_file();

    #[cfg(feature = "std")]
    std::process::exit(last_status);
}

fn run_command_line(line: &str, history: &mut History, mut last_status: i32, should_exit: &mut bool) -> i32 {
    let env_lookup = |key: &str| platform::get_env(key);
    let tokens = Parser::tokenize(line);
    let chain = match Parser::parse_chain(&tokens, &env_lookup, last_status) {
        Some(c) => c,
        None => return 0,
    };

    let mut skip_cond = None;

    for item in chain.pipelines {
        if *should_exit {
            break;
        }

        if let Some(cond) = skip_cond.take() {
            match cond {
                wsh::parser::ChainOp::And if last_status != 0 => {
                    // Предыдущая команда завершилась ошибкой: пропускаем ветку &&
                    skip_cond = Some(item.op);
                    continue;
                }
                wsh::parser::ChainOp::Or if last_status == 0 => {
                    // Предыдущая команда завершилась успехом: пропускаем ветку ||
                    skip_cond = Some(item.op);
                    continue;
                }
                _ => {}
            }
        }

        last_status = execute_single_pipeline(&item.pipeline, history, last_status, should_exit);

        if item.op != wsh::parser::ChainOp::Always {
            skip_cond = Some(item.op);
        }
    }

    last_status
}

fn execute_single_pipeline(
    pipeline: &wsh::parser::Pipeline,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> i32 {
    if pipeline.commands.is_empty() {
        return 0;
    }

    // Если пайплайн состоит из одной команды, проверяем, не является ли она встроенной
    if pipeline.commands.len() == 1 {
        let single_cmd = &pipeline.commands[0];
        if let Some(code) = Builtins::execute(single_cmd, history, last_status, should_exit) {
            return code;
        }
    }

    // В противном случае запускаем реальный конвейер процессов через платформу
    platform::execute_pipeline(pipeline)
}

fn print_out(s: &str) {
    #[cfg(feature = "std")]
    {
        use std::io::Write;
        let _ = std::io::stdout().write_all(s.as_bytes());
        let _ = std::io::stdout().flush();
    }
    #[cfg(not(feature = "std"))]
    {
        libwunix::write(libwunix::STDOUT_FILENO, s.as_bytes());
    }
}

#[cfg(all(not(feature = "std"), not(test)))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    libwunix::write(libwunix::STDERR_FILENO, b"panic in wsh\n");
    libwunix::exit(1);
}
