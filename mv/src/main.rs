#![no_std]
#![no_main]

mod cli;
mod io;
mod mover;
mod path;

use cli::{parse_args, print_help, ParseResult, MAX_OPERANDS};
use io::print_err;
use libwunix::{exit, t, write, STDERR_FILENO};
use mover::{is_existing_dir, move_entry, move_into_dir};

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        print_err(t(
            "mv: пропущен операнд файла\nПопробуйте 'mv --help' для получения дополнительной информации.\n",
            "mv: missing file operand\nTry 'mv --help' for more information.\n",
        ));
        exit(1);
    }

    let mut raw_args: [&[u8]; MAX_OPERANDS + 16] = [b""; MAX_OPERANDS + 16];
    let mut count = 0usize;
    for i in 1..argc {
        if count >= raw_args.len() {
            break;
        }
        let ptr = unsafe { *argv.add(i as usize) };
        if ptr.is_null() {
            continue;
        }
        let mut len = 0usize;
        unsafe {
            while *ptr.add(len) != 0 && len < 1024 {
                len += 1;
            }
            raw_args[count] = core::slice::from_raw_parts(ptr, len);
        }
        count += 1;
    }

    let parsed = match parse_args(&raw_args[..count]) {
        ParseResult::Run(p) => p,
        ParseResult::Help => {
            print_help();
            exit(0);
        }
        ParseResult::Error => exit(1),
    };

    if parsed.operand_count == 0 {
        print_err(t(
            "mv: пропущен операнд файла\nПопробуйте 'mv --help' для получения дополнительной информации.\n",
            "mv: missing file operand\nTry 'mv --help' for more information.\n",
        ));
        exit(1);
    }

    if parsed.operand_count == 1 {
        print_err(t("mv: после '", "mv: missing destination file operand after '"));
        write(STDERR_FILENO, parsed.operands[0]);
        print_err(t(
            "' пропущен операнд, задающий целевой файл\nПопробуйте 'mv --help' для получения дополнительной информации.\n",
            "'\nTry 'mv --help' for more information.\n",
        ));
        exit(1);
    }

    let code = execute(&parsed.operands[..parsed.operand_count], parsed.opts);
    exit(code as u64);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in mv\n");
    exit(1);
}

fn execute(operands: &[&[u8]], opts: cli::Options) -> i32 {
    let dst = operands[operands.len() - 1];
    let sources = &operands[..operands.len() - 1];

    if sources.len() > 1 {
        if opts.no_target_dir || !is_existing_dir(dst) {
            print_err(t("mv: указанная цель '", "mv: target '"));
            write(STDERR_FILENO, dst);
            print_err(t("' не является каталогом\n", "' is not a directory\n"));
            return 1;
        }
        let mut ok = true;
        for &src in sources {
            if !move_into_dir(src, dst, opts) {
                ok = false;
            }
        }
        return if ok { 0 } else { 1 };
    }

    let src = sources[0];
    let ok = if !opts.no_target_dir && is_existing_dir(dst) {
        move_into_dir(src, dst, opts)
    } else {
        move_entry(src, dst, opts)
    };
    if ok { 0 } else { 1 }
}
