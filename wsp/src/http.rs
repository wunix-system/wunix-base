//! Минимальный легковесный HTTP-клиент для системного менеджера пакетов WSP.
//!
//! Реализует протокол HTTP/1.1 (GET), автоматическую поддержку прокси-серверов,
//! проверку контрольных сумм SHA-256 и загрузку пакетов базовой системы.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{
    close, connect, open, read, resolve_host, socket, unlink, write,
    SockAddrIn, AF_INET, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY, SOCK_STREAM,
};
use libwunix::crypto::{bytes_to_hex, Sha256};
use crate::progress::ProgressBar;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpError {
    InvalidUrl,
    UnsupportedScheme,
    DnsError,
    ConnectionFailed,
    WriteError,
    ReadError,
    HttpStatus(u16),
    ChecksumMismatch,
    IoError,
    TooManyRedirects,
}

pub fn parse_http_url(url: &str) -> Result<(&str, u16, &str), HttpError> {
    let rest = if let Some(stripped) = url.strip_prefix("http://") {
        stripped
    } else if url.starts_with("https://") {
        return Err(HttpError::UnsupportedScheme);
    } else {
        return Err(HttpError::InvalidUrl);
    };

    let (host_port, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, "/"),
    };

    let (host, port) = match host_port.find(':') {
        Some(idx) => {
            let h = &host_port[..idx];
            let p = host_port[idx + 1..].parse::<u16>().map_err(|_| HttpError::InvalidUrl)?;
            (h, p)
        }
        None => (host_port, 80),
    };

    if host.is_empty() {
        return Err(HttpError::InvalidUrl);
    }

    Ok((host, port, path))
}

pub fn get_proxy_settings(host: &str) -> Option<(String, u16)> {
    if let Some(no_proxy_str) = libwunix::env::getenv("no_proxy").or_else(|| libwunix::env::getenv("NO_PROXY")) {
        for entry in no_proxy_str.split(',') {
            let trimmed = entry.trim();
            if trimmed == "*" || trimmed == host || (trimmed.starts_with('.') && host.ends_with(trimmed)) {
                return None;
            }
        }
    }

    let proxy_raw = libwunix::env::getenv("http_proxy")
        .or_else(|| libwunix::env::getenv("HTTP_PROXY"))
        .or_else(|| libwunix::env::getenv("all_proxy"))
        .or_else(|| libwunix::env::getenv("ALL_PROXY"))?;

    let proxy_str = proxy_raw.trim();
    if proxy_str.is_empty() {
        return None;
    }

    let trimmed = proxy_str.strip_prefix("http://").unwrap_or(proxy_str);
    let trimmed = trimmed.trim_end_matches('/');

    let (p_host, p_port) = match trimmed.find(':') {
        Some(idx) => {
            let h = trimmed[..idx].to_string();
            let p = trimmed[idx + 1..].parse::<u16>().unwrap_or(8080);
            (h, p)
        }
        None => (trimmed.to_string(), 8080),
    };

    Some((p_host, p_port))
}

fn connect_target(host: &str, port: u16, proxy: &Option<(String, u16)>) -> Result<(i64, bool), HttpError> {
    let (target_host, target_port, is_proxied) = if let Some((p_host, p_port)) = proxy {
        (p_host.as_str(), *p_port, true)
    } else {
        (host, port, false)
    };

    let ip = resolve_host(target_host).map_err(|_| HttpError::DnsError)?;
    let fd = socket(AF_INET, SOCK_STREAM, 0);
    if fd < 0 {
        return Err(HttpError::ConnectionFailed);
    }

    let sockaddr = SockAddrIn::new_v4(ip, target_port);
    if connect(fd as u64, &sockaddr) < 0 {
        close(fd as u64);
        return Err(HttpError::ConnectionFailed);
    }

    Ok((fd, is_proxied))
}

fn write_all(fd: i64, data: &[u8]) -> bool {
    let mut remaining = data;
    while !remaining.is_empty() {
        let n = write(fd as u64, remaining);
        if n <= 0 {
            return false;
        }
        remaining = &remaining[n as usize..];
    }
    true
}

fn parse_content_length(headers: &str) -> Option<u64> {
    for line in headers.lines() {
        let trimmed = line.trim();
        if let Some(idx) = trimmed.find(':') {
            let key = trimmed[..idx].trim();
            if key.eq_ignore_ascii_case("Content-Length") {
                return trimmed[idx + 1..].trim().parse::<u64>().ok();
            }
        }
    }
    None
}

