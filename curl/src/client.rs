use libwunix::{
    close, connect, current_language, eprintln, exit, open, read, socket, t, write, Language,
    SockAddrIn, AF_INET, SOCK_STREAM, STDOUT_FILENO,
};

use crate::url::{parse_content_length, ParsedProxy, ParsedUrl};

unsafe extern "C" {
    fn wunix_tls_init_session(
        hostname: *const u8,
        ca_file: *const u8,
        insecure: i32,
        verbose: i32,
    ) -> *mut core::ffi::c_void;
    fn wunix_tls_handshake(handle: *mut core::ffi::c_void, fd: i32, verbose: i32) -> i32;
    #[allow(dead_code)]
    fn wunix_tls_connect(
        fd: i32,
        hostname: *const u8,
        ca_file: *const u8,
        insecure: i32,
        verbose: i32,
    ) -> *mut core::ffi::c_void;
    fn wunix_tls_write(handle: *mut core::ffi::c_void, buf: *const u8, len: i32) -> i32;
    fn wunix_tls_read(handle: *mut core::ffi::c_void, buf: *mut u8, len: i32) -> i32;
    fn wunix_tls_close(handle: *mut core::ffi::c_void);
}

pub struct RequestConfig<'a> {
    pub parsed: &'a ParsedUrl<'a>,
    pub method: Option<&'a [u8]>,
    pub post_data: Option<&'a [u8]>,
    pub extra_headers: &'a [&'a [u8]],
    pub output_file: Option<&'a [u8]>,
    pub include_headers: bool,
    pub head_only: bool,
    pub silent: bool,
    pub show_error: bool,
    pub verbose: bool,
    pub insecure: bool,
    pub ca_file: Option<&'a [u8]>,
    pub proxy: Option<ParsedProxy<'a>>,
}

fn append_u16_digits(buf: &mut [u8], len: &mut usize, mut val: u16) {
    if val == 0 {
        if *len < buf.len() {
            buf[*len] = b'0';
            *len += 1;
        }
        return;
    }
    let mut digits = [0u8; 6];
    let mut idx = 0;
    while val > 0 {
        digits[idx] = (val % 10) as u8 + b'0';
        val /= 10;
        idx += 1;
    }
    while idx > 0 {
        idx -= 1;
        if *len < buf.len() {
            buf[*len] = digits[idx];
            *len += 1;
        }
    }
}

/// Устанавливает HTTP CONNECT туннель через прокси для защищенного TLS-трафика
fn establish_http_tunnel(fd: i32, host: &[u8], port: u16, verbose: bool) -> bool {
    let mut req = [0u8; 512];
    let mut len = 0;
    macro_rules! append {
        ($s:expr) => {
            let slice = $s;
            if len + slice.len() <= req.len() {
                req[len..len + slice.len()].copy_from_slice(slice);
                len += slice.len();
            }
        };
    }

    append!(b"CONNECT ");
    append!(host);
    append!(b":");
    append_u16_digits(&mut req, &mut len, port);
    append!(b" HTTP/1.1\r\nHost: ");
    append!(host);
    append!(b":");
    append_u16_digits(&mut req, &mut len, port);
    append!(b"\r\nUser-Agent: curl/8.5.0 (Wunix)\r\nProxy-Connection: Keep-Alive\r\n\r\n");

    if verbose {
        eprintln!(
            "* Establish HTTP proxy CONNECT tunnel to {}:{}...",
            core::str::from_utf8(host).unwrap_or(""),
            port
        );
    }

    let w = write(fd as u64, &req[..len]);
    if w <= 0 {
        return false;
    }

    let mut resp = [0u8; 1024];
    let mut resp_len = 0;
    while resp_len < resp.len() {
        let n = read(fd as u64, &mut resp[resp_len..]);
        if n <= 0 {
            return false;
        }
        resp_len += n as usize;
        if resp_len >= 4 && resp[..resp_len].windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }

    let status_bytes = &resp[..resp_len];
    let is_ok = status_bytes.windows(4).any(|w| w == b" 200" || w == b" 200 ");
    if is_ok {
        if verbose {
            eprintln!("* Proxy CONNECT tunnel established successfully");
        }
        true
    } else {
        if verbose {
            eprintln!(
                "* Proxy CONNECT tunnel rejected: {}",
                core::str::from_utf8(status_bytes).unwrap_or("")
            );
        }
        false
    }
}

