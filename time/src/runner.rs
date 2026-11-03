use libwunix::time::clock_monotonic;
use libwunix::{execve, println, t};
use crate::args::TimeArgs;
use crate::format::TimeResult;

pub fn execute<'a>(args: &'a TimeArgs<'a>) -> Result<TimeResult<'a>, i32> {
    let mut null_cmd = [0u8; 256];
    if args.cmd.len() >= null_cmd.len() {
        println!("{}", t("time: имя команды слишком длинное", "time: command path too long"));
        return Err(1);
    }
    null_cmd[..args.cmd.len()].copy_from_slice(args.cmd);
    null_cmd[args.cmd.len()] = 0;

    let mut argv_storage = [[0u8; 256]; 32];
    let mut argv_ptrs: [*const u8; 33] = [core::ptr::null(); 33];

    let argc = args.cmd_args.len().min(32);
    for (i, &arg) in args.cmd_args.iter().take(argc).enumerate() {
        let len = arg.len().min(255);
        argv_storage[i][..len].copy_from_slice(&arg[..len]);
        argv_storage[i][len] = 0;
        argv_ptrs[i] = argv_storage[i].as_ptr();
    }
    argv_ptrs[argc] = core::ptr::null();

    let start = clock_monotonic();

    let ret = execve(
        &null_cmd[..args.cmd.len() + 1],
        argv_ptrs.as_ptr() as u64,
        0,
    );

    let end = clock_monotonic();

    if ret < 0 {
        let cmd_str = core::str::from_utf8(args.cmd).unwrap_or("");
        println!(
            "{}: '{}'",
            t("time: команда не найдена", "time: command not found"),
            cmd_str
        );
        return Err(127);
    }

    let elapsed_ms = end.to_ms().saturating_sub(start.to_ms());
    let cmd_name = core::str::from_utf8(args.cmd).unwrap_or("");

    Ok(TimeResult {
        elapsed_ms,
        exit_status: ret as i32,
        cmd_name,
    })
}