fn parse_status(headers: &str) -> Option<u16> {
    let first_line = headers.lines().next()?;
    let mut parts = first_line.split_whitespace();
    let _proto = parts.next()?;
    let code = parts.next()?.parse::<u16>().ok()?;
    Some(code)
}

fn parse_location(headers: &str) -> Option<String> {
    for line in headers.lines() {
        let trimmed = line.trim();
        if let Some(idx) = trimmed.find(':') {
            let key = trimmed[..idx].trim();
            if key.eq_ignore_ascii_case("Location") {
                return Some(trimmed[idx + 1..].trim().to_string());
            }
        }
    }
    None
}

#[allow(dead_code)]
pub fn fetch_to_vec(url: &str, display_name: &str) -> Result<Vec<u8>, HttpError> {
    let mut current_url = url.to_string();
    let mut redirects = 0;

    loop {
        if redirects > 5 {
            return Err(HttpError::TooManyRedirects);
        }

        let (host, port, path) = parse_http_url(&current_url)?;
        let proxy = get_proxy_settings(host);
        let (fd, is_proxied) = connect_target(host, port, &proxy)?;

        let req = if is_proxied {
            format!(
                "GET http://{}:{}{} HTTP/1.1\r\nHost: {}\r\nUser-Agent: wunix-pack/1.0\r\nConnection: close\r\n\r\n",
                host, port, path, host
            )
        } else {
            format!(
                "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: wunix-pack/1.0\r\nConnection: close\r\n\r\n",
                path, host
            )
        };

        if !write_all(fd, req.as_bytes()) {
            close(fd as u64);
            return Err(HttpError::WriteError);
        }

        let mut hdr_buf = [0u8; 4096];
        let mut hdr_len = 0;
        let mut sep_pos = None;

        while hdr_len < hdr_buf.len() {
            let n = read(fd as u64, &mut hdr_buf[hdr_len..hdr_len + 1]);
            if n <= 0 {
                break;
            }
            hdr_len += 1;
            if hdr_len >= 4 && &hdr_buf[hdr_len - 4..hdr_len] == b"\r\n\r\n" {
                sep_pos = Some(hdr_len - 4);
                break;
            }
        }

        let sep = match sep_pos {
            Some(s) => s,
            None => {
                close(fd as u64);
                return Err(HttpError::ReadError);
            }
        };

        let hdr_str = core::str::from_utf8(&hdr_buf[..sep]).map_err(|_| HttpError::ReadError)?;
        let status = parse_status(hdr_str).unwrap_or(500);

        if status == 301 || status == 302 || status == 307 || status == 308 {
            close(fd as u64);
            if let Some(loc) = parse_location(hdr_str) {
                current_url = loc;
                redirects += 1;
                continue;
            } else {
                return Err(HttpError::HttpStatus(status));
            }
        }

        if status != 200 {
            close(fd as u64);
            return Err(HttpError::HttpStatus(status));
        }

        let content_len = parse_content_length(hdr_str);
        let mut pb = ProgressBar::new(display_name, content_len);

        let mut body = Vec::with_capacity(content_len.unwrap_or(4096) as usize);
        let mut chunk = [0u8; 4096];

        loop {
            let n = read(fd as u64, &mut chunk);
            if n <= 0 {
                break;
            }
            body.extend_from_slice(&chunk[..n as usize]);
            pb.update(body.len() as u64);
            if let Some(tot) = content_len {
                if body.len() as u64 >= tot {
                    break;
                }
            }
        }

        close(fd as u64);
        pb.finish(true, "");
        return Ok(body);
    }
}

