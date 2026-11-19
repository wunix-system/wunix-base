#![cfg_attr(test, allow(dead_code))]

use crate::terminal::read_password_line;
use libwunix::{t, verify_password, write, STDERR_FILENO, STDOUT_FILENO};

/// Стандартное количество попыток ввода пароля в sudo (passwd_tries = 3)
pub const MAX_AUTH_TRIES: usize = 3;

/// Решение о следующем шаге после проверки введенного пароля
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AuthAttemptDecision {
    Success,
    Retry,
    Exhausted,
}

/// Чистая логика определения статуса попытки:
/// если пароль неверен, даем повторить, пока не исчерпан лимит max_tries.
pub fn evaluate_auth_attempt(
    is_correct: bool,
    current_attempt: usize,
    max_tries: usize,
) -> AuthAttemptDecision {
    if is_correct {
        AuthAttemptDecision::Success
    } else if current_attempt < max_tries {
        AuthAttemptDecision::Retry
    } else {
        AuthAttemptDecision::Exhausted
    }
}

/// Выполняет интерактивный запрос пароля пользователя с повторными попытками,
/// полностью повторяя поведение классического sudo в Linux.
pub fn authenticate_user(username: &str) -> bool {
    let prompt_pre = t("[sudo] пароль для ", "[sudo] password for ");
    let prompt_post = ": ";
    let retry_msg = t(
        "К сожалению, попробуйте еще раз.\n",
        "Sorry, try again.\n",
    );
    let exhausted_msg = t(
        "sudo: 3 неверные попытки ввода пароля\n",
        "sudo: 3 incorrect password attempts\n",
    );

    for attempt in 1..=MAX_AUTH_TRIES {
        write(STDOUT_FILENO, prompt_pre.as_bytes());
        write(STDOUT_FILENO, username.as_bytes());
        write(STDOUT_FILENO, prompt_post.as_bytes());

        let mut pwd_buf = [0u8; 128];
        let raw_pwd = match read_password_line(&mut pwd_buf) {
            Some(p) => p,
            None => return false,
        };

        if verify_password(username, raw_pwd).is_ok() {
            return true;
        }

        match evaluate_auth_attempt(false, attempt, MAX_AUTH_TRIES) {
            AuthAttemptDecision::Retry => {
                write(STDERR_FILENO, retry_msg.as_bytes());
            }
            AuthAttemptDecision::Exhausted => {
                write(STDERR_FILENO, exhausted_msg.as_bytes());
                return false;
            }
            AuthAttemptDecision::Success => unreachable!(),
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Уведомление для интерфейса пользователя
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub enum AuthNotice {
        Retry,
        Exhausted,
    }

    /// Поток интерактивной аутентификации с абстрагированным вводом и проверкой.
    /// Позволяет честно протестировать поведение циклов и обработку Ctrl+C без реального TTY.
    pub fn authenticate_flow<P, V, N>(
        username: &str,
        max_tries: usize,
        mut get_password: P,
        mut verify: V,
        mut notify: N,
    ) -> bool
    where
        P: FnMut(usize) -> Option<&'static str>,
        V: FnMut(&str, &str) -> bool,
        N: FnMut(AuthNotice),
    {
        for attempt in 1..=max_tries {
            let raw_pwd = match get_password(attempt) {
                Some(p) => p,
                None => {
                    // Пользователь нажал Ctrl+C или закрыл поток ввода — завершаем немедленно
                    return false;
                }
            };

            let is_correct = verify(username, raw_pwd);
            match evaluate_auth_attempt(is_correct, attempt, max_tries) {
                AuthAttemptDecision::Success => return true,
                AuthAttemptDecision::Retry => notify(AuthNotice::Retry),
                AuthAttemptDecision::Exhausted => {
                    notify(AuthNotice::Exhausted);
                    return false;
                }
            }
        }
        false
    }

    #[test]
    fn test_evaluate_auth_attempt_success() {
        assert_eq!(
            evaluate_auth_attempt(true, 1, 3),
            AuthAttemptDecision::Success
        );
        assert_eq!(
            evaluate_auth_attempt(true, 3, 3),
            AuthAttemptDecision::Success
        );
    }

    #[test]
    fn test_evaluate_auth_attempt_retries() {
        assert_eq!(
            evaluate_auth_attempt(false, 1, 3),
            AuthAttemptDecision::Retry
        );
        assert_eq!(
            evaluate_auth_attempt(false, 2, 3),
            AuthAttemptDecision::Retry
        );
        assert_eq!(
            evaluate_auth_attempt(false, 3, 3),
            AuthAttemptDecision::Exhausted
        );
    }

    #[test]
    fn test_flow_success_on_first_try() {
        let mut retries = 0;
        let mut exhausted = false;

        let ok = authenticate_flow(
            "alice",
            3,
            |_| Some("secret"),
            |user, pwd| user == "alice" && pwd == "secret",
            |notice| match notice {
                AuthNotice::Retry => retries += 1,
                AuthNotice::Exhausted => exhausted = true,
            },
        );

        assert!(ok);
        assert_eq!(retries, 0);
        assert!(!exhausted);
    }

    #[test]
    fn test_flow_retry_then_success() {
        let passwords = ["wrong_pass", "secret"];
        let mut prompt_count = 0;
        let mut retries = 0;
        let mut exhausted = false;

        let ok = authenticate_flow(
            "alice",
            3,
            |attempt| {
                prompt_count += 1;
                Some(passwords[attempt - 1])
            },
            |user, pwd| user == "alice" && pwd == "secret",
            |notice| match notice {
                AuthNotice::Retry => retries += 1,
                AuthNotice::Exhausted => exhausted = true,
            },
        );

        assert!(ok);
        assert_eq!(prompt_count, 2);
        assert_eq!(retries, 1);
        assert!(!exhausted);
    }

    #[test]
    fn test_flow_exhausted_after_three_attempts() {
        let mut prompt_count = 0;
        let mut retries = 0;
        let mut exhausted = false;

        let ok = authenticate_flow(
            "alice",
            3,
            |_| {
                prompt_count += 1;
                Some("wrong_password")
            },
            |_, _| false,
            |notice| match notice {
                AuthNotice::Retry => retries += 1,
                AuthNotice::Exhausted => exhausted = true,
            },
        );

        assert!(!ok);
        assert_eq!(prompt_count, 3);
        assert_eq!(retries, 2);
        assert!(exhausted);
    }

    #[test]
    fn test_flow_ctrl_c_cancels_without_retry_message() {
        let mut retries = 0;
        let mut exhausted = false;

        let ok = authenticate_flow(
            "alice",
            3,
            |_| None,
            |_, _| false,
            |notice| match notice {
                AuthNotice::Retry => retries += 1,
                AuthNotice::Exhausted => exhausted = true,
            },
        );

        assert!(!ok);
        assert_eq!(retries, 0);
        assert!(!exhausted);
    }
}
