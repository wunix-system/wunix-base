#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{format, string::String};

#[cfg(feature = "std")]
use std::string::String;

use crate::history::History;
use crate::parser::SimpleCommand;
use crate::platform;

pub struct Builtins;

impl Builtins {
    pub fn is_builtin(name: &str) -> bool {
        matches!(
            name,
            "cd" | "pwd" | "exit" | "echo" | "export" | "unset" | "env" | "history" | "clear" | "cls" | "help" | "type" | "which" | "whoami" | "hostname"
        )
    }

    /// Выполняет встроенную команду шелла.
    /// Возвращает Some(exit_code), если команда была встроенной, или None, если это внешняя программа.
    pub fn execute(
        cmd: &SimpleCommand,
        history: &mut History,
        last_status: i32,
        should_exit: &mut bool,
    ) -> Option<i32> {
        match cmd.name.as_str() {
            "cd" => Some(Self::builtin_cd(&cmd.args)),
            "pwd" => Some(Self::builtin_pwd()),
            "exit" => {
                *should_exit = true;
                let code = cmd
                    .args
                    .first()
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(last_status);
                Some(code)
            }
            "echo" => Some(Self::builtin_echo(&cmd.args)),
            "export" => Some(Self::builtin_export(&cmd.args)),
            "unset" => Some(Self::builtin_unset(&cmd.args)),
            "env" => Some(Self::builtin_env()),
            "history" => Some(Self::builtin_history(history, &cmd.args)),
            "clear" | "cls" => {
                print_out("\x1b[2J\x1b[H");
                Some(0)
            }
            "whoami" => Some(Self::builtin_whoami()),
            "hostname" => Some(Self::builtin_hostname()),
            "help" => Some(Self::builtin_help()),
            "type" | "which" => Some(Self::builtin_type(&cmd.args)),
            _ => None,
        }
    }

    fn builtin_cd(args: &[String]) -> i32 {
        let target = if args.is_empty() {
            platform::get_env("HOME").unwrap_or_else(|| String::from("/home/user"))
        } else if args[0] == "~" {
            platform::get_env("HOME").unwrap_or_else(|| String::from("/home/user"))
        } else if args[0] == "-" {
            match platform::get_env("OLDPWD") {
                Some(old) => {
                    print_out(&old);
                    print_out("\n");
                    old
                }
                None => {
                    #[cfg(not(feature = "std"))]
                    print_err(libwunix::t("wsh: cd: переменная OLDPWD не задана\n", "wsh: cd: OLDPWD not set\n"));
                    #[cfg(feature = "std")]
                    print_err("wsh: cd: OLDPWD not set\n");
                    return 1;
                }
            }
        } else {
            args[0].clone()
        };

        let current_pwd = platform::get_cwd();
        match platform::set_cwd(&target) {
            Ok(()) => {
                platform::set_env("OLDPWD", &current_pwd);
                platform::set_env("PWD", &platform::get_cwd());
                0
            }
            Err(e) => {
                print_err(&format!("wsh: cd: {}: {}\n", target, e));
                1
            }
        }
    }

    fn builtin_pwd() -> i32 {
        let cwd = platform::get_cwd();
        print_out(&cwd);
        print_out("\n");
        0
    }

    fn builtin_echo(args: &[String]) -> i32 {
        if args.is_empty() {
            print_out("\n");
            return 0;
        }

        let mut start_idx = 0;
        let mut no_newline = false;
        if args[0] == "-n" {
            no_newline = true;
            start_idx = 1;
        }

        for i in start_idx..args.len() {
            print_out(&args[i]);
            if i + 1 < args.len() {
                print_out(" ");
            }
        }

        if !no_newline {
            print_out("\n");
        }
        0
    }

    fn builtin_export(args: &[String]) -> i32 {
        if args.is_empty() {
            return Self::builtin_env();
        }

        for arg in args {
            if let Some((k, v)) = arg.split_once('=') {
                platform::set_env(k, v);
            } else if let Some(val) = platform::get_env(arg) {
                platform::set_env(arg, &val);
            }
        }
        0
    }

    fn builtin_unset(args: &[String]) -> i32 {
        for arg in args {
            platform::unset_env(arg);
        }
        0
    }

    fn builtin_env() -> i32 {
        let vars = platform::get_all_env();
        for (k, v) in vars {
            print_out(&format!("{}={}\n", k, v));
        }
        0
    }

