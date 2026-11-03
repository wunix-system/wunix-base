//! Разбор аргументов командной строки утилиты управления службами `system`.

extern crate alloc;
use alloc::string::String;

#[derive(Debug, PartialEq, Eq)]
pub enum SystemAction {
    Start(String),
    Stop(String),
    Restart(String),
    Enable(String),
    Disable(String),
    Status(Option<String>),
    Edit(String),
    Create(String),
    Logs {
        service: String,
        tail: Option<usize>,
        head: Option<usize>,
        follow: bool,
    },
    List,
    Help,
    Version,
}

pub struct SystemArgs {
    pub action: SystemAction,
}

impl SystemArgs {
    pub fn parse(args: &[&str]) -> Result<Self, &'static str> {
        if args.is_empty() {
            return Ok(Self {
                action: SystemAction::List,
            });
        }

        let cmd = args[0];
        match cmd {
            "start" => {
                if args.len() < 2 {
                    return Err("system: укажите имя службы для запуска (system start <служба>)");
                }
                Ok(Self {
                    action: SystemAction::Start(String::from(args[1])),
                })
            }
            "stop" => {
                if args.len() < 2 {
                    return Err("system: укажите имя службы для остановки (system stop <служба>)");
                }
                Ok(Self {
                    action: SystemAction::Stop(String::from(args[1])),
                })
            }
            "restart" => {
                if args.len() < 2 {
                    return Err("system: укажите имя службы для перезапуска (system restart <служба>)");
                }
                Ok(Self {
                    action: SystemAction::Restart(String::from(args[1])),
                })
            }
            "enable" => {
                if args.len() < 2 {
                    return Err("system: укажите имя службы для включения автозапуска (system enable <служба>)");
                }
                Ok(Self {
                    action: SystemAction::Enable(String::from(args[1])),
                })
            }
            "disable" => {
                if args.len() < 2 {
                    return Err("system: укажите имя службы для отключения автозапуска (system disable <служба>)");
                }
                Ok(Self {
                    action: SystemAction::Disable(String::from(args[1])),
                })
            }
            "status" => {
                if args.len() >= 2 {
                    Ok(Self {
                        action: SystemAction::Status(Some(String::from(args[1]))),
                    })
                } else {
                    Ok(Self {
                        action: SystemAction::Status(None),
                    })
                }
            }
            "list" => Ok(Self {
                action: SystemAction::List,
            }),
            "edit" => {
                if args.len() < 2 {
                    return Err("system: укажите имя службы для редактирования (system edit <служба>)");
                }
                Ok(Self {
                    action: SystemAction::Edit(String::from(args[1])),
                })
            }
            "create" => {
                if args.len() < 2 {
                    return Err("system: укажите имя новой службы (system create <служба>)");
                }
                Ok(Self {
                    action: SystemAction::Create(String::from(args[1])),
                })
            }
            "logs" => {
                if args.len() < 2 {
                    return Err("system: укажите имя службы для просмотра журнала (system logs <служба>)");
                }
                let service = String::from(args[1]);
                let mut tail = None;
                let mut head = None;
                let mut follow = false;

                let mut i = 2;
                while i < args.len() {
                    let arg = args[i];
                    if arg == "-f" || arg == "--follow" {
                        follow = true;
                    } else if arg == "-n" || arg == "--lines" {
                        if i + 1 < args.len() {
                            i += 1;
                            tail = parse_usize(args[i]);
                        } else {
                            tail = Some(20);
                        }
                    } else if arg.starts_with("-n") {
                        tail = parse_usize(&arg[2..]);
                    } else if arg == "--head" {
                        if i + 1 < args.len() {
                            i += 1;
                            head = parse_usize(args[i]);
                        } else {
                            head = Some(20);
                        }
                    } else if !arg.starts_with('-') && tail.is_none() {
                        tail = parse_usize(arg);
                    }
                    i += 1;
                }

                // По умолчанию если ни head, ни tail не заданы и не follow — выводим последние 20 строк
                if head.is_none() && tail.is_none() && !follow {
                    tail = Some(20);
                }

                Ok(Self {
                    action: SystemAction::Logs {
                        service,
                        tail,
                        head,
                        follow,
                    },
                })
            }
            "-h" | "--help" | "help" => Ok(Self {
                action: SystemAction::Help,
            }),
            "-V" | "--version" | "version" => Ok(Self {
                action: SystemAction::Version,
            }),
            _ => Err("system: неизвестная команда. Используйте 'system --help' для справки."),
        }
    }
}

fn parse_usize(s: &str) -> Option<usize> {
    let mut val = 0usize;
    if s.is_empty() {
        return None;
    }
    for &b in s.as_bytes() {
        if b >= b'0' && b <= b'9' {
            val = val.checked_mul(10)?.checked_add((b - b'0') as usize)?;
        } else {
            return None;
        }
    }
    Some(val)
}
