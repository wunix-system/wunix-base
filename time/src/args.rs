pub struct TimeArgs<'a> {
    pub portability: bool,
    pub verbose: bool,
    pub output_file: Option<&'a str>,
    pub append: bool,
    pub format: Option<&'a str>,
    pub cmd: &'a [u8],
    pub cmd_args: &'a [&'a [u8]],
}

impl<'a> TimeArgs<'a> {
    pub fn parse(args: &'a [&'a [u8]]) -> Result<Self, &'static str> {
        let mut portability = false;
        let mut verbose = false;
        let mut output_file = None;
        let mut append = false;
        let mut format = None;

        let mut i = 0;
        while i < args.len() {
            let arg = args[i];
            let arg_s = core::str::from_utf8(arg).unwrap_or("");

            if arg == b"--" {
                i += 1;
                break;
            } else if arg == b"-h" || arg == b"--help" {
                return Err("help");
            } else if arg == b"-V" || arg == b"--version" {
                return Err("version");
            } else if arg == b"-p" || arg == b"--portability" {
                portability = true;
            } else if arg == b"-v" || arg == b"--verbose" {
                verbose = true;
            } else if arg == b"-a" || arg == b"--append" {
                append = true;
            } else if arg == b"-o" || arg == b"--output" {
                i += 1;
                if i < args.len() {
                    output_file = core::str::from_utf8(args[i]).ok();
                }
            } else if arg_s.starts_with("--output=") {
                output_file = Some(&arg_s[9..]);
            } else if arg == b"-f" || arg == b"--format" {
                i += 1;
                if i < args.len() {
                    format = core::str::from_utf8(args[i]).ok();
                }
            } else if arg_s.starts_with("--format=") {
                format = Some(&arg_s[9..]);
            } else if !arg.starts_with(b"-") {
                break;
            }
            i += 1;
        }

        if i >= args.len() {
            return Err("missing_command");
        }

        let cmd = args[i];
        let cmd_args = &args[i..];

        Ok(Self {
            portability,
            verbose,
            output_file,
            append,
            format,
            cmd,
            cmd_args,
        })
    }
}
