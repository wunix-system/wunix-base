pub enum IsoFormat {
    Date,
    Hours,
    Minutes,
    Seconds,
    Nanoseconds,
}

pub struct DateArgs<'a> {
    pub is_utc: bool,
    pub rfc_email: bool,
    pub iso_8601: Option<IsoFormat>,
    pub date_str: Option<&'a str>,
    pub set_str: Option<&'a str>,
    pub reference_file: Option<&'a str>,
    pub custom_format: Option<&'a str>,
}

impl<'a> DateArgs<'a> {
    pub fn parse(args: &'a [&'a [u8]]) -> Result<Self, &'static str> {
        let mut is_utc = false;
        let mut rfc_email = false;
        let mut iso_8601 = None;
        let mut date_str = None;
        let mut set_str = None;
        let mut reference_file = None;
        let mut custom_format = None;

        let mut i = 0;
        while i < args.len() {
            let arg = args[i];
            let arg_s = core::str::from_utf8(arg).unwrap_or("");

            if arg == b"-h" || arg == b"--help" {
                return Err("help");
            } else if arg == b"-V" || arg == b"--version" {
                return Err("version");
            } else if arg == b"-u" || arg == b"--utc" || arg == b"--universal" {
                is_utc = true;
            } else if arg == b"-R" || arg == b"--rfc-email" {
                rfc_email = true;
            } else if arg == b"-I" || arg == b"--iso-8601" {
                iso_8601 = Some(IsoFormat::Date);
            } else if arg_s.starts_with("-I") {
                let sub = &arg_s[2..];
                iso_8601 = Some(parse_iso_spec(sub));
            } else if arg_s.starts_with("--iso-8601=") {
                let sub = &arg_s[11..];
                iso_8601 = Some(parse_iso_spec(sub));
            } else if arg == b"-d" || arg == b"--date" {
                i += 1;
                if i < args.len() {
                    date_str = core::str::from_utf8(args[i]).ok();
                }
            } else if arg_s.starts_with("--date=") {
                date_str = Some(&arg_s[7..]);
            } else if arg == b"-s" || arg == b"--set" {
                i += 1;
                if i < args.len() {
                    set_str = core::str::from_utf8(args[i]).ok();
                }
            } else if arg_s.starts_with("--set=") {
                set_str = Some(&arg_s[6..]);
            } else if arg == b"-r" || arg == b"--reference" {
                i += 1;
                if i < args.len() {
                    reference_file = core::str::from_utf8(args[i]).ok();
                }
            } else if arg_s.starts_with("--reference=") {
                reference_file = Some(&arg_s[12..]);
            } else if arg_s.starts_with('+') {
                custom_format = Some(&arg_s[1..]);
            }
            i += 1;
        }

        Ok(Self {
            is_utc,
            rfc_email,
            iso_8601,
            date_str,
            set_str,
            reference_file,
            custom_format,
        })
    }
}

fn parse_iso_spec(s: &str) -> IsoFormat {
    match s {
        "hours" => IsoFormat::Hours,
        "minutes" => IsoFormat::Minutes,
        "seconds" => IsoFormat::Seconds,
        "ns" => IsoFormat::Nanoseconds,
        _ => IsoFormat::Date,
    }
}
