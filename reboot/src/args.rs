#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebootAction {
    Reboot,
    Poweroff,
    Halt,
}

pub struct RebootArgs {
    pub action: RebootAction,
    pub force: bool,
    pub wtmp_only: bool,
    pub no_sync: bool,
    pub no_wall: bool,
}

impl RebootArgs {
    pub fn parse(args: &[&[u8]]) -> Result<Self, &'static str> {
        let mut action = RebootAction::Reboot;
        let mut force = false;
        let mut wtmp_only = false;
        let mut no_sync = false;
        let mut no_wall = false;

        let mut i = 0;
        while i < args.len() {
            let arg = args[i];

            if arg == b"-h" || arg == b"--help" {
                return Err("help");
            } else if arg == b"-V" || arg == b"--version" {
                return Err("version");
            } else if arg == b"-f" || arg == b"--force" {
                force = true;
            } else if arg == b"-p" || arg == b"--poweroff" {
                action = RebootAction::Poweroff;
            } else if arg == b"-d" || arg == b"--halt" {
                action = RebootAction::Halt;
            } else if arg == b"-w" || arg == b"--wtmp-only" {
                wtmp_only = true;
            } else if arg == b"-n" || arg == b"--no-sync" {
                no_sync = true;
            } else if arg == b"--no-wall" {
                no_wall = true;
            }
            i += 1;
        }

        Ok(Self {
            action,
            force,
            wtmp_only,
            no_sync,
            no_wall,
        })
    }
}