pub fn execute_http_request(cfg: &RequestConfig) -> usize {
    let host_s = core::str::from_utf8(cfg.parsed.host).unwrap_or("");

    // Заранее инициализируем контекст OpenSSL TLS до установки соединения с сетью.
    // Это исключает задержку на чтение и парсинг корневых сертификатов CA из файловой системы
    // после открытия HTTP CONNECT туннеля, из-за которой прокси-сервер сбрасывает соединение по idle таймауту.
    let pre_tls: *mut core::ffi::c_void = if cfg.parsed.is_https {
        let mut host_null = [0u8; 256];
        let h_len = cfg.parsed.host.len().min(255);
        host_null[..h_len].copy_from_slice(&cfg.parsed.host[..h_len]);
        host_null[h_len] = 0;

        let mut ca_null = [0u8; 512];
        let ca_ptr = if let Some(ca) = cfg.ca_file {
            let ca_len = ca.len().min(511);
            ca_null[..ca_len].copy_from_slice(&ca[..ca_len]);
            ca_null[ca_len] = 0;
            ca_null.as_ptr()
        } else {
            core::ptr::null()
        };

        let handle = unsafe {
            wunix_tls_init_session(
                host_null.as_ptr(),
                ca_ptr,
                if cfg.insecure { 1 } else { 0 },
                if cfg.verbose { 1 } else { 0 },
            )
        };
        if handle.is_null() {
            if !cfg.silent || cfg.show_error {
                eprintln!(
                    "{}",
                    t(
                        "curl: (35) Не удалось инициализировать контекст OpenSSL TLS",
                        "curl: (35) Failed to initialize OpenSSL TLS context"
                    )
                );
            }
            exit(35);
        }
        handle
    } else {
        core::ptr::null_mut()
    };

    // Определяем адрес подключения: либо указанный прокси, либо целевой сервер напрямую
    let (connect_ip, connect_port, is_proxied) = if let Some(p) = cfg.proxy {
        let p_host_s = core::str::from_utf8(p.host).unwrap_or("");
        let p_ip = match libwunix::resolve_host(p_host_s) {
            Ok(ip) => ip,
            Err(_) => {
                if !cfg.silent || cfg.show_error {
                    eprintln!(
                        "{}: {}",
                        t("curl: (5) Не удалось разрешить адрес прокси", "curl: (5) Could not resolve proxy"),
                        p_host_s
                    );
                }
                exit(5);
            }
        };
        (p_ip, p.port, true)
    } else {
        let host_ip = match libwunix::resolve_host(host_s) {
            Ok(ip) => ip,
            Err(_) => {
                if !cfg.silent || cfg.show_error {
                    eprintln!(
                        "{}: {}",
                        t(
                            "curl: (6) Не удалось разрешить имя хоста",
                            "curl: (6) Could not resolve host"
                        ),
                        host_s
                    );
                }
                exit(6);
            }
        };
        (host_ip, cfg.parsed.port, false)
    };

    if cfg.verbose {
        if is_proxied {
            eprintln!(
                "*   Trying proxy {}.{}.{}.{}:{}...",
                connect_ip[0], connect_ip[1], connect_ip[2], connect_ip[3], connect_port
            );
        } else {
            eprintln!("*   Trying {}:{}...", host_s, connect_port);
        }
    }

    let fd = socket(AF_INET, SOCK_STREAM, 0);
    if fd < 0 {
        if !cfg.silent || cfg.show_error {
            eprintln!("{}: {}", t("curl: ошибка сокета", "curl: socket error"), fd);
        }
        exit(7);
    }

    let sockaddr = SockAddrIn::new_v4(connect_ip, connect_port);
    let conn_res = connect(fd as u64, &sockaddr);
    if conn_res < 0 {
        if !cfg.silent || cfg.show_error {
            eprintln!(
                "{}: {} {}:{}",
                t(
                    "curl: (7) Не удалось подключиться к хосту",
                    "curl: (7) Failed to connect to"
                ),
                if is_proxied { "proxy" } else { host_s },
                connect_port,
                conn_res
            );
        }
        close(fd as u64);
        exit(7);
    }

    if cfg.verbose {
        eprintln!(
            "* Connected to {} port {}",
            if is_proxied { "proxy" } else { host_s },
            connect_port
        );
    }

    // Если соединение через прокси и целевой протокол HTTPS — поднимаем HTTP CONNECT туннель
    if is_proxied && cfg.parsed.is_https {
        if !establish_http_tunnel(fd as i32, cfg.parsed.host, cfg.parsed.port, cfg.verbose) {
            if !cfg.silent || cfg.show_error {
                eprintln!(
                    "{}",
                    t(
                        "curl: (56) Не удалось установить HTTP CONNECT туннель через прокси",
                        "curl: (56) Received HTTP code 5xx/4xx from proxy during CONNECT"
                    )
                );
            }
            close(fd as u64);
            exit(56);
        }
    }

    // Выполняем OpenSSL TLS рукопожатие на подготовленном контексте
    let tls_handle: *mut core::ffi::c_void = if cfg.parsed.is_https {
        if cfg.verbose {
            eprintln!("* Initializing OpenSSL TLS handshake with {}...", host_s);
        }

        let hs_ok = unsafe {
            wunix_tls_handshake(
                pre_tls,
                fd as i32,
                if cfg.verbose { 1 } else { 0 },
            )
        };

        if hs_ok <= 0 {
            unsafe { wunix_tls_close(pre_tls) };
            if !cfg.silent || cfg.show_error {
                eprintln!(
                    "{}",
                    t(
                        "curl: (35) Ошибка TLS/SSL рукопожатия или верификации сертификата OpenSSL",
                        "curl: (35) OpenSSL TLS/SSL handshake or certificate verification failed"
                    )
                );
            }
            close(fd as u64);
            exit(35);
        }

        if cfg.verbose {
            eprintln!("* OpenSSL TLS handshake completed successfully");
        }
        pre_tls
    } else {
        core::ptr::null_mut()
    };

    // Собираем валидный HTTP/1.0 запрос
    let mut req_buf = [0u8; 2048];
    let mut req_len = 0;

    macro_rules! append_bytes {
        ($b:expr) => {
            let slice = $b;
            if req_len + slice.len() <= req_buf.len() {
                req_buf[req_len..req_len + slice.len()].copy_from_slice(slice);
                req_len += slice.len();
            }
        };
    }

    let eff_method = cfg.method.unwrap_or(b"GET");
    append_bytes!(eff_method);
    append_bytes!(b" ");

    if is_proxied && !cfg.parsed.is_https {
        append_bytes!(b"http://");
        append_bytes!(cfg.parsed.host);
        if cfg.parsed.port != 80 {
            append_bytes!(b":");
            append_u16_digits(&mut req_buf, &mut req_len, cfg.parsed.port);
        }
        append_bytes!(cfg.parsed.path);
    } else {
        append_bytes!(cfg.parsed.path);
    }

    append_bytes!(b" HTTP/1.0\r\nHost: ");
    append_bytes!(cfg.parsed.host);
    append_bytes!(b"\r\nUser-Agent: curl/8.5.0 (Wunix; OpenSSL)\r\nAccept: */*\r\nConnection: close\r\n");

    for h in cfg.extra_headers {
        if !h.is_empty() {
            append_bytes!(h);
            append_bytes!(b"\r\n");
        }
    }

    if let Some(body) = cfg.post_data {
        append_bytes!(b"Content-Type: application/x-www-form-urlencoded\r\nContent-Length: ");
        let mut len_digits = [0u8; 10];
        let mut d_idx = 0;
        let mut l = body.len();
        if l == 0 {
            append_bytes!(b"0");
        } else {
            while l > 0 {
                len_digits[d_idx] = (l % 10) as u8 + b'0';
                l /= 10;
                d_idx += 1;
            }
            while d_idx > 0 {
                d_idx -= 1;
                append_bytes!(&[len_digits[d_idx]]);
            }
        }
        append_bytes!(b"\r\n\r\n");
        append_bytes!(body);
    } else {
        append_bytes!(b"\r\n");
    }

    if cfg.verbose {
        if let Ok(req_str) = core::str::from_utf8(&req_buf[..req_len]) {
            for line in req_str.lines() {
                eprintln!("> {}", line);
            }
            eprintln!(">");
        }
    }

    if !tls_handle.is_null() {
        let _ = unsafe { wunix_tls_write(tls_handle, req_buf.as_ptr(), req_len as i32) };
    } else {
        let _ = write(fd as u64, &req_buf[..req_len]);
    }

    let out_fd: u64 = if let Some(path) = cfg.output_file {
        if path == b"-" {
            STDOUT_FILENO
        } else {
            let mut null_term = [0u8; 1024];
            let copy_len = path.len().min(1023);
            null_term[..copy_len].copy_from_slice(&path[..copy_len]);
            null_term[copy_len] = 0;
            // O_CREAT (0x40) | O_TRUNC (0x200) | O_WRONLY (0x01)
            let opened = open(&null_term[..copy_len + 1], 0x40 | 0x200 | 0x01, 0o644);
            if opened < 0 {
                if !cfg.silent || cfg.show_error {
                    eprintln!(
                        "{}: {}",
                        t("curl: ошибка открытия файла для записи", "curl: cannot open output file"),
                        core::str::from_utf8(path).unwrap_or("")
                    );
                }
                if !tls_handle.is_null() {
                    unsafe { wunix_tls_close(tls_handle) };
                }
                close(fd as u64);
                exit(1);
            }
            opened as u64
        }
    } else {
        STDOUT_FILENO
    };

    let mut resp_buf = [0u8; 4096];
    let mut header_skipped = false;
    let mut accumulated = [0u8; 4096];
    let mut acc_len = 0;
    let mut total_bytes_written: usize = 0;
    let mut body_bytes_written: usize = 0;
    let mut expected_body_len: Option<usize> = None;
    let mut verbose_headers_printed = false;

    loop {
        let n = if !tls_handle.is_null() {
            let res = unsafe {
                wunix_tls_read(tls_handle, resp_buf.as_mut_ptr(), resp_buf.len() as i32)
            };
            if res <= 0 {
                0
            } else {
                res as usize
            }
        } else {
            let res = read(fd as u64, &mut resp_buf);
            if res <= 0 {
                0
            } else {
                res as usize
            }
        };

        if n == 0 {
            break;
        }

        let chunk = &resp_buf[..n];

        if cfg.head_only {
            if acc_len + chunk.len() <= accumulated.len() {
                accumulated[acc_len..acc_len + chunk.len()].copy_from_slice(chunk);
                acc_len += chunk.len();
            }
            let _ = write(out_fd, chunk);
            total_bytes_written += chunk.len();

            if accumulated[..acc_len].windows(4).any(|w| w == b"\r\n\r\n") {
                if cfg.verbose && !verbose_headers_printed {
                    if let Ok(hdr_str) = core::str::from_utf8(&accumulated[..acc_len]) {
                        for line in hdr_str.lines() {
                            eprintln!("< {}", line);
                        }
                        eprintln!("<");
                    }
                }
                break;
            }
        } else if cfg.include_headers {
            if !header_skipped {
                if acc_len + chunk.len() <= accumulated.len() {
                    accumulated[acc_len..acc_len + chunk.len()].copy_from_slice(chunk);
                    acc_len += chunk.len();
                }
                if let Some(pos) = accumulated[..acc_len].windows(4).position(|w| w == b"\r\n\r\n") {
                    expected_body_len = parse_content_length(&accumulated[..pos]);
                    header_skipped = true;
                    let body_start = pos + 4;
                    if body_start < acc_len {
                        body_bytes_written += acc_len - body_start;
                    }
                }
            } else {
                body_bytes_written += chunk.len();
            }

            let _ = write(out_fd, chunk);
            total_bytes_written += chunk.len();

            if let Some(exp) = expected_body_len {
                if body_bytes_written >= exp {
                    break;
                }
            }
        } else {
            if !header_skipped {
                if acc_len + chunk.len() <= accumulated.len() {
                    accumulated[acc_len..acc_len + chunk.len()].copy_from_slice(chunk);
                    acc_len += chunk.len();
                }
                if let Some(pos) = accumulated[..acc_len].windows(4).position(|w| w == b"\r\n\r\n") {
                    header_skipped = true;
                    expected_body_len = parse_content_length(&accumulated[..pos]);

                    if cfg.verbose && !verbose_headers_printed {
                        if let Ok(hdr_str) = core::str::from_utf8(&accumulated[..pos]) {
                            for line in hdr_str.lines() {
                                eprintln!("< {}", line);
                            }
                            eprintln!("<");
                        }
                        verbose_headers_printed = true;
                    }

                    let body_start = pos + 4;
                    if body_start < acc_len {
                        let initial_body = &accumulated[body_start..acc_len];
                        let _ = write(out_fd, initial_body);
                        total_bytes_written += initial_body.len();
                        body_bytes_written += initial_body.len();
                    }

                    if let Some(exp) = expected_body_len {
                        if body_bytes_written >= exp {
                            break;
                        }
                    }
                }
            } else {
                let _ = write(out_fd, chunk);
                total_bytes_written += chunk.len();
                body_bytes_written += chunk.len();

                if let Some(exp) = expected_body_len {
                    if body_bytes_written >= exp {
                        break;
                    }
                }
            }
        }
    }

    if !tls_handle.is_null() {
        unsafe { wunix_tls_close(tls_handle) };
    }
    close(fd as u64);
    if out_fd != STDOUT_FILENO {
        close(out_fd);
    }

    if cfg.verbose {
        eprintln!("* Closing connection 0");
    }

    if let Some(path) = cfg.output_file {
        if path != b"-" && !cfg.silent {
            let path_str = core::str::from_utf8(path).unwrap_or("");
            let is_ru = current_language() == Language::Russian;
            if is_ru {
                eprintln!("curl: сохранено {} байт в '{}'", total_bytes_written, path_str);
            } else {
                eprintln!("curl: saved {} bytes to '{}'", total_bytes_written, path_str);
            }
        }
    }

    total_bytes_written
}
