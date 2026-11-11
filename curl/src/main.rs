#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(test, allow(dead_code, unused))]

mod client;
mod url;

use client::{execute_http_request, RequestConfig};
use libwunix::{eprintln, exit, println, t, STDERR_FILENO};
use url::{extract_remote_filename, parse_output_destination, parse_proxy, parse_url};

fn cstr_to_slice(ptr: *const u8) -> &'static [u8] {
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

unsafe extern "C" {
    fn wunix_init_tls(argc: isize, argv: *const *const u8);
}

#[cfg(not(test))]
core::arch::global_asm!(
    r#"
    .global _start
    _start:
        xor rbp, rbp
        and rsp, -16
        call main_entry
        ud2
    "#
);

#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn main_entry(argc: isize, argv: *const *const u8) -> ! {
    unsafe { wunix_init_tls(argc, argv) };
    let mut target_url: Option<&'static [u8]> = None;
    let mut output_file: Option<&'static [u8]> = None;
    let mut remote_name = false;
    let mut include_headers = false;
    let mut head_only = false;
    let mut silent = false;
    let mut show_error = false;
    let mut verbose = false;
    let mut insecure = false;
    let mut ca_file: Option<&'static [u8]> = None;
    let mut method: Option<&'static [u8]> = None;
    let mut post_data: Option<&'static [u8]> = None;
    let mut proxy_arg: Option<&'static [u8]> = None;
    let mut extra_headers: [&'static [u8]; 8] = [&b""[..]; 8];
    let mut extra_header_count = 0;

    let mut i = 1;
    while i < argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование: curl [опции...] <url>\n\
                     Опции:\n  \
                       -o, --output <файл>     Сохранить ответ в файл вместо stdout\n  \
                       -O, --remote-name       Сохранить файл под именем из URL\n  \
                       -s, --silent            Не выводить индикаторы и сообщения об ошибках\n  \
                       -S, --show-error        Вместе с -s выводить ошибки при сбоях\n  \
                       -i, --include           Выводить HTTP-заголовки ответа\n  \
                       -I, --head              Запрашивать только HTTP-заголовки (метод HEAD)\n  \
                       -k, --insecure          Разрешить небезопасные SSL/TLS соединения\n  \
                       --cacert <файл>         Указать путь к бандлу CA-сертификатов\n  \
                       -x, --proxy <прокси>    Использовать HTTP-прокси ([http://]host:port)\n  \
                       -v, --verbose           Подробный лог подключения и TLS-рукопожатия\n  \
                       -X, --request <метод>   Метод HTTP (GET, POST, HEAD, PUT, DELETE)\n  \
                       -d, --data <данные>     Данные для HTTP POST запроса\n  \
                       -H, --header <заголовок> Дополнительный HTTP-заголовок\n  \
                       -h, --help              Показать эту справку и выйти\n  \
                       -V, --version           Вывести информацию о версии и выйти",
                    "Usage: curl [options...] <url>\n\
                     Options:\n  \
                       -o, --output <file>     Write to file instead of stdout\n  \
                       -O, --remote-name       Write output to a local file named like the remote file\n  \
                       -s, --silent            Silent mode (don't output anything)\n  \
                       -S, --show-error        When used with -s, make curl show error when it fails\n  \
                       -i, --include           Include HTTP response headers in the output\n  \
                       -I, --head              Show document info only (HEAD request)\n  \
                       -k, --insecure          Allow insecure server connections when using SSL\n  \
                       --cacert <file>         CA certificate to verify peer against\n  \
                       -x, --proxy <proxy>     Use HTTP proxy ([http://]host:port)\n  \
                       -v, --verbose           Make the operation more talkative\n  \
                       -X, --request <method>  HTTP method (GET, POST, HEAD, PUT, DELETE)\n  \
                       -d, --data <data>       HTTP POST data\n  \
                       -H, --header <header>   Extra header to include in request\n  \
                       -h, --help              This help text and exit\n  \
                       -V, --version           Display version and exit"
                )
            );
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("curl 1.0.0 (x86_64-wunix) OpenSSL/3.4.0");
            exit(0);
        } else if arg == b"-k" || arg == b"--insecure" {
            insecure = true;
        } else if arg == b"-x" || arg == b"--proxy" {
            i += 1;
            if i < argc {
                proxy_arg = Some(cstr_to_slice(unsafe { *argv.add(i as usize) }));
            }
        } else if arg.starts_with(b"-x") && arg.len() > 2 {
            proxy_arg = Some(&arg[2..]);
        } else if arg == b"--cacert" {
            i += 1;
            if i < argc {
                ca_file = Some(cstr_to_slice(unsafe { *argv.add(i as usize) }));
            }
        } else if arg == b"-O" || arg == b"--remote-name" {
            remote_name = true;
        } else if arg == b"-o" || arg == b"--output" {
            i += 1;
            if i < argc {
                output_file = parse_output_destination(cstr_to_slice(unsafe { *argv.add(i as usize) }));
            }
        } else if arg.starts_with(b"-o") && arg.len() > 2 {
            output_file = parse_output_destination(&arg[2..]);
        } else if arg == b"-s" || arg == b"--silent" {
            silent = true;
        } else if arg == b"-S" || arg == b"--show-error" {
            show_error = true;
        } else if arg == b"-i" || arg == b"--include" {
            include_headers = true;
        } else if arg == b"-I" || arg == b"--head" {
            head_only = true;
            include_headers = true;
            if method.is_none() {
                method = Some(b"HEAD");
            }
        } else if arg == b"-v" || arg == b"--verbose" {
            verbose = true;
        } else if arg == b"-X" || arg == b"--request" {
            i += 1;
            if i < argc {
                method = Some(cstr_to_slice(unsafe { *argv.add(i as usize) }));
            }
        } else if arg.starts_with(b"-X") && arg.len() > 2 {
            method = Some(&arg[2..]);
        } else if arg == b"-d" || arg == b"--data" {
            i += 1;
            if i < argc {
                post_data = Some(cstr_to_slice(unsafe { *argv.add(i as usize) }));
                if method.is_none() {
                    method = Some(b"POST");
                }
            }
        } else if arg.starts_with(b"-d") && arg.len() > 2 {
            post_data = Some(&arg[2..]);
            if method.is_none() {
                method = Some(b"POST");
            }
        } else if arg == b"-H" || arg == b"--header" {
            i += 1;
            if i < argc && extra_header_count < 8 {
                extra_headers[extra_header_count] = cstr_to_slice(unsafe { *argv.add(i as usize) });
                extra_header_count += 1;
            }
        } else if arg.starts_with(b"-") && !arg.starts_with(b"--") {
            let mut j = 1;
            while j < arg.len() {
                match arg[j] {
                    b'k' => insecure = true,
                    b's' => silent = true,
                    b'S' => show_error = true,
                    b'i' => include_headers = true,
                    b'I' => {
                        head_only = true;
                        include_headers = true;
                        if method.is_none() {
                            method = Some(b"HEAD");
                        }
                    }
                    b'v' => verbose = true,
                    b'O' => remote_name = true,
                    b'o' => {
                        if j + 1 < arg.len() {
                            output_file = parse_output_destination(&arg[j + 1..]);
                            break;
                        } else {
                            i += 1;
                            if i < argc {
                                output_file = parse_output_destination(cstr_to_slice(unsafe { *argv.add(i as usize) }));
                            }
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
        } else if !arg.starts_with(b"-") && target_url.is_none() {
            target_url = Some(arg);
        }
        i += 1;
    }

    let url_raw = match target_url {
        Some(u) => u,
        None => {
            if !silent || show_error {
                eprintln!(
                    "{}",
                    t(
                        "curl: попробуйте 'curl --help' для получения справки\ncurl: URL не указан!",
                        "curl: try 'curl --help' for more information\ncurl: no URL specified!"
                    )
                );
            }
            exit(2);
        }
    };

    let parsed = match parse_url(url_raw) {
        Some(p) => p,
        None => {
            if !silent || show_error {
                eprintln!(
                    "{}",
                    t("curl: не удалось разобрать URL", "curl: URL malformed")
                );
            }
            exit(3);
        }
    };

    if remote_name && output_file.is_none() {
        output_file = Some(extract_remote_filename(parsed.path));
    }

    let envp = unsafe { argv.add(argc as usize + 1) };
    if !envp.is_null() && unsafe { !(*envp).is_null() } {
        libwunix::env::init(envp);
    }

    let proxy_raw = proxy_arg.or_else(|| {
        if parsed.is_https {
            libwunix::env::getenv("https_proxy")
                .or_else(|| libwunix::env::getenv("HTTPS_PROXY"))
                .or_else(|| libwunix::env::getenv("all_proxy"))
                .or_else(|| libwunix::env::getenv("ALL_PROXY"))
                .map(|s| s.as_bytes())
        } else {
            libwunix::env::getenv("http_proxy")
                .or_else(|| libwunix::env::getenv("HTTP_PROXY"))
                .or_else(|| libwunix::env::getenv("all_proxy"))
                .or_else(|| libwunix::env::getenv("ALL_PROXY"))
                .map(|s| s.as_bytes())
        }
    });

    let proxy = proxy_raw.and_then(parse_proxy);

    let cfg = RequestConfig {
        parsed: &parsed,
        method,
        post_data,
        extra_headers: &extra_headers[..extra_header_count],
        output_file,
        include_headers,
        head_only,
        silent,
        show_error,
        verbose,
        insecure,
        ca_file,
        proxy,
    };

    execute_http_request(&cfg);
    exit(0);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    libwunix::write(STDERR_FILENO, b"curl: panic occurred\n");
    exit(1);
}

#[cfg(test)]
fn main() {}