    fn builtin_history(history: &mut History, args: &[String]) -> i32 {
        if let Some(arg) = args.first() {
            match arg.as_str() {
                "--help" | "-h" => {
                    let msg = t(
                        "Использование: history [N] | history [-c] [-d СМЕЩЕНИЕ] [-w [ФАЙЛ]] [-r [ФАЙЛ]] [-a [ФАЙЛ]]\n\
                         Управление историей команд Wunix Shell.\n\n\
                         Опции:\n\
                           N           Показать последние N команд\n\
                           -c          Очистить историю команд\n\
                           -d СМЕЩ    Удалить запись по номеру\n\
                           -w [ФАЙЛ]   Сохранить историю в файл\n\
                           -r [ФАЙЛ]   Загрузить историю из файла\n\
                           -a [ФАЙЛ]   Добавить историю в файл\n\
                           --help      Показать эту справку\n",
                        "Usage: history [N] | history [-c] [-d OFFSET] [-w [FILE]] [-r [FILE]] [-a [FILE]]\n\
                         Display or manipulate the command history list.\n\n\
                         Options:\n\
                           N           Display last N commands\n\
                           -c          Clear the history list\n\
                           -d OFFSET   Delete history entry at OFFSET\n\
                           -w [FILE]   Write current history to file\n\
                           -r [FILE]   Read history from file\n\
                           -a [FILE]   Append history to file\n\
                           --help      Display this help\n",
                    );
                    print_out(msg);
                    return 0;
                }
                "-c" => {
                    history.clear();
                    let _ = history.save_default_file();
                    return 0;
                }
                "-d" => {
                    if let Some(offset_str) = args.get(1) {
                        if let Ok(idx) = offset_str.parse::<usize>() {
                            if idx >= 1 && idx <= history.len() {
                                history.remove_at(idx - 1);
                                let _ = history.save_default_file();
                                return 0;
                            }
                        }
                    }
                    print_err(t(
                        "history: неверный номер записи\n",
                        "history: invalid entry number\n",
                    ));
                    return 1;
                }
                "-w" => {
                    let path = args.get(1).map(|s| s.as_str());
                    let ok = if let Some(p) = path {
                        history.save_file(p)
                    } else {
                        history.save_default_file()
                    };
                    if !ok {
                        print_err(t(
                            "history: ошибка записи в файл\n",
                            "history: failed to write to file\n",
                        ));
                        return 1;
                    }
                    return 0;
                }
                "-r" => {
                    let path = args.get(1).map(|s| s.as_str());
                    let ok = if let Some(p) = path {
                        history.load_file(p)
                    } else {
                        history.load_default_file()
                    };
                    if !ok {
                        print_err(t(
                            "history: ошибка чтения из файла\n",
                            "history: failed to read from file\n",
                        ));
                        return 1;
                    }
                    return 0;
                }
                "-a" => {
                    let path = args.get(1).map(|s| s.as_str());
                    if let Some(p) = path {
                        for entry in history.entries() {
                            let _ = history.append_entry_to_file(entry, p);
                        }
                    } else {
                        for entry in history.entries() {
                            let _ = history.append_default_file(entry);
                        }
                    }
                    return 0;
                }
                _ => {}
            }
        }

        let count_filter = args.first().and_then(|s| s.parse::<usize>().ok());
        let entries = history.entries();
        let total = entries.len();
        let start_idx = match count_filter {
            Some(n) if n < total => total - n,
            _ => 0,
        };

        for (idx, entry) in entries[start_idx..].iter().enumerate() {
            let num = start_idx + idx + 1;
            print_out(&format!(" {:5}  {}\n", num, entry));
        }
        0
    }

    fn builtin_whoami() -> i32 {
        let user = platform::get_env("USER").unwrap_or_else(|| String::from("user"));
        print_out(&user);
        print_out("\n");
        0
    }

    fn builtin_hostname() -> i32 {
        let host = platform::get_env("HOSTNAME").unwrap_or_else(|| String::from("wunix"));
        print_out(&host);
        print_out("\n");
        0
    }

