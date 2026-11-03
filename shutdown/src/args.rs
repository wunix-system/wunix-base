#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownMode {
    PowerOff,
    Reboot,
    Halt,
}

pub struct ShutdownArgs<'a> {
    pub mode: ShutdownMode,
    pub cancel: bool,
    pub warn_only: bool,
    pub no_wall: bool,
    pub delay_secs: u64,
    pub wall_message: Option<&'a str>,
}

impl<'a> ShutdownArgs<'a> {
    pub fn parse(args: &'a [&'a [u8]]) -> Result<Self, &'static str> {
        let mut mode = ShutdownMode::PowerOff;
        let mut cancel = false;
        let mut warn_only = false;
        let mut no_wall = false;
        let mut time_str: Option<&str> = None;
        let mut wall_message: Option<&str> = None;

        let mut i = 0;
        while i < args.len() {
            let arg = args[i];
            let arg_s = core::str::from_utf8(arg).unwrap_or("");

            if arg == b"-h" || arg == b"-P" || arg == b"--poweroff" {
                mode = ShutdownMode::PowerOff;
            } else if arg == b"--help" {
                return Err("help");
            } else if arg == b"-V" || arg == b"--version" {
                return Err("version");
            } else if arg == b"-r" || arg == b"--reboot" {
                mode = ShutdownMode::Reboot;
            } else if arg == b"-H" || arg == b"--halt" {
                mode = ShutdownMode::Halt;
            } else if arg == b"-c" || arg == b"--cancel" {
                cancel = true;
            } else if arg == b"-k" {
                warn_only = true;
            } else if arg == b"--no-wall" {
                no_wall = true;
            } else if !arg.starts_with(b"-") {
                if time_str.is_none() {
                    time_str = Some(arg_s);
                } else if wall_message.is_none() {
                    wall_message = Some(arg_s);
                }
            }
            i += 1;
        }

        let delay_secs = match time_str {
            Some("now") | Some("+0") | None => 0,
            Some(t) if t.starts_with('+') => {
                let mins = libwunix::str_utils::strtoul(&t[1..], 10).unwrap_or(1);
                mins.saturating_mul(60)
            }
            Some(t) if t.len() == 5 && t.as_bytes()[2] == b':' => {
                let target_h = libwunix::str_utils::strtoul(&t[0..2], 10).unwrap_or(0) as u32;
                let target_m = libwunix::str_utils::strtoul(&t[3..5], 10).unwrap_or(0) as u32;
                let now = libwunix::time::clock_realtime();
                let dt = libwunix::time::DateTime::from_epoch(now.tv_sec, 0, 0);
                let current_mins = (dt.hour as u32) * 60 + (dt.minute as u32);
                let target_mins = target_h * 60 + target_m;
                let diff_mins = if target_mins >= current_mins {
                    target_mins - current_mins
                } else {
                    (24 * 60 - current_mins) + target_mins
                };
                (diff_mins as u64).saturating_mul(60)
            }
            Some(t) => {
                // Если указано просто число, трактуем как минуты
                let mins = libwunix::str_utils::strtoul(t, 10).unwrap_or(1);
                mins.saturating_mul(60)
            }
        };

        Ok(Self {
            mode,
            cancel,
            warn_only,
            no_wall,
            delay_secs,
            wall_message,
        })
    }
}
