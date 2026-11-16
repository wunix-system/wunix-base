#![no_std]
#![no_main]

mod action;
mod format;

use action::{run_id, IdOptions};
use libwunix::{exit, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO};

fn print_help() {
    let help_msg = t(
        "Использование: id [ОПЦИЯ]... [ПОЛЬЗОВАТЕЛЬ]\n\
         Выводит информацию о пользователе и группах для текущего пользователя или указанного ПОЛЬЗОВАТЕЛЯ.\n\n\
         Опции:\n\
           -g, --group    вывести только эффективный GID\n\
           -G, --groups   вывести все GID групп\n\
           -n, --name     вывести имя вместо числа (с -u, -g, -G)\n\
           -r, --real     вывести реальный ID вместо эффективного (с -u, -g, -G)\n\
           -u, --user     вывести только эффективный UID\n\
           -h, --help     показать эту справку и выйти\n",
        "Usage: id [OPTION]... [USER]\n\
         Print user and group information for the current user, or for the specified USER.\n\n\
         Options:\n\
           -g, --group    print only the effective group ID\n\
           -G, --groups   print all group IDs\n\
           -n, --name     print a name instead of a number, for -ugG\n\
           -r, --real     print the real ID instead of the effective ID, with -ugG\n\
           -u, --user     print only the effective user ID\n\
           -h, --help     display this help and exit\n",
    );
    write(STDOUT_FILENO, help_msg.as_bytes());
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut specified_user = None;
    let mut opts = IdOptions::default();

    for i in 1..argc {
        let p = unsafe { *argv.add(i as usize) };
        if p.is_null() {
            continue;
        }
        let len = str_utils::strlen(p);
        let arg = core::str::from_utf8(unsafe { core::slice::from_raw_parts(p, len) }).unwrap_or("");
        if arg == "--help" || arg == "-h" {
            print_help();
            exit(0);
        } else if arg == "--user" {
            opts.only_user = true;
        } else if arg == "--group" {
            opts.only_group = true;
        } else if arg == "--groups" {
            opts.only_groups = true;
        } else if arg == "--name" {
            opts.name = true;
        } else if arg == "--real" {
            opts.real = true;
        } else if arg.starts_with('-') && arg.len() > 1 {
            for b in arg.bytes().skip(1) {
                match b {
                    b'u' => opts.only_user = true,
                    b'g' => opts.only_group = true,
                    b'G' => opts.only_groups = true,
                    b'n' => opts.name = true,
                    b'r' => opts.real = true,
                    b'h' => {
                        print_help();
                        exit(0);
                    }
                    _ => {}
                }
            }
        } else if specified_user.is_none() {
            specified_user = Some(arg);
        }
    }

    run_id(specified_user, opts);
    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in id\n");
    exit(1);
}