    fn builtin_type(args: &[String]) -> i32 {
        if args.is_empty() {
            #[cfg(not(feature = "std"))]
            print_err(libwunix::t("wsh: type: пропущен аргумент\n", "wsh: type: missing operand\n"));
            #[cfg(feature = "std")]
            print_err("wsh: type: missing operand\n");
            return 1;
        }

        #[allow(unused_mut)]
        let mut status = 0;
        for name in args {
            if Self::is_builtin(name) {
                #[cfg(not(feature = "std"))]
                {
                    let desc = libwunix::t("является встроенной командой шелла", "is a shell builtin");
                    print_out(&format!("{} {}\n", name, desc));
                }
                #[cfg(feature = "std")]
                print_out(&format!("{} is a shell builtin\n", name));
            } else {
                // Ищем в PATH
                #[cfg(feature = "std")]
                {
                    if let Ok(path) = std::env::var("PATH") {
                        let mut found = false;
                        for dir in path.split(':') {
                            let p = std::path::Path::new(dir).join(name);
                            if p.is_file() {
                                print_out(&format!("{} is {}\n", name, p.display()));
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            print_err(&format!("wsh: type: {}: not found\n", name));
                            status = 1;
                        }
                    }
                }
                #[cfg(not(feature = "std"))]
                {
                    let mut path_buf = [0u8; 128];
                    let prefix = b"/bin/";
                    let mut exists = false;
                    if prefix.len() + name.len() + 1 < path_buf.len() {
                        path_buf[..prefix.len()].copy_from_slice(prefix);
                        path_buf[prefix.len()..prefix.len() + name.len()].copy_from_slice(name.as_bytes());
                        path_buf[prefix.len() + name.len()] = 0;
                        let mut st = libwunix::Stat::zeroed();
                        if libwunix::stat(&path_buf[..prefix.len() + name.len() + 1], &mut st) == 0 {
                            exists = true;
                        }
                    }

                    if exists {
                        let is_str = libwunix::t("это", "is");
                        print_out(&format!("{} {} /bin/{}\n", name, is_str, name));
                    } else {
                        let not_found_str = libwunix::t("не найдено", "not found");
                        print_err(&format!("wsh: type: {}: {}\n", name, not_found_str));
                        status = 1;
                    }
                }
            }
        }
        status
    }

    fn builtin_help() -> i32 {
        #[cfg(not(feature = "std"))]
        {
            print_out(libwunix::t(
                "\x1b[1;36mВстроенные команды wsh:\x1b[0m\n  \x1b[1;32mcd\x1b[0m [путь]             Сменить текущий рабочий каталог\n  \x1b[1;32mpwd\x1b[0m                   Показать текущий рабочий каталог\n  \x1b[1;32mecho\x1b[0m [-n] [текст...]   Вывести переданный текст на экран\n  \x1b[1;32mclear\x1b[0m, \x1b[1;32mcls\x1b[0m           Очистить экран терминала\n  \x1b[1;32mexport\x1b[0m [КЛЮЧ=ЗНАЧ]     Установить переменную окружения\n  \x1b[1;32munset\x1b[0m [КЛЮЧ...]        Удалить переменную окружения\n  \x1b[1;32menv\x1b[0m                   Вывести список переменных окружения\n  \x1b[1;32mhistory\x1b[0m               Показать историю введенных команд\n  \x1b[1;32mwhoami\x1b[0m                Показать имя пользователя\n  \x1b[1;32mhostname\x1b[0m              Показать имя устройства\n  \x1b[1;32mtype\x1b[0m [имя...]         Показать тип и расположение команды\n  \x1b[1;32mhelp\x1b[0m                  Показать эту справочную информацию\n  \x1b[1;32mexit\x1b[0m [код]             Завершить сессию командной строки\n\n\x1b[1;36mГорячие клавиши:\x1b[0m\n  \x1b[1mTab\x1b[0m           Автодополнение команд и путей\n  \x1b[1mCtrl+C\x1b[0m        Прервать ввод / завершить запущенную программу\n  \x1b[1mCtrl+D\x1b[0m        Конец файла (EOF) / выход\n  \x1b[1mCtrl+L\x1b[0m        Очистить экран консоли\n  \x1b[1mCtrl+A/E\x1b[0m      Начало / конец строки\n  \x1b[1mCtrl+U/K\x1b[0m      Удалить до начала / конца строки\n  \x1b[1mCtrl+W\x1b[0m        Удалить слово перед курсором\n  \x1b[1mCtrl+Y\x1b[0m        Вставить из буфера вырезания\n  \x1b[1mAlt+B/F\x1b[0m       Слово назад / вперёд\n  \x1b[1m↑ ↓\x1b[0m           История команд\n  \x1b[1mShift+←→\x1b[0m      Выделение текста\n  \x1b[1mCtrl+Shift+C/V\x1b[0m Копировать / вставить\n",
                "\x1b[1;36mWunix Shell Builtin Commands:\x1b[0m\n  \x1b[1;32mcd\x1b[0m [dir]              Change working directory\n  \x1b[1;32mpwd\x1b[0m                   Print current working directory\n  \x1b[1;32mecho\x1b[0m [-n] [text...]    Display arguments as text\n  \x1b[1;32mclear\x1b[0m, \x1b[1;32mcls\x1b[0m           Clear terminal display\n  \x1b[1;32mexport\x1b[0m [KEY=VAL]       Set environment variable\n  \x1b[1;32munset\x1b[0m [KEY...]         Remove environment variable\n  \x1b[1;32menv\x1b[0m                   List exported environment variables\n  \x1b[1;32mhistory\x1b[0m               Display session command history\n  \x1b[1;32mwhoami\x1b[0m                Display current username\n  \x1b[1;32mhostname\x1b[0m              Display device hostname\n  \x1b[1;32mtype\x1b[0m [name...]        Display information about command type\n  \x1b[1;32mhelp\x1b[0m                  Display this help overview\n  \x1b[1;32mexit\x1b[0m [code]           Terminate shell session\n\n\x1b[1;36mKeyboard Shortcuts:\x1b[0m\n  \x1b[1mTab\x1b[0m           Autocomplete commands and file paths\n  \x1b[1mCtrl+C\x1b[0m        Interrupt input / abort running child process\n  \x1b[1mCtrl+D\x1b[0m        End-of-file (EOF) / exit\n  \x1b[1mCtrl+L\x1b[0m        Clear terminal screen\n  \x1b[1mCtrl+A/E\x1b[0m      Beginning / end of line\n  \x1b[1mCtrl+U/K\x1b[0m      Kill to line start / end\n  \x1b[1mCtrl+W\x1b[0m        Delete word backward\n  \x1b[1mCtrl+Y\x1b[0m        Yank (paste) from kill ring\n  \x1b[1mAlt+B/F\x1b[0m       Word backward / forward\n  \x1b[1m↑ ↓\x1b[0m           Navigate command history\n  \x1b[1mShift+←→\x1b[0m      Select text\n  \x1b[1mCtrl+Shift+C/V\x1b[0m Copy / paste\n",
            ));
        }
        #[cfg(feature = "std")]
        {
            print_out("Wunix Shell Builtin Commands:\n  cd [dir]           Change working directory\n  pwd                Print current working directory\n  echo [-n] [args]   Display arguments as text\n  clear, cls         Clear terminal display\n  export [KEY=VAL]   Set environment variable\n  unset [KEY...]     Remove environment variable\n  env                List exported environment variables\n  history            Display session command history\n  whoami             Display current username\n  hostname           Display device hostname\n  type [name...]     Display information about command type\n  help               Display this help overview\n  exit [code]        Terminate shell session\n\nKeyboard Shortcuts:\n  Tab           Autocomplete commands and paths\n  Ctrl+C        Interrupt / abort\n  Ctrl+D        EOF / exit\n  Ctrl+L        Clear screen\n  Ctrl+A/E      Begin / end of line\n  Ctrl+U/K      Kill to start / end\n  Ctrl+W        Delete word backward\n  Ctrl+Y        Yank from kill ring\n  Alt+B/F       Word backward / forward\n  Up/Down       Command history\n  Shift+Arrows  Select text\n  Ctrl+Shift+C/V Copy / paste\n");
        }
        0
    }
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

fn print_err(s: &str) {
    #[cfg(feature = "std")]
    {
        use std::io::Write;
        let _ = std::io::stderr().write_all(s.as_bytes());
        let _ = std::io::stderr().flush();
    }
    #[cfg(not(feature = "std"))]
    {
        libwunix::write(libwunix::STDERR_FILENO, s.as_bytes());
    }
}

fn t(ru: &'static str, en: &'static str) -> &'static str {
    #[cfg(not(feature = "std"))]
    {
        libwunix::t(ru, en)
    }
    #[cfg(feature = "std")]
    {
        ru
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SimpleCommand;

    #[test]
    fn test_builtin_history_execution() {
        let mut history = History::new(10);
        history.add("echo one");
        history.add("echo two");
        history.add("echo three");

        let mut should_exit = false;
        let cmd = SimpleCommand {
            name: String::from("history"),
            args: vec![],
            redirects: vec![],
        };
        let res = Builtins::execute(&cmd, &mut history, 0, &mut should_exit);
        assert_eq!(res, Some(0));

        let cmd_n = SimpleCommand {
            name: String::from("history"),
            args: vec![String::from("2")],
            redirects: vec![],
        };
        let res_n = Builtins::execute(&cmd_n, &mut history, 0, &mut should_exit);
        assert_eq!(res_n, Some(0));

        let cmd_d = SimpleCommand {
            name: String::from("history"),
            args: vec![String::from("-d"), String::from("2")],
            redirects: vec![],
        };
        let res_d = Builtins::execute(&cmd_d, &mut history, 0, &mut should_exit);
        assert_eq!(res_d, Some(0));
        assert_eq!(history.entries(), &["echo one", "echo three"]);

        let cmd_c = SimpleCommand {
            name: String::from("history"),
            args: vec![String::from("-c")],
            redirects: vec![],
        };
        let res_c = Builtins::execute(&cmd_c, &mut history, 0, &mut should_exit);
        assert_eq!(res_c, Some(0));
        assert!(history.is_empty());
    }
}
