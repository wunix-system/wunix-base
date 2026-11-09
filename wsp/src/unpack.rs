//! Модуль распаковки системных пакетов ядра и базы Wunix Package (.wp) для WSP.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{chmod, close, mkdir, open, read, symlink, write, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY};
use libpackwunix::format::PackageContainer;
use libpackwunix::manifest::{PackageManifest, PackageType};

const BLOCK_SIZE: usize = 512;

/// Считывает файл целиком в буфер памяти
pub fn read_file_to_vec(path: &str) -> Result<Vec<u8>, &'static str> {
    let mut null_path = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() >= null_path.len() - 1 {
        return Err("Слишком длинный путь к файлу пакета");
    }
    null_path[..bytes.len()].copy_from_slice(bytes);
    null_path[bytes.len()] = 0;

    let fd = open(&null_path, O_RDONLY as u64, 0);
    if fd < 0 {
        return Err("Не удалось открыть файл системного пакета");
    }

    let mut result = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = read(fd as u64, &mut chunk);
        if n == 0 {
            break;
        }
        result.extend_from_slice(&chunk[..n as usize]);
    }
    close(fd as u64);

    Ok(result)
}

fn is_safe_path(path: &str) -> bool {
    let b = path.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'.' && i + 1 < b.len() && b[i + 1] == b'.' {
            let prev_sep = i == 0 || b[i - 1] == b'/';
            let next_sep = i + 2 == b.len() || b[i + 2] == b'/';
            if prev_sep && next_sep {
                return false;
            }
        }
        i += 1;
    }
    true
}

fn ensure_parent_dirs(full_path: &str) {
    let bytes = full_path.as_bytes();
    let mut temp = [0u8; 512];
    let len = bytes.len().min(511);
    temp[..len].copy_from_slice(&bytes[..len]);

    for i in 1..len {
        if temp[i] == b'/' {
            temp[i] = 0;
            let _ = mkdir(&temp[..i + 1], 0o755);
            temp[i] = b'/';
        }
    }
}

fn parse_octal(bytes: &[u8]) -> u64 {
    let mut val = 0u64;
    let mut started = false;
    for &b in bytes {
        if (b'0'..=b'7').contains(&b) {
            val = (val << 3) | ((b - b'0') as u64);
            started = true;
        } else if b == 0 || b == b' ' {
            if started {
                break;
            }
        } else {
            break;
        }
    }
    val
}

fn cstr_slice(bytes: &[u8]) -> &str {
    let mut len = 0;
    while len < bytes.len() && bytes[len] != 0 {
        len += 1;
    }
    core::str::from_utf8(&bytes[..len]).unwrap_or("")
}

/// Распаковывает системный пакет WSP (.wp) в файловую систему ядра
pub fn unpack_wsp_package(pkg_bytes: &[u8], verbose: bool) -> Result<(PackageManifest, Vec<String>), &'static str> {
    let container = PackageContainer::parse(pkg_bytes)?;

    if container.manifest.package_type == PackageType::User {
        return Err("Данный пакет является прикладной программой. Для пользовательских программ используйте 'wpm'.");
    }

    let mut tar_bytes = Vec::new();
    if container.payload.len() >= 2 && container.payload[0] == 0x1f && container.payload[1] == 0x8b {
        libwunix::compression::gzip::gzip_decompress(container.payload, |chunk| {
            tar_bytes.extend_from_slice(chunk);
            Ok(())
        }).map_err(|_| "Ошибка распаковки сжатого системного архива gzip")?;
    } else {
        tar_bytes.extend_from_slice(container.payload);
    }

    let mut offset = 0usize;
    let mut installed_files: Vec<String> = Vec::new();

    while offset + BLOCK_SIZE <= tar_bytes.len() {
        let block = &tar_bytes[offset..offset + BLOCK_SIZE];
        if block[0] == 0 {
            break;
        }

        let name = cstr_slice(&block[0..100]);
        let prefix = cstr_slice(&block[345..500]);
        let typeflag = block[156];
        let file_size = parse_octal(&block[124..136]) as usize;

        let raw_path = if !prefix.is_empty() {
            format!("{}/{}", prefix, name)
        } else {
            name.to_string()
        };

        let clean_path = raw_path
            .trim_start_matches('/')
            .trim_start_matches("./");

        if clean_path.is_empty() || clean_path == "manifest.toml" || !is_safe_path(clean_path) {
            let padded_size = ((file_size + BLOCK_SIZE - 1) / BLOCK_SIZE) * BLOCK_SIZE;
            offset += BLOCK_SIZE + padded_size;
            continue;
        }

        let target_path = format!("/{}", clean_path);
        let mut null_target = [0u8; 512];
        let t_bytes = target_path.as_bytes();
        let copy_len = t_bytes.len().min(511);
        null_target[..copy_len].copy_from_slice(&t_bytes[..copy_len]);
        null_target[copy_len] = 0;

        ensure_parent_dirs(&target_path);

        match typeflag {
            b'5' => {
                let _ = mkdir(&null_target, 0o755);
            }
            b'2' => {
                let linkname = cstr_slice(&block[157..257]);
                if !linkname.is_empty() {
                    let mut null_link = [0u8; 256];
                    let l_bytes = linkname.as_bytes();
                    let l_len = l_bytes.len().min(255);
                    null_link[..l_len].copy_from_slice(&l_bytes[..l_len]);
                    null_link[l_len] = 0;

                    let _ = symlink(&null_link, &null_target);
                    installed_files.push(target_path.clone());
                    if verbose {
                        libwunix::println!("       ↳ {} -> {}", target_path, linkname);
                    }
                }
            }
            _ => {
                let data_start = offset + BLOCK_SIZE;
                let data_end = data_start + file_size;

                if data_end > tar_bytes.len() {
                    return Err("Поврежденный архив: срез файла выходит за границы tar данных");
                }

                let file_data = &tar_bytes[data_start..data_end];
                let is_bin = target_path.starts_with("/bin/") || target_path.starts_with("/boot/");
                let mode: u32 = if is_bin { 0o755 } else { 0o644 };

                let fd = open(&null_target, (O_CREAT | O_WRONLY | O_TRUNC) as u64, mode as u64);
                if fd >= 0 {
                    let _ = write(fd as u64, file_data);
                    close(fd as u64);
                    let _ = chmod(&null_target, mode);
                    installed_files.push(target_path.clone());
                    if verbose {
                        libwunix::println!("       ↳ {}", target_path);
                    }
                }
            }
        }

        let padded_size = ((file_size + BLOCK_SIZE - 1) / BLOCK_SIZE) * BLOCK_SIZE;
        offset += BLOCK_SIZE + padded_size;
    }

    Ok((container.manifest, installed_files))
}
