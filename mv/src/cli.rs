use crate::io::{print_err, print_out};
use libwunix::t;

pub const MAX_OPERANDS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverwriteMode {
    Default,
    Force,
    Interactive,
    NoClobber,
}

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub overwrite: OverwriteMode,
    pub update: bool,
    pub verbose: bool,
    pub no_target_dir: bool,
}

impl Options {
    pub const fn new() -> Self {
        Self {
            overwrite: OverwriteMode::Default,
            update: false,
            verbose: false,
            no_target_dir: false,
        }
    }
}

pub struct ParsedArgs<'a> {
    pub opts: Options,
    pub operands: [&'a [u8]; MAX_OPERANDS],
    pub operand_count: usize,
}

pub enum ParseResult<'a> {
    Run(ParsedArgs<'a>),
    Help,
    Error,
}

pub fn parse_args<'a>(raw_args: &[&'a [u8]]) -> ParseResult<'a> {
    let mut opts = Options::new();
    let mut operands: [&[u8]; MAX_OPERANDS] = [b""; MAX_OPERANDS];
    let mut operand_count = 0usize;
    let mut end_of_options = false;

    for &arg in raw_args {
        if !end_of_options && arg == b"--" {
            end_of_options = true;
            continue;
        }

        if !end_of_options && arg.starts_with(b"--") {
            match parse_long_option(arg, &mut opts) {
                LongOpt::Help => return ParseResult::Help,
                LongOpt::Ok => continue,
                LongOpt::Unknown => {
                    print_err(t("mv: неизвестная опция '", "mv: unrecognized option '"));
                    libwunix::write(libwunix::STDERR_FILENO, arg);
                    print_err(t(
                        "'\nПопробуйте 'mv --help' для получения дополнительной информации.\n",
                        "'\nTry 'mv --help' for more information.\n",
                    ));
                    return ParseResult::Error;
                }
            }
        }

        if !end_of_options && arg.starts_with(b"-") && arg.len() > 1 {
            if let Err(bad) = parse_short_options(&arg[1..], &mut opts) {
                print_err(t("mv: неверная опция -- '", "mv: invalid option -- '"));
                libwunix::write(libwunix::STDERR_FILENO, &[bad]);
                print_err(t(
                    "'\nПопробуйте 'mv --help' для получения дополнительной информации.\n",
                    "'\nTry 'mv --help' for more information.\n",
                ));
                return ParseResult::Error;
            }
            continue;
        }

        if operand_count >= MAX_OPERANDS {
            print_err(t("mv: слишком много операндов\n", "mv: too many operands\n"));
            return ParseResult::Error;
        }
        operands[operand_count] = arg;
        operand_count += 1;
    }

    ParseResult::Run(ParsedArgs {
        opts,
        operands,
        operand_count,
    })
}

enum LongOpt {
    Ok,
    Help,
    Unknown,
}

fn parse_long_option(arg: &[u8], opts: &mut Options) -> LongOpt {
    match arg {
        b"--help" => LongOpt::Help,
        b"--force" => {
            opts.overwrite = OverwriteMode::Force;
            LongOpt::Ok
        }
        b"--interactive" => {
            opts.overwrite = OverwriteMode::Interactive;
            LongOpt::Ok
        }
        b"--no-clobber" => {
            opts.overwrite = OverwriteMode::NoClobber;
            LongOpt::Ok
        }
        b"--update" => {
            opts.update = true;
            LongOpt::Ok
        }
        b"--verbose" => {
            opts.verbose = true;
            LongOpt::Ok
        }
        b"--no-target-directory" => {
            opts.no_target_dir = true;
            LongOpt::Ok
        }
        _ => LongOpt::Unknown,
    }
}

fn parse_short_options(flags: &[u8], opts: &mut Options) -> Result<(), u8> {
    for &b in flags {
        match b {
            b'f' => opts.overwrite = OverwriteMode::Force,
            b'i' => opts.overwrite = OverwriteMode::Interactive,
            b'n' => opts.overwrite = OverwriteMode::NoClobber,
            b'u' => opts.update = true,
            b'v' => opts.verbose = true,
            b'T' => opts.no_target_dir = true,
            _ => return Err(b),
        }
    }
    Ok(())
}

pub fn print_help() {
    print_out(t(
        "Использование: mv [ОПЦИЯ]... [-T] ИСТОЧНИК НАЗНАЧЕНИЕ\n  или:  mv [ОПЦИЯ]... ИСТОЧНИК... КАТАЛОГ\nПереименовывает ИСТОЧНИК в НАЗНАЧЕНИЕ или перемещает ИСТОЧНИК(И) в КАТАЛОГ.\n\nОпции:\n  -f, --force                  не спрашивать перед перезаписью\n  -i, --interactive            спрашивать перед перезаписью\n  -n, --no-clobber             не перезаписывать существующий файл\n  -T, --no-target-directory    считать НАЗНАЧЕНИЕ обычным файлом\n  -u, --update                 перемещать только если источник новее назначения\n  -v, --verbose                пояснять производимые действия\n      --help                   показать эту справку и выйти\n",
        "Usage: mv [OPTION]... [-T] SOURCE DEST\n  or:  mv [OPTION]... SOURCE... DIRECTORY\nRename SOURCE to DEST, or move SOURCE(s) to DIRECTORY.\n\nOptions:\n  -f, --force                  do not prompt before overwriting\n  -i, --interactive            prompt before overwrite\n  -n, --no-clobber             do not overwrite an existing file\n  -T, --no-target-directory    treat DEST as a normal file\n  -u, --update                 move only when the SOURCE file is newer than the destination\n  -v, --verbose                explain what is being done\n      --help                   display this help and exit\n",
    ));
}
