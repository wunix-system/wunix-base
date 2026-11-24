//! Модуль действий утилиты locale: сбор данных об окружении и доступных локалях.

use crate::format::{print_charmaps, print_locale_settings, print_locales_list};
use libwunix::{current_locale, env};

const STATIC_LOCALES: &[&str] = &[
    "C",
    "POSIX",
    "de_DE.UTF-8",
    "en_US.UTF-8",
    "es_ES.UTF-8",
    "fr_FR.UTF-8",
    "ru_RU.UTF-8",
];

/// Выполняет сбор параметров локали и форматированный вывод.
pub fn show_current_locale() {
    let lc_all = env::getenv("LC_ALL");
    let lc_messages = env::getenv("LC_MESSAGES");
    let env_lang = env::getenv("LANG");

    let lang = env_lang.unwrap_or_else(|| current_locale());
    let effective = lc_all.unwrap_or(lang);

    print_locale_settings(lang, lc_all, lc_messages, effective);
}

/// Выводит список всех установленных/доступных локалей в системе.
pub fn show_all_locales() {
    print_locales_list(STATIC_LOCALES);
}

/// Выводит поддерживаемые кодировки.
pub fn show_charmaps() {
    print_charmaps();
}
