pub fn parse_u16(s: &[u8]) -> Option<u16> {
    let mut val = 0u32;
    if s.is_empty() {
        return None;
    }
    for &b in s {
        if (b'0'..=b'9').contains(&b) {
            val = val * 10 + (b - b'0') as u32;
            if val > 65535 {
                return None;
            }
        } else {
            return None;
        }
    }
    Some(val as u16)
}

pub struct ParsedUrl<'a> {
    pub host: &'a [u8],
    pub port: u16,
    pub path: &'a [u8],
    pub is_https: bool,
}

pub fn parse_url(url: &[u8]) -> Option<ParsedUrl<'_>> {
    let (rest, is_https) = if url.starts_with(b"http://") {
        (&url[7..], false)
    } else if url.starts_with(b"https://") {
        (&url[8..], true)
    } else {
        (url, false)
    };

    let (host_port, path) = match rest.iter().position(|&b| b == b'/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, &b"/"[..]),
    };

    let (host, port) = match host_port.iter().position(|&b| b == b':') {
        Some(idx) => {
            let h = &host_port[..idx];
            let p = parse_u16(&host_port[idx + 1..])?;
            (h, p)
        }
        None => (host_port, if is_https { 443 } else { 80 }),
    };

    Some(ParsedUrl {
        host,
        port,
        path,
        is_https,
    })
}

pub fn extract_remote_filename(path: &[u8]) -> &[u8] {
    let clean = match path.iter().position(|&b| b == b'?') {
        Some(idx) => &path[..idx],
        None => path,
    };
    let name = match clean.iter().rposition(|&b| b == b'/') {
        Some(idx) => &clean[idx + 1..],
        None => clean,
    };
    if name.is_empty() {
        b"index.html"
    } else {
        name
    }
}

/// Распознает назначение вывода для флага -o: дефис '-' означает stdout (None)
pub fn parse_output_destination(dest: &[u8]) -> Option<&[u8]> {
    if dest == b"-" {
        None
    } else {
        Some(dest)
    }
}

/// Извлекает числовое значение заголовка Content-Length из HTTP-ответа
pub fn parse_content_length(headers: &[u8]) -> Option<usize> {
    for line in headers.split(|&b| b == b'\n') {
        let trimmed = if line.ends_with(b"\r") {
            &line[..line.len() - 1]
        } else {
            line
        };

        if trimmed.len() > 15 && trimmed[..15].eq_ignore_ascii_case(b"content-length:") {
            let mut val_slice = &trimmed[15..];
            while !val_slice.is_empty() && (val_slice[0] == b' ' || val_slice[0] == b'\t') {
                val_slice = &val_slice[1..];
            }
            let mut val: usize = 0;
            let mut has_digits = false;
            for &b in val_slice {
                if (b'0'..=b'9').contains(&b) {
                    val = val.saturating_mul(10).saturating_add((b - b'0') as usize);
                    has_digits = true;
                } else if b == b' ' || b == b'\t' {
                    break;
                } else {
                    return None;
                }
            }
            if has_digits {
                return Some(val);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_content_length() {
        let hdr1 = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 123\r\n\r\n";
        assert_eq!(parse_content_length(hdr1), Some(123));

        let hdr2 = b"HTTP/1.0 200 OK\r\ncontent-length: 0\r\nConnection: close\r\n";
        assert_eq!(parse_content_length(hdr2), Some(0));

        let hdr3 = b"HTTP/1.1 200 OK\r\nCoNtEnT-LeNgTh:   4567 \r\n";
        assert_eq!(parse_content_length(hdr3), Some(4567));

        let hdr4 = b"HTTP/1.1 200 OK\r\nServer: nginx\r\n\r\n";
        assert_eq!(parse_content_length(hdr4), None);
    }

    #[test]
    fn test_parse_url_schemes_and_ports() {
        let u1 = parse_url(b"http://example.com/test").unwrap();
        assert_eq!(u1.host, b"example.com");
        assert_eq!(u1.port, 80);
        assert_eq!(u1.path, b"/test");
        assert!(!u1.is_https);

        let u2 = parse_url(b"https://example.com/secure").unwrap();
        assert_eq!(u2.host, b"example.com");
        assert_eq!(u2.port, 443);
        assert_eq!(u2.path, b"/secure");
        assert!(u2.is_https);

        let u3 = parse_url(b"https://example.com:8443/custom").unwrap();
        assert_eq!(u3.host, b"example.com");
        assert_eq!(u3.port, 8443);
        assert_eq!(u3.path, b"/custom");
        assert!(u3.is_https);
    }

    #[test]
    fn test_extract_remote_filename() {
        assert_eq!(extract_remote_filename(b"/install.sh"), b"install.sh");
        assert_eq!(extract_remote_filename(b"/path/to/archive.tar.gz?v=1"), b"archive.tar.gz");
        assert_eq!(extract_remote_filename(b"/"), b"index.html");
    }

    #[test]
    fn test_parse_output_destination() {
        assert_eq!(parse_output_destination(b"-"), None);
        assert_eq!(parse_output_destination(b"output.txt"), Some(&b"output.txt"[..]));
        assert_eq!(parse_output_destination(b"-o"), Some(&b"-o"[..]));
    }
}