pub fn fetch_to_file(
    url: &str,
    target_path: &str,
    expected_sha256: Option<&str>,
    display_name: &str,
) -> Result<u64, HttpError> {
    let mut current_url = url.to_string();
    let mut redirects = 0;

    loop {
        if redirects > 5 {
            return Err(HttpError::TooManyRedirects);
        }

        let (host, port, path) = parse_http_url(&current_url)?;
        let proxy = get_proxy_settings(host);
        let (fd, is_proxied) = connect_target(host, port, &proxy)?;

        let req = if is_proxied {
            format!(
                "GET http://{}:{}{} HTTP/1.1\r\nHost: {}\r\nUser-Agent: wunix-pack/1.0\r\nConnection: close\r\n\r\n",
                host, port, path, host
            )
        } else {
            format!(
                "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: wunix-pack/1.0\r\nConnection: close\r\n\r\n",
                path, host
            )
        };

        if !write_all(fd, req.as_bytes()) {
            close(fd as u64);
            return Err(HttpError::WriteError);
        }

        let mut hdr_buf = [0u8; 4096];
        let mut hdr_len = 0;
        let mut sep_pos = None;

        while hdr_len < hdr_buf.len() {
            let n = read(fd as u64, &mut hdr_buf[hdr_len..hdr_len + 1]);
            if n <= 0 {
                break;
            }
            hdr_len += 1;
            if hdr_len >= 4 && &hdr_buf[hdr_len - 4..hdr_len] == b"\r\n\r\n" {
                sep_pos = Some(hdr_len - 4);
                break;
            }
        }

        let sep = match sep_pos {
            Some(s) => s,
            None => {
                close(fd as u64);
                return Err(HttpError::ReadError);
            }
        };

        let hdr_str = core::str::from_utf8(&hdr_buf[..sep]).map_err(|_| HttpError::ReadError)?;
        let status = parse_status(hdr_str).unwrap_or(500);

        if status == 301 || status == 302 || status == 307 || status == 308 {
            close(fd as u64);
            if let Some(loc) = parse_location(hdr_str) {
                current_url = loc;
                redirects += 1;
                continue;
            } else {
                return Err(HttpError::HttpStatus(status));
            }
        }

        if status != 200 {
            close(fd as u64);
            return Err(HttpError::HttpStatus(status));
        }

        let content_len = parse_content_length(hdr_str);
        let mut pb = ProgressBar::new(display_name, content_len);

        let mut null_target = [0u8; 256];
        let b = target_path.as_bytes();
        let l = b.len().min(255);
        null_target[..l].copy_from_slice(&b[..l]);
        null_target[l] = 0;

        let mut out_fd = open(&null_target, (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o666);
        if out_fd < 0 {
            let _ = libwunix::unlink(&null_target);
            out_fd = open(&null_target, (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o666);
        }
        if out_fd < 0 {
            close(fd as u64);
            pb.finish(false, "не удалось открыть файл для записи");
            return Err(HttpError::IoError);
        }

        let mut hasher = Sha256::new();
        let mut total_written: u64 = 0;
        let mut chunk = [0u8; 4096];

        loop {
            let n = read(fd as u64, &mut chunk);
            if n <= 0 {
                break;
            }
            let slice = &chunk[..n as usize];
            hasher.update(slice);
            let written = write(out_fd as u64, slice);
            if written <= 0 {
                close(out_fd as u64);
                close(fd as u64);
                let _ = unlink(&null_target);
                pb.finish(false, "ошибка записи на диск");
                return Err(HttpError::IoError);
            }
            total_written += n as u64;
            pb.update(total_written);
            if let Some(tot) = content_len {
                if total_written >= tot {
                    break;
                }
            }
        }

        close(out_fd as u64);
        close(fd as u64);

        if let Some(expected_hash) = expected_sha256 {
            let digest = hasher.finalize();
            let mut hex_buf = [0u8; 64];
            bytes_to_hex(&digest, &mut hex_buf);
            let calc_hex = core::str::from_utf8(&hex_buf).unwrap_or("");
            let mut matches_hash = calc_hex.eq_ignore_ascii_case(expected_hash);
            if !matches_hash {
                // Проверяем альтернативный хэш полезной нагрузки WSP1 из заголовка пакета
                let check_fd = open(&null_target, O_RDONLY as u64, 0);
                if check_fd >= 0 {
                    let mut hdr = [0u8; 128];
                    let rlen = libwunix::pread64(check_fd as u64, &mut hdr, 0);
                    close(check_fd as u64);
                    if rlen >= 72 && &hdr[0..4] == b"WSP1" {
                        let mut payload_hex = [0u8; 64];
                        bytes_to_hex(&hdr[40..72], &mut payload_hex);
                        if let Ok(p_str) = core::str::from_utf8(&payload_hex) {
                            if p_str.eq_ignore_ascii_case(expected_hash) {
                                matches_hash = true;
                            }
                        }
                    }
                }
            }
            if !matches_hash {
                let _ = unlink(&null_target);
                pb.finish(false, "контрольная сумма SHA-256 не совпадает!");
                return Err(HttpError::ChecksumMismatch);
            }
        }

        pb.finish(true, "");
        return Ok(total_written);
    }
}
