#![no_std]
#![no_main]

use libwunix::{exit, println, t, uname, write, STDERR_FILENO, STDOUT_FILENO};

const UTS_FIELD_SIZE: usize = 65;

// Структура utsname согласно POSIX и стандарту Linux x86_64 ABI.
#[repr(C)]
struct UtsName {
    sysname: [u8; UTS_FIELD_SIZE],
    nodename: [u8; UTS_FIELD_SIZE],
    release: [u8; UTS_FIELD_SIZE],
    version: [u8; UTS_FIELD_SIZE],
    machine: [u8; UTS_FIELD_SIZE],
    domainname: [u8; UTS_FIELD_SIZE],
}

impl UtsName {
    const fn zeroed() -> Self {
        Self {
            sysname: [0; UTS_FIELD_SIZE],
            nodename: [0; UTS_FIELD_SIZE],
            release: [0; UTS_FIELD_SIZE],
            version: [0; UTS_FIELD_SIZE],
            machine: [0; UTS_FIELD_SIZE],
            domainname: [0; UTS_FIELD_SIZE],
        }
    }
}

fn cstr_to_str(buf: &[u8]) -> &str {
    let mut len = 0;
    while len < buf.len() && buf[len] != 0 {
        len += 1;
    }
    core::str::from_utf8(&buf[..len]).unwrap_or("")
}

fn cstr_from_ptr(ptr: *const u8) -> &'static [u8] {
    if ptr.is_null() {
        return &[];
    }
    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 && len < 4096 {
            len += 1;
        }
        core::slice::from_raw_parts(ptr, len)
    }
}

#[derive(Default)]
struct UnameFlags {
    sysname: bool,
    nodename: bool,
    release: bool,
    version: bool,
    machine: bool,
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut flags = UnameFlags::default();
    let mut any_flag = false;

    let mut i = 1;
    while i < argc {
        let arg = cstr_from_ptr(unsafe { *argv.add(i as usize) });
        if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование: uname [ОПЦИИ]...\n\
                     Вывод системной информации.\n\
                     Без ОПЦИЙ эквивалентно -s.\n\n\
                     Опции:\n  \
                       -a, --all                вывести всю информацию\n  \
                       -s, --kernel-name        вывести имя ядра\n  \
                       -n, --nodename           вывести сетевое имя узла\n  \
                       -r, --kernel-release     вывести релиз ядра\n  \
                       -v, --kernel-version     вывести версию ядра\n  \
                       -m, --machine            вывести аппаратный тип процессора\n  \
                       -o, --operating-system   вывести имя операционной системы\n  \
                       -h, --help               показать эту справку и выйти\n  \
                       -V, --version            вывести информацию о версии и выйти",
                    "Usage: uname [OPTION]...\n\
                     Print certain system information.  With no OPTION, same as -s.\n\n\
                     Options:\n  \
                       -a, --all                print all information\n  \
                       -s, --kernel-name        print the kernel name\n  \
                       -n, --nodename           print the network node hostname\n  \
                       -r, --kernel-release     print the kernel release\n  \
                       -v, --kernel-version     print the kernel version\n  \
                       -m, --machine            print the machine hardware name\n  \
                       -o, --operating-system   print the operating system\n  \
                       -h, --help               display this help and exit\n  \
                       -V, --version            output version information and exit"
                )
            );
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("uname (Wunix coreutils) 9.4");
            exit(0);
        } else if arg == b"-a" || arg == b"--all" {
            flags.sysname = true;
            flags.nodename = true;
            flags.release = true;
            flags.version = true;
            flags.machine = true;
            any_flag = true;
        } else if arg.starts_with(b"-") && !arg.starts_with(b"--") {
            for &ch in &arg[1..] {
                match ch {
                    b'a' => {
                        flags.sysname = true;
                        flags.nodename = true;
                        flags.release = true;
                        flags.version = true;
                        flags.machine = true;
                        any_flag = true;
                    }
                    b's' => { flags.sysname = true; any_flag = true; }
                    b'n' => { flags.nodename = true; any_flag = true; }
                    b'r' => { flags.release = true; any_flag = true; }
                    b'v' => { flags.version = true; any_flag = true; }
                    b'm' => { flags.machine = true; any_flag = true; }
                    b'o' => { flags.sysname = true; any_flag = true; }
                    _ => {}
                }
            }
        } else if arg == b"--kernel-name" {
            flags.sysname = true;
            any_flag = true;
        } else if arg == b"--nodename" {
            flags.nodename = true;
            any_flag = true;
        } else if arg == b"--kernel-release" {
            flags.release = true;
            any_flag = true;
        } else if arg == b"--kernel-version" {
            flags.version = true;
            any_flag = true;
        } else if arg == b"--machine" {
            flags.machine = true;
            any_flag = true;
        } else if arg == b"--operating-system" {
            flags.sysname = true;
            any_flag = true;
        }
        i += 1;
    }

    // По стандарту POSIX: если опции не переданы, выводится только имя ядра (-s)
    if !any_flag {
        flags.sysname = true;
    }

    let mut uts = UtsName::zeroed();
    let ret = uname(&mut uts as *mut _ as *mut u8);

    if ret != 0 {
        write(
            STDERR_FILENO,
            t(
                "uname: не удалось получить системную информацию\n",
                "uname: cannot get system information\n",
            )
            .as_bytes(),
        );
        exit(1);
    }

    let mut first = true;
    let mut print_field = |val: &str| {
        if !val.is_empty() {
            if !first {
                write(STDOUT_FILENO, b" ");
            }
            write(STDOUT_FILENO, val.as_bytes());
            first = false;
        }
    };

    if flags.sysname {
        print_field(cstr_to_str(&uts.sysname));
    }
    if flags.nodename {
        print_field(cstr_to_str(&uts.nodename));
    }
    if flags.release {
        print_field(cstr_to_str(&uts.release));
    }
    if flags.version {
        print_field(cstr_to_str(&uts.version));
    }
    if flags.machine {
        print_field(cstr_to_str(&uts.machine));
    }

    write(STDOUT_FILENO, b"\n");
    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in uname\n");
    exit(1);
}
