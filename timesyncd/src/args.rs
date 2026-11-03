#[derive(Debug, Clone, Default)]
pub struct TimesyncdArgs {
    pub once: bool,
    pub daemon: bool,
    pub verbose: bool,
}

impl TimesyncdArgs {
    pub fn parse(args: &[&[u8]]) -> Result<Self, &'static str> {
        let mut parsed = Self::default();

        for &arg in args {
            if arg == b"-h" || arg == b"--help" {
                return Err("help");
            } else if arg == b"-V" || arg == b"--version" {
                return Err("version");
            } else if arg == b"-1" || arg == b"--once" {
                parsed.once = true;
            } else if arg == b"-d" || arg == b"--daemon" {
                parsed.daemon = true;
            } else if arg == b"-v" || arg == b"--verbose" {
                parsed.verbose = true;
            }
        }

        Ok(parsed)
    }
}
