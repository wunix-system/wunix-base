use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ManMode {
    Display {
        section: Option<u8>,
        pages: Vec<String>,
        all: bool,
        ascii: bool,
        no_pager: bool,
    },
    Apropos {
        keyword: String,
    },
    Whatis {
        names: Vec<String>,
    },
    Where {
        section: Option<u8>,
        pages: Vec<String>,
    },
    Help,
    Version,
    MissingPage,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CliArgs {
    pub mode: ManMode,
}

impl CliArgs {
    pub fn parse<I, S>(args: I) -> Result<Self, &'static str>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut iter = args.into_iter().skip(1); // пропускаем имя программы (argv[0])
        let mut section: Option<u8> = None;
        let mut pages: Vec<String> = Vec::new();
        let mut all = false;
        let mut ascii = false;
        let mut no_pager = false;
        let mut is_apropos = false;
        let mut is_whatis = false;
        let mut is_where = false;
        let mut apropos_kw = String::new();

        while let Some(arg_ref) = iter.next() {
            let arg = arg_ref.as_ref();
            if arg == "-h" || arg == "--help" {
                return Ok(Self { mode: ManMode::Help });
            }
            if arg == "-V" || arg == "--version" {
                return Ok(Self { mode: ManMode::Version });
            }
            if arg == "-a" || arg == "--all" {
                all = true;
                continue;
            }
            if arg == "-7" || arg == "--ascii" {
                ascii = true;
                continue;
            }
            if arg == "--no-pager" {
                no_pager = true;
                continue;
            }
            if arg == "-k" || arg == "--apropos" {
                is_apropos = true;
                if let Some(next_arg) = iter.next() {
                    apropos_kw = String::from(next_arg.as_ref());
                }
                continue;
            }
            if arg.starts_with("-k") && arg.len() > 2 {
                is_apropos = true;
                apropos_kw = String::from(&arg[2..]);
                continue;
            }
            if arg == "-f" || arg == "--whatis" {
                is_whatis = true;
                continue;
            }
            if arg == "-w" || arg == "-W" || arg == "--where" || arg == "--path" {
                is_where = true;
                continue;
            }
            if arg == "-s" || arg == "--section" {
                if let Some(next_arg) = iter.next() {
                    if let Ok(sec) = next_arg.as_ref().parse::<u8>() {
                        section = Some(sec);
                    }
                }
                continue;
            }
            if let Some(val) = arg.strip_prefix("-s") {
                if let Ok(sec) = val.parse::<u8>() {
                    section = Some(sec);
                    continue;
                }
            }
            if let Some(val) = arg.strip_prefix("--section=") {
                if let Ok(sec) = val.parse::<u8>() {
                    section = Some(sec);
                    continue;
                }
            }
            if arg.starts_with('-') {
                // Неизвестный ключ
                continue;
            }

            // Позиционный аргумент: если это цифра 1..8 и секция еще не была задана, и страниц пока нет
            if section.is_none() && pages.is_empty() && arg.len() == 1 {
                if let Ok(sec) = arg.parse::<u8>() {
                    if (1..=8).contains(&sec) {
                        section = Some(sec);
                        continue;
                    }
                }
            }

            pages.push(String::from(arg));
        }

        if is_apropos {
            if apropos_kw.is_empty() {
                if let Some(first) = pages.first() {
                    apropos_kw = first.clone();
                }
            }
            if apropos_kw.is_empty() {
                return Ok(Self { mode: ManMode::MissingPage });
            }
            return Ok(Self {
                mode: ManMode::Apropos { keyword: apropos_kw },
            });
        }

        if is_whatis {
            if pages.is_empty() {
                return Ok(Self { mode: ManMode::MissingPage });
            }
            return Ok(Self {
                mode: ManMode::Whatis { names: pages },
            });
        }

        if is_where {
            return Ok(Self {
                mode: ManMode::Where { section, pages },
            });
        }

        if pages.is_empty() {
            return Ok(Self {
                mode: ManMode::MissingPage,
            });
        }

        Ok(Self {
            mode: ManMode::Display {
                section,
                pages,
                all,
                ascii,
                no_pager,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_parse_help_and_version() {
        let args = vec!["man", "--help"];
        let parsed = CliArgs::parse(args).unwrap();
        assert_eq!(parsed.mode, ManMode::Help);

        let args = vec!["man", "-V"];
        let parsed = CliArgs::parse(args).unwrap();
        assert_eq!(parsed.mode, ManMode::Version);
    }

    #[test]
    fn test_parse_missing_page() {
        let args = vec!["man"];
        let parsed = CliArgs::parse(args).unwrap();
        assert_eq!(parsed.mode, ManMode::MissingPage);
    }

    #[test]
    fn test_parse_simple_page() {
        let args = vec!["man", "cat"];
        let parsed = CliArgs::parse(args).unwrap();
        assert_eq!(
            parsed.mode,
            ManMode::Display {
                section: None,
                pages: vec![String::from("cat")],
                all: false,
                ascii: false,
                no_pager: false,
            }
        );
    }

    #[test]
    fn test_parse_section_and_page() {
        let args = vec!["man", "1", "ls"];
        let parsed = CliArgs::parse(args).unwrap();
        assert_eq!(
            parsed.mode,
            ManMode::Display {
                section: Some(1),
                pages: vec![String::from("ls")],
                all: false,
                ascii: false,
                no_pager: false,
            }
        );

        let args = vec!["man", "-s", "2", "write"];
        let parsed = CliArgs::parse(args).unwrap();
        assert_eq!(
            parsed.mode,
            ManMode::Display {
                section: Some(2),
                pages: vec![String::from("write")],
                all: false,
                ascii: false,
                no_pager: false,
            }
        );
    }

    #[test]
    fn test_parse_flags() {
        let args = vec!["man", "-a", "-7", "--no-pager", "wsh"];
        let parsed = CliArgs::parse(args).unwrap();
        assert_eq!(
            parsed.mode,
            ManMode::Display {
                section: None,
                pages: vec![String::from("wsh")],
                all: true,
                ascii: true,
                no_pager: true,
            }
        );
    }

    #[test]
    fn test_parse_apropos_and_whatis() {
        let args = vec!["man", "-k", "terminal"];
        let parsed = CliArgs::parse(args).unwrap();
        assert_eq!(
            parsed.mode,
            ManMode::Apropos {
                keyword: String::from("terminal"),
            }
        );

        let args = vec!["man", "-f", "cat", "ls"];
        let parsed = CliArgs::parse(args).unwrap();
        assert_eq!(
            parsed.mode,
            ManMode::Whatis {
                names: vec![String::from("cat"), String::from("ls")],
            }
        );
    }

    #[test]
    fn test_parse_where() {
        let args = vec!["man", "-w", "1", "cat"];
        let parsed = CliArgs::parse(args).unwrap();
        assert_eq!(
            parsed.mode,
            ManMode::Where {
                section: Some(1),
                pages: vec![String::from("cat")],
            }
        );

        let args = vec!["man", "-w"];
        let parsed = CliArgs::parse(args).unwrap();
        assert_eq!(
            parsed.mode,
            ManMode::Where {
                section: None,
                pages: vec![],
            }
        );
    }
}
