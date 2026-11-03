#[derive(Debug, Clone)]
pub struct NtpdateArgs<'a> {
    pub query_only: bool,
    pub step: bool,
    pub verbose: bool,
    pub silent: bool,
    pub port: u16,
    pub servers: [&'a str; 8],
    pub server_count: usize,
}

impl<'a> Default for NtpdateArgs<'a> {
    fn default() -> Self {
        Self {
            query_only: false,
            step: true,
            verbose: false,
            silent: false,
            port: 123,
            servers: [""; 8],
            server_count: 0,
        }
    }
}

impl<'a> NtpdateArgs<'a> {
    pub fn parse(args: &[&'a [u8]]) -> Result<Self, &'static str> {
        let mut parsed = Self::default();
        let mut idx = 0;

        while idx < args.len() {
            let arg = args[idx];
            if arg == b"-h" || arg == b"--help" {
                return Err("help");
            } else if arg == b"-V" || arg == b"--version" {
                return Err("version");
            } else if arg == b"-q" {
                parsed.query_only = true;
            } else if arg == b"-b" {
                parsed.step = true;
            } else if arg == b"-u" {
                // Использовать непривилегированный порт
            } else if arg == b"-s" {
                parsed.silent = true;
            } else if arg == b"-v" {
                parsed.verbose = true;
            } else if arg == b"-p" {
                if idx + 1 < args.len() {
                    idx += 1;
                    let p_arg = args[idx];
                    let mut p = 0u16;
                    for &b in p_arg {
                        if b >= b'0' && b <= b'9' {
                            p = p.saturating_mul(10).saturating_add((b - b'0') as u16);
                        } else {
                            break;
                        }
                    }
                    if p > 0 {
                        parsed.port = p;
                    }
                }
            } else if !arg.starts_with(b"-") {
                if let Ok(server_str) = core::str::from_utf8(arg) {
                    if parsed.server_count < parsed.servers.len() {
                        parsed.servers[parsed.server_count] = server_str;
                        parsed.server_count += 1;
                    }
                }
            }
            idx += 1;
        }

        Ok(parsed)
    }
}
