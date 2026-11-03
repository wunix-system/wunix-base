use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

pub struct ManPageEntry {
    pub name: &'static str,
    pub section: u8,
    pub desc_en: &'static str,
    pub desc_ru: &'static str,
    pub roff_en: &'static str,
    pub roff_ru: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundPage {
    pub name: String,
    pub section: u8,
    pub path: String,
    pub roff_content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AproposMatch {
    pub name: String,
    pub section: u8,
    pub description: String,
}

pub static BUILTIN_PAGES: &[ManPageEntry] = &[
    // --- СЕКЦИЯ 1: Команды пользователя ---
    ManPageEntry {
        name: "man",
        section: 1,
        desc_en: "an interface to the system reference manuals",
        desc_ru: "интерфейс к справочным руководствам операционной системы",
        roff_en: r#".TH MAN 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
man \- an interface to the system reference manuals
.SH SYNOPSIS
\fBman\fR [\fIOPTION\fR...] [\fISECTION\fR] \fIPAGE\fR...
.br
\fBman\fR \fB\-k\fR \fIKEYWORD\fR...
.br
\fBman\fR \fB\-f\fR \fIPAGE\fR...
.SH DESCRIPTION
\fBman\fR is the system's manual pager. Each \fIPAGE\fR argument given to \fBman\fR is normally the name of a program, utility or function. The manual page associated with each of these arguments is then found and displayed. A \fISECTION\fR, if provided, will direct \fBman\fR to look only in that section of the manual.
.SH SECTIONS
The standard sections of the manual include:
.TP
\fB1\fR
Executable programs or shell commands
.TP
\fB2\fR
System calls (functions provided by the kernel)
.TP
\fB3\fR
Library calls (functions within program libraries)
.TP
\fB4\fR
Special files (usually found in /dev)
.TP
\fB5\fR
File formats and conventions, e.g. /etc/passwd
.TP
\fB6\fR
Games
.TP
\fB7\fR
Miscellaneous (including macro packages and conventions)
.TP
\fB8\fR
System administration commands (usually only for root)
.SH OPTIONS
.TP
\fB\-s\fR \fISECTION\fR, \fB\-\-section\fR=\fISECTION\fR
Look only in the specified section of the manual.
.TP
\fB\-a\fR, \fB\-\-all\fR
Display all matching manual pages found across all sections.
.TP
\fB\-k\fR, \fB\-\-apropos\fR
Search the short descriptions and manual page names for keywords (equivalent to apropos).
.TP
\fB\-f\fR, \fB\-\-whatis\fR
Display short one-line description of the specified manual pages (equivalent to whatis).
.TP
\fB\-w\fR, \fB\-\-where\fR, \fB\-\-path\fR
Print the physical location of the manual page files, or MANPATH search directories.
.TP
\fB\-7\fR, \fB\-\-ascii\fR
Output plain 7-bit ASCII text without ANSI escape sequences or text styling.
.TP
\fB\-\-no-pager\fR
Do not pipe output to the interactive pager; write directly to stdout.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display a short help message and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Display version information and exit.
.SH "EXIT STATUS"
.TP
\fB0\fR
Successful program execution.
.TP
\fB1\fR
Usage, syntax or configuration file error.
.TP
\fB16\fR
Nothing was found for the specified manual page.
.SH "SEE ALSO"
\fBwsh\fR(1), \fBless\fR(1), \fBcat\fR(1)
"#,
        roff_ru: r#".TH MAN 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
man \- интерфейс к справочным руководствам операционной системы
.SH СИНТАКСИС
\fBman\fR [\fIОПЦИИ\fR...] [\fIРАЗДЕЛ\fR] \fIСТРАНИЦА\fR...
.br
\fBman\fR \fB\-k\fR \fIКЛЮЧЕВОЕ_СЛОВО\fR...
.br
\fBman\fR \fB\-f\fR \fIСТРАНИЦА\fR...
.SH ОПИСАНИЕ
\fBman\fR является стандартным средством просмотра документации Wunix. В качестве аргумента \fIСТРАНИЦА\fR обычно указывается имя программы, утилиты или системного вызова. Программа находит соответствующую страницу руководства и выводит ее через встроенный пейджер с подсветкой и навигацией.
.SH РАЗДЕЛЫ
Справочное руководство разделено на следующие стандартные разделы:
.TP
\fB1\fR
Пользовательские программы и команды оболочки
.TP
\fB2\fR
Системные вызовы ядра операционной системы
.TP
\fB3\fR
Функции библиотек ядра и libwunix
.TP
\fB4\fR
Специальные файлы и виртуальные устройства (/dev)
.TP
\fB5\fR
Форматы системных конфигурационных файлов
.TP
\fB6\fR
Игры и демонстрационные программы
.TP
\fB7\fR
Общие соглашения, сетевые протоколы и архитектура
.TP
\fB8\fR
Команды системного администрирования
.SH ОПЦИИ
.TP
\fB\-s\fR \fIРАЗДЕЛ\fR, \fB\-\-section\fR=\fIРАЗДЕЛ\fR
Ограничить поиск страниц только указанным разделом.
.TP
\fB\-a\fR, \fB\-\-all\fR
Отобразить по очереди все найденные страницы из всех разделов.
.TP
\fB\-k\fR, \fB\-\-apropos\fR
Поиск по ключевому слову в кратких описаниях (аналог apropos).
.TP
\fB\-f\fR, \fB\-\-whatis\fR
Вывод однострочной справки по имени страницы (аналог whatis).
.TP
\fB\-w\fR, \fB\-\-where\fR, \fB\-\-path\fR
Вывод физического пути к файлу страницы или путей каталогов MANPATH.
.TP
\fB\-7\fR, \fB\-\-ascii\fR
Вывод простого текста без управляющих escape-последовательностей ANSI.
.TP
\fB\-\-no-pager\fR
Отключить интерактивный пейджер и вывести весь текст страницы в stdout.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту краткую справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Показать информацию о версии и выйти.
.SH "КОДЫ ВОЗВРАТА"
.TP
\fB0\fR
Успешное выполнение.
.TP
\fB1\fR
Ошибка в параметрах или синтаксисе вызова.
.TP
\fB16\fR
Страница руководства с указанным именем не найдена.
.SH "СМ. ТАКЖЕ"
\fBwsh\fR(1), \fBless\fR(1), \fBcat\fR(1)
"#,
    },
    ManPageEntry {
        name: "cat",
        section: 1,
        desc_en: "concatenate files and print on the standard output",
        desc_ru: "объединение и вывод содержимого файлов",
        roff_en: r#".TH CAT 1 "2026-10-28" "Wunix" "User Commands"
.SH NAME
cat \- concatenate files and print on the standard output
.SH SYNOPSIS
\fBcat\fR [\fIOPTION\fR]... [\fIFILE\fR]...
.SH DESCRIPTION
Concatenate FILE(s) to standard output. With no FILE, or when FILE is -, read standard input.
.SH OPTIONS
.TP
\fB\-\-help\fR
display this help and exit
.TP
\fB\-\-version\fR
output version information and exit
.SH EXAMPLES
.TP
\fBcat f \- g\fR
Output f's contents, then standard input, then g's contents.
.TP
\fBcat\fR
Copy standard input to standard output.
.SH "SEE ALSO"
\fBless\fR(1), \fBhead\fR(1), \fBtail\fR(1)
"#,
        roff_ru: r#".TH CAT 1 "2026-10-28" "Wunix" "Команды пользователя"
.SH ИМЯ
cat \- объединение и вывод содержимого файлов
.SH СИНТАКСИС
\fBcat\fR [\fIОПЦИИ\fR]... [\fIФАЙЛ\fR]...
.SH ОПИСАНИЕ
Сцепляет указанные файлы и выводит их содержимое в стандартный вывод. Если ФАЙЛ не задан или равен '-', читает стандартный ввод.
.SH ОПЦИИ
.TP
\fB\-\-help\fR
показать справку и выйти
.TP
\fB\-\-version\fR
вывести версию программы
.SH ПРИМЕРЫ
.TP
\fBcat /etc/hostname\fR
Вывести сетевое имя компьютера.
.TP
\fBcat file1 file2 > out\fR
Объединить два файла в один.
.SH "СМ. ТАКЖЕ"
\fBless\fR(1), \fBhead\fR(1), \fBtail\fR(1)
"#,
    },
    ManPageEntry {
        name: "ls",
        section: 1,
        desc_en: "list directory contents",
        desc_ru: "вывод содержимого каталогов",
        roff_en: r#".TH LS 1 "2026-10-28" "Wunix" "User Commands"
.SH NAME
ls \- list directory contents
.SH SYNOPSIS
\fBls\fR [\fIOPTION\fR]... [\fIFILE\fR]...
.SH DESCRIPTION
List information about the FILEs (the current directory by default).
.SH OPTIONS
.TP
\fB\-a\fR, \fB\-\-all\fR
do not ignore entries starting with .
.TP
\fB\-l\fR
use a long listing format
.TP
\fB\-\-help\fR
display help and exit
.SH "SEE ALSO"
\fBstat\fR(1), \fBtree\fR(1)
"#,
        roff_ru: r#".TH LS 1 "2026-10-28" "Wunix" "Команды пользователя"
.SH ИМЯ
ls \- вывод содержимого каталогов
.SH СИНТАКСИС
\fBls\fR [\fIОПЦИИ\fR]... [\fIФАЙЛ\fR]...
.SH ОПИСАНИЕ
Выводит список файлов и папок в указанных каталогах (по умолчанию в текущей рабочей директории).
.SH ОПЦИИ
.TP
\fB\-a\fR, \fB\-\-all\fR
не скрывать файлы, начинающиеся с точки
.TP
\fB\-l\fR
выводить в расширенном формате с правами и размером
.TP
\fB\-\-help\fR
показать справку и выйти
.SH "СМ. ТАКЖЕ"
\fBstat\fR(1), \fBtree\fR(1)
"#,
    },
    ManPageEntry {
        name: "wsh",
        section: 1,
        desc_en: "Wunix command language interpreter (shell)",
        desc_ru: "командная оболочка Wunix",
        roff_en: r#".TH WSH 1 "2026-10-29" "Wunix" "User Commands"
.SH NAME
wsh \- Wunix command language interpreter (shell)
.SH SYNOPSIS
\fBwsh\fR [\fIOPTION\fR]... [\fISCRIPT\fR]
.SH DESCRIPTION
\fBwsh\fR is an interactive command language interpreter that executes commands read from standard input or from a script file. It features rich line editing, history navigation, pipe chaining, environment variable expansion, and job control.
.SH "BUILT-IN COMMANDS"
.TP
\fBcd\fR [\fIPATH\fR]
Change current working directory.
.TP
\fBpwd\fR
Print working directory.
.TP
\fBexit\fR [\fISTATUS\fR]
Exit the current shell session.
.TP
\fBexport\fR \fIVAR\fR=\fIVAL\fR
Set environment variable.
.TP
\fBhistory\fR
Display command history.
.SH "SEE ALSO"
\fBman\fR(1), \fBinit\fR(8)
"#,
        roff_ru: r#".TH WSH 1 "2026-10-29" "Wunix" "Команды пользователя"
.SH ИМЯ
wsh \- командная оболочка Wunix
.SH СИНТАКСИС
\fBwsh\fR [\fIОПЦИИ\fR]... [\fIСКРИПТ\fR]
.SH ОПИСАНИЕ
\fBwsh\fR является интерактивной командной оболочкой Wunix. Поддерживает редактирование строк с помощью горячих клавиш, историю команд, конвейеры (пайпы), перенаправление потоков ввода-вывода и переменные окружения.
.SH "ВСТРОЕННЫЕ КОМАНДЫ"
.TP
\fBcd\fR [\fIПУТЬ\fR]
Сменить текущий рабочий каталог.
.TP
\fBpwd\fR
Вывести путь к текущему каталогу.
.TP
\fBexit\fR [\fIКОД\fR]
Завершить сессию командной оболочки.
.TP
\fBexport\fR \fIПЕРЕМЕННАЯ\fR=\fIЗНАЧЕНИЕ\fR
Установить переменную окружения.
.TP
\fBhistory\fR
Показать список выполненных команд.
.SH "СМ. ТАКЖЕ"
\fBman\fR(1), \fBinit\fR(8)
"#,
    },
    ManPageEntry {
        name: "uname",
        section: 1,
        desc_en: "print system information",
        desc_ru: "вывод информации о системе",
        roff_en: r#".TH UNAME 1 "2026-10-28" "Wunix" "User Commands"
.SH NAME
uname \- print system information
.SH SYNOPSIS
\fBuname\fR [\fIOPTION\fR]...
.SH DESCRIPTION
Print certain system information. With no OPTION, same as -s.
.SH OPTIONS
.TP
\fB\-a\fR, \fB\-\-all\fR
print all information
.TP
\fB\-s\fR, \fB\-\-kernel\-name\fR
print the kernel name
.TP
\fB\-n\fR, \fB\-\-nodename\fR
print the network node hostname
.TP
\fB\-r\fR, \fB\-\-kernel\-release\fR
print the kernel release
.TP
\fB\-m\fR, \fB\-\-machine\fR
print the machine hardware name
.SH "SEE ALSO"
\fBuname\fR(2)
"#,
        roff_ru: r#".TH UNAME 1 "2026-10-28" "Wunix" "Команды пользователя"
.SH ИМЯ
uname \- вывод информации о системе
.SH СИНТАКСИС
\fBuname\fR [\fIОПЦИИ\fR]...
.SH ОПИСАНИЕ
Выводит информацию об операционной системе, версии ядра и аппаратной архитектуре. Без параметров эквивалентно -s.
.SH ОПЦИИ
.TP
\fB\-a\fR, \fB\-\-all\fR
вывести всю системную информацию
.TP
\fB\-s\fR, \fB\-\-kernel\-name\fR
вывести имя ядра (Linux / Wunix)
.TP
\fB\-n\fR, \fB\-\-nodename\fR
вывести сетевое имя узла (hostname)
.TP
\fB\-r\fR, \fB\-\-kernel\-release\fR
вывести релиз ядра
.TP
\fB\-m\fR, \fB\-\-machine\fR
вывести архитектуру процессора (x86_64)
.SH "СМ. ТАКЖЕ"
\fBuname\fR(2)
"#,
    },
    ManPageEntry {
        name: "less",
        section: 1,
        desc_en: "opposite of more, terminal pager",
        desc_ru: "терминальный пейджер для просмотра текста",
        roff_en: r#".TH LESS 1 "2026-10-28" "Wunix" "User Commands"
.SH NAME
less \- opposite of more, terminal pager
.SH SYNOPSIS
\fBless\fR [\fIFILE\fR]
.SH DESCRIPTION
\fBless\fR is a program similar to \fBmore\fR(1), but which allows backward movement in the file as well as forward movement.
.SH COMMANDS
.TP
\fBq\fR, \fBQ\fR
Exit less.
.TP
\fBSpace\fR, \fBf\fR
Scroll forward one screenful.
.TP
\fBb\fR
Scroll backward one screenful.
.TP
\fBEnter\fR, \fBj\fR
Scroll forward one line.
.TP
\fBk\fR
Scroll backward one line.
.SH "SEE ALSO"
\fBmore\fR(1), \fBman\fR(1)
"#,
        roff_ru: r#".TH LESS 1 "2026-10-28" "Wunix" "Команды пользователя"
.SH ИМЯ
less \- терминальный пейджер для просмотра текста
.SH СИНТАКСИС
\fBless\fR [\fIФАЙЛ\fR]
.SH ОПИСАНИЕ
\fBless\fR позволяет постранично просматривать файлы и потоки ввода с поддержкой двунаправленной прокрутки.
.SH КЛАВИШИ УПРАВЛЕНИЯ
.TP
\fBq\fR, \fBQ\fR
Выход из пейджера.
.TP
\fBПробел\fR, \fBf\fR
Прокрутка на один экран вперед.
.TP
\fBb\fR
Прокрутка на один экран назад.
.TP
\fBEnter\fR, \fBj\fR
Смещение на одну строку вниз.
.TP
\fBk\fR
Смещение на одну строку вверх.
.SH "СМ. ТАКЖЕ"
\fBmore\fR(1), \fBman\fR(1)
"#,
    },
    // --- СЕКЦИЯ 2: Системные вызовы ядра ---
    ManPageEntry {
        name: "read",
        section: 2,
        desc_en: "read from a file descriptor",
        desc_ru: "чтение данных из файлового дескриптора",
        roff_en: r#".TH READ 2 "2026-10-28" "Wunix" "System Calls"
.SH NAME
read \- read from a file descriptor
.SH SYNOPSIS
\fB#include <unistd.h>\fR
.sp
\fBssize_t read(int \fIfd\fR, void *\fIbuf\fR, size_t \fIcount\fR);\fR
.SH DESCRIPTION
\fBread\fR() attempts to read up to \fIcount\fR bytes from file descriptor \fIfd\fR into the buffer starting at \fIbuf\fR.
.SH "RETURN VALUE"
On success, the number of bytes read is returned (zero indicates end of file). On error, -1 is returned.
.SH "SEE ALSO"
\fBclose\fR(2), \fBopen\fR(2), \fBwrite\fR(2)
"#,
        roff_ru: r#".TH READ 2 "2026-10-28" "Wunix" "Системные вызовы"
.SH ИМЯ
read \- чтение данных из файлового дескриптора
.SH СИНТАКСИС
\fBssize_t read(int \fIfd\fR, void *\fIbuf\fR, size_t \fIcount\fR);\fR
.SH ОПИСАНИЕ
Системный вызов \fBread\fR() считывает до \fIcount\fR байт из файлового дескриптора \fIfd\fR в буфер памяти, на который указывает \fIbuf\fR.
.SH "ВОЗВРАЩАЕМОЕ ЗНАЧЕНИЕ"
При успехе возвращается количество прочитанных байт (0 означает конец файла). При ошибке возвращается отрицательный код ошибки.
.SH "СМ. ТАКЖЕ"
\fBclose\fR(2), \fBopen\fR(2), \fBwrite\fR(2)
"#,
    },
    ManPageEntry {
        name: "write",
        section: 2,
        desc_en: "write to a file descriptor",
        desc_ru: "запись данных в файловый дескриптор",
        roff_en: r#".TH WRITE 2 "2026-10-28" "Wunix" "System Calls"
.SH NAME
write \- write to a file descriptor
.SH SYNOPSIS
\fB#include <unistd.h>\fR
.sp
\fBssize_t write(int \fIfd\fR, const void *\fIbuf\fR, size_t \fIcount\fR);\fR
.SH DESCRIPTION
\fBwrite\fR() writes up to \fIcount\fR bytes from the buffer starting at \fIbuf\fR to the file referred to by the file descriptor \fIfd\fR.
.SH "RETURN VALUE"
On success, the number of bytes written is returned. On error, -1 is returned.
.SH "SEE ALSO"
\fBclose\fR(2), \fBopen\fR(2), \fBread\fR(2)
"#,
        roff_ru: r#".TH WRITE 2 "2026-10-28" "Wunix" "Системные вызовы"
.SH ИМЯ
write \- запись данных в файловый дескриптор
.SH СИНТАКСИС
\fBssize_t write(int \fIfd\fR, const void *\fIbuf\fR, size_t \fIcount\fR);\fR
.SH ОПИСАНИЕ
Системный вызов \fBwrite\fR() записывает до \fIcount\fR байт из буфера памяти \fIbuf\fR в файл или устройство, ассоциированное с файловым дескриптором \fIfd\fR.
.SH "ВОЗВРАЩАЕМОЕ ЗНАЧЕНИЕ"
При успехе возвращается количество записанных байт. При ошибке возвращается отрицательное значение.
.SH "СМ. ТАКЖЕ"
\fBclose\fR(2), \fBopen\fR(2), \fBread\fR(2)
"#,
    },
    ManPageEntry {
        name: "open",
        section: 2,
        desc_en: "open and possibly create a file",
        desc_ru: "открытие и создание файла",
        roff_en: r#".TH OPEN 2 "2026-10-28" "Wunix" "System Calls"
.SH NAME
open \- open and possibly create a file
.SH SYNOPSIS
\fBint open(const char *\fIpathname\fR, int \fIflags\fR, mode_t \fImode\fR);\fR
.SH DESCRIPTION
The \fBopen\fR() system call opens the file specified by \fIpathname\fR. If the specified file does not exist, it may optionally be created by \fBopen\fR().
.SH "SEE ALSO"
\fBclose\fR(2), \fBread\fR(2), \fBwrite\fR(2)
"#,
        roff_ru: r#".TH OPEN 2 "2026-10-28" "Wunix" "Системные вызовы"
.SH ИМЯ
open \- открытие и создание файла
.SH СИНТАКСИС
\fBint open(const char *\fIpathname\fR, int \fIflags\fR, mode_t \fImode\fR);\fR
.SH ОПИСАНИЕ
Системный вызов \fBopen\fR() открывает файл по указанному пути \fIpathname\fR и возвращает файловый дескриптор.
.SH "СМ. ТАКЖЕ"
\fBclose\fR(2), \fBread\fR(2), \fBwrite\fR(2)
"#,
    },
    // --- СЕКЦИЯ 5: Форматы файлов ---
    ManPageEntry {
        name: "os-release",
        section: 5,
        desc_en: "operating system identification",
        desc_ru: "файл идентификации операционной системы",
        roff_en: r#".TH OS-RELEASE 5 "2026-10-28" "Wunix" "File Formats"
.SH NAME
os-release \- operating system identification
.SH SYNOPSIS
/etc/os-release
.SH DESCRIPTION
The \fB/etc/os-release\fR file contains operating system identification data in newline-separated KEY=VALUE variable format.
.SH "SEE ALSO"
\fBuname\fR(1)
"#,
        roff_ru: r#".TH OS-RELEASE 5 "2026-10-28" "Wunix" "Форматы файлов"
.SH ИМЯ
os-release \- файл идентификации операционной системы
.SH СИНТАКСИС
/etc/os-release
.SH ОПИСАНИЕ
Файл \fB/etc/os-release\fR содержит параметры идентификации операционной системы Wunix в формате переменных КЛЮЧ=ЗНАЧЕНИЕ.
.SH "СМ. ТАКЖЕ"
\fBuname\fR(1)
"#,
    },
    // --- СЕКЦИЯ 7: Архитектура и соглашения ---
    ManPageEntry {
        name: "wunix",
        section: 7,
        desc_en: "overview of the Wunix operating system architecture",
        desc_ru: "обзор архитектуры и подсистем операционной системы Wunix",
        roff_en: r#".TH WUNIX 7 "2026-10-30" "Wunix" "Miscellaneous"
.SH NAME
wunix \- overview of the Wunix operating system architecture
.SH DESCRIPTION
Wunix is a modern 64-bit Unix-like operating system written in Rust, featuring a modular hybrid kernel, VFS (RAMFS, FAT32), VirtIO device drivers (virtio-blk, virtio-net), full networking stack (TCP/UDP/ICMP), and native bilingual localization (Russian and English).
.SH "SEE ALSO"
\fBwsh\fR(1), \fBman\fR(1), \fBinit\fR(8)
"#,
        roff_ru: r#".TH WUNIX 7 "2026-10-30" "Wunix" "Разное"
.SH ИМЯ
wunix \- обзор архитектуры и подсистем операционной системы Wunix
.SH ОПИСАНИЕ
Wunix \- модульная 64-битная Unix-подобная операционная система, написанная на Rust. Включает гибридное ядро, подсистему виртуальной памяти с постраничным распределением, виртуальную файловую систему VFS (RAMFS, FAT32), драйверы устройств VirtIO, сетевой стек и встроенную двуязычную локализацию (русский и английский).
.SH "СМ. ТАКЖЕ"
\fBwsh\fR(1), \fBman\fR(1), \fBinit\fR(8)
"#,
    },
    // --- СЕКЦИЯ 8: Системное администрирование ---
    ManPageEntry {
        name: "fdisk",
        section: 8,
        desc_en: "manipulate disk partition table",
        desc_ru: "управление таблицей разделов диска",
        roff_en: r#".TH FDISK 8 "2026-10-30" "Wunix" "System Administration Commands"
.SH NAME
fdisk \- manipulate disk partition table
.SH SYNOPSIS
\fBfdisk\fR [\fIOPTION\fR] [\fIDEVICE\fR]
.SH DESCRIPTION
\fBfdisk\fR is a dialog-driven program for creation and manipulation of partition tables. It understands MBR and GPT partition tables.
.SH OPTIONS
.TP
\fB\-l\fR, \fB\-\-list\fR
List the partition tables for the specified devices and exit.
.SH "SEE ALSO"
\fBmount\fR(8), \fBumount\fR(8)
"#,
        roff_ru: r#".TH FDISK 8 "2026-10-30" "Wunix" "Команды системного администрирования"
.SH ИМЯ
fdisk \- управление таблицей разделов диска
.SH СИНТАКСИС
\fBfdisk\fR [\fIОПЦИИ\fR] [\fIУСТРОЙСТВО\fR]
.SH ОПИСАНИЕ
\fBfdisk\fR предназначена для просмотра и редактирования таблицы разделов блочных накопителей (MBR и GPT).
.SH ОПЦИИ
.TP
\fB\-l\fR, \fB\-\-list\fR
Вывести таблицу разделов указанного устройства и выйти.
.SH "СМ. ТАКЖЕ"
\fBmount\fR(8), \fBumount\fR(8)
"#,
    },
    ManPageEntry {
        name: "mount",
        section: 8,
        desc_en: "mount a filesystem",
        desc_ru: "монтирование файловой системы",
        roff_en: r#".TH MOUNT 8 "2026-10-30" "Wunix" "System Administration Commands"
.SH NAME
mount \- mount a filesystem
.SH SYNOPSIS
\fBmount\fR [\fB\-t\fR \fIFSTYPE\fR] \fIDEVICE\fR \fIDIR\fR
.SH DESCRIPTION
All files accessible in a Wunix system are arranged in one big tree, the file hierarchy, rooted at /. These files can be spread out over several devices. The \fBmount\fR command serves to attach the filesystem found on some device to the big file tree.
.SH "SEE ALSO"
\fBumount\fR(8), \fBfdisk\fR(8)
"#,
        roff_ru: r#".TH MOUNT 8 "2026-10-30" "Wunix" "Команды системного администрирования"
.SH ИМЯ
mount \- монтирование файловой системы
.SH СИНТАКСИС
\fBmount\fR [\fB\-t\fR \fIТИП_ФС\fR] \fIУСТРОЙСТВО\fR \fIКАТАЛОГ\fR
.SH ОПИСАНИЕ
Подключает файловую систему указанного накопителя к общему дереву каталогов Wunix в точку монтирования \fIКАТАЛОГ\fR.
.SH "СМ. ТАКЖЕ"
\fBumount\fR(8), \fBfdisk\fR(8)
"#,
    },
    ManPageEntry {
        name: "clear",
        section: 1,
        desc_en: "clear the terminal screen",
        desc_ru: "очистка экрана терминала",
        roff_en: r#".TH CLEAR 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
clear \- clear the terminal screen
.SH SYNOPSIS
\fBclear\fR
.SH DESCRIPTION
\fBclear\fR clears your terminal screen and places the cursor at the top left corner (row 1, column 1) using standard ANSI escape sequences (\fB\e[2J\e[H\fR).
.SH "SEE ALSO"
\fBwsh\fR(1)
"#,
        roff_ru: r#".TH CLEAR 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
clear \- очистка экрана терминала
.SH СИНТАКСИС
\fBclear\fR
.SH ОПИСАНИЕ
\fBclear\fR полностью очищает экран терминала консоли и переводит курсор в верхний левый угол (строка 1, столбец 1) с помощью управляющих escape-последовательностей ANSI (\fB\e[2J\e[H\fR).
.SH "СМ. ТАКЖЕ"
\fBwsh\fR(1)
"#,
    },
    ManPageEntry {
        name: "curl",
        section: 1,
        desc_en: "transfer a URL using HTTP",
        desc_ru: "передача данных по сети по протоколу HTTP",
        roff_en: r#".TH CURL 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
curl \- transfer a URL using HTTP
.SH SYNOPSIS
\fBcurl\fR [\fIOPTIONS\fR]... \fIURL\fR
.SH DESCRIPTION
\fBcurl\fR is a tool for transferring data from or to a server using HTTP (GET, POST, HEAD, PUT, DELETE).
.SH OPTIONS
.TP
\fB\-o\fR, \fB\-\-output\fR \fIFILE\fR
Write output to FILE instead of stdout.
.TP
\fB\-O\fR, \fB\-\-remote\-name\fR
Write output to a local file named like the remote file we get.
.TP
\fB\-s\fR, \fB\-\-silent\fR
Silent mode. Do not print progress or error messages.
.TP
\fB\-S\fR, \fB\-\-show\-error\fR
When used with \fB\-s\fR, make curl show an error message if it fails.
.TP
\fB\-i\fR, \fB\-\-include\fR
Include HTTP response headers in the output.
.TP
\fB\-I\fR, \fB\-\-head\fR
Fetch the HTTP headers only (HEAD method).
.TP
\fB\-v\fR, \fB\-\-verbose\fR
Make curl verbose during connection and transfer.
.TP
\fB\-X\fR, \fB\-\-request\fR \fIMETHOD\fR
Specify custom HTTP request method to use (e.g. GET, POST, HEAD).
.TP
\fB\-d\fR, \fB\-\-data\fR \fIDATA\fR
Send specified data in an HTTP POST request.
.TP
\fB\-H\fR, \fB\-\-header\fR \fIHEADER\fR
Pass custom HTTP header to server.
.TP
\fB\-h\fR, \fB\-\-help\fR
Show help and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Display version information and exit.
.SH "SEE ALSO"
\fBping\fR(8), \fBip\fR(8)
"#,
        roff_ru: r#".TH CURL 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
curl \- передача данных по сети по протоколу HTTP
.SH СИНТАКСИС
\fBcurl\fR [\fIОПЦИИ\fR]... \fIURL\fR
.SH ОПИСАНИЕ
\fBcurl\fR — клиент для загрузки и отправки данных по протоколу HTTP (поддерживает методы GET, POST, HEAD, PUT, DELETE).
.SH ОПЦИИ
.TP
\fB\-o\fR, \fB\-\-output\fR \fIФАЙЛ\fR
Сохранить ответ сервера в указанный файл вместо стандартного вывода.
.TP
\fB\-O\fR, \fB\-\-remote\-name\fR
Сохранить файл под именем из запрашиваемого URL.
.TP
\fB\-s\fR, \fB\-\-silent\fR
Тихий режим. Не выводить индикаторы и сообщения.
.TP
\fB\-S\fR, \fB\-\-show\-error\fR
При использовании с \fB\-s\fR выводить ошибки при сбоях сети.
.TP
\fB\-i\fR, \fB\-\-include\fR
Выводить HTTP-заголовки ответа сервера вместе с телом.
.TP
\fB\-I\fR, \fB\-\-head\fR
Запрашивать только HTTP-заголовки ответа (метод HEAD).
.TP
\fB\-v\fR, \fB\-\-verbose\fR
Подробный диагностический лог сетевого подключения.
.TP
\fB\-X\fR, \fB\-\-request\fR \fIМЕТОД\fR
Указать HTTP-метод запроса (GET, POST, HEAD, PUT, DELETE).
.TP
\fB\-d\fR, \fB\-\-data\fR \fIДАННЫЕ\fR
Данные для отправки в теле HTTP POST запроса.
.TP
\fB\-H\fR, \fB\-\-header\fR \fIЗАГОЛОВОК\fR
Передать дополнительный HTTP-заголовок серверу.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать справку по опциям и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести информацию о версии и выйти.
.SH "СМ. ТАКЖЕ"
\fBping\fR(8), \fBip\fR(8)
"#,
    },
    ManPageEntry {
        name: "echo",
        section: 1,
        desc_en: "display a line of text",
        desc_ru: "вывод строки текста в стандартный вывод",
        roff_en: r#".TH ECHO 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
echo \- display a line of text
.SH SYNOPSIS
\fBecho\fR [\fISTRING\fR]...
.SH DESCRIPTION
Echo the STRING(s) to standard output separated by single spaces and terminated by a newline.
.SH "SEE ALSO"
\fBcat\fR(1), \fBwsh\fR(1)
"#,
        roff_ru: r#".TH ECHO 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
echo \- вывод строки текста в стандартный вывод
.SH СИНТАКСИС
\fBecho\fR [\fIСТРОКА\fR]...
.SH ОПИСАНИЕ
Выводит переданные аргументы СТРОКА в стандартный вывод, разделяя их одиночными пробелами и завершая символом перевода строки.
.SH "СМ. ТАКЖЕ"
\fBcat\fR(1), \fBwsh\fR(1)
"#,
    },
    ManPageEntry {
        name: "fbset",
        section: 1,
        desc_en: "show and modify frame buffer device settings",
        desc_ru: "управление кадровым буфером и видеорежимами дисплея",
        roff_en: r#".TH FBSET 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
fbset \- show and modify frame buffer device settings
.SH SYNOPSIS
\fBfbset\fR [\fIOPTIONS\fR]...
.SH DESCRIPTION
\fBfbset\fR is a system utility to show or change the settings of the frame buffer device (resolution, depth, pitch, and pixel format).
.SH OPTIONS
.TP
\fB\-i\fR, \fB\-\-info\fR
Display detailed information about the frame buffer device.
.TP
\fB\-x\fR \fIwidth\fR
Set the horizontal resolution.
.TP
\fB\-y\fR \fIheight\fR
Set the vertical resolution.
.TP
\fB\-depth\fR \fIbits\fR
Set color depth (16, 24, 32).
.TP
\fB\-h\fR, \fB\-\-help\fR
Show help and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Show version information.
.SH "SEE ALSO"
\fBlshw\fR(1)
"#,
        roff_ru: r#".TH FBSET 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
fbset \- управление кадровым буфером и видеорежимами дисплея
.SH СИНТАКСИС
\fBfbset\fR [\fIОПЦИИ\fR]...
.SH ОПИСАНИЕ
\fBfbset\fR \- утилита для настройки и отображения параметров графического кадрового буфера дисплея: разрешения, глубины цвета (bpp), шага строки и формата пикселей.
.SH ОПЦИИ
.TP
\fB\-i\fR, \fB\-\-info\fR
Подробный вывод характеристик кадрового буфера (размер видеобуфера в байтах, формат TrueColor).
.TP
\fB\-x\fR \fIширина\fR
Установка ширины разрешения экрана.
.TP
\fB\-y\fR \fIвысота\fR
Установка высоты разрешения экрана.
.TP
\fB\-depth\fR \fIбит\fR
Установка битовой глубины цвета (16, 24, 32).
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести версию утилиты.
.SH "СМ. ТАКЖЕ"
\fBlshw\fR(1)
"#,
    },
    ManPageEntry {
        name: "free",
        section: 1,
        desc_en: "display amount of free and used memory in the system",
        desc_ru: "отображение объема свободной и используемой памяти",
        roff_en: r#".TH FREE 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
free \- display amount of free and used memory in the system
.SH SYNOPSIS
\fBfree\fR
.SH DESCRIPTION
\fBfree\fR displays the total amount of free and used physical memory in the system, as well as the buffers and caches used by the kernel, queried via the sysinfo system call.
.SH "SEE ALSO"
\fBtop\fR(1)
"#,
        roff_ru: r#".TH FREE 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
free \- отображение объема свободной и используемой памяти
.SH СИНТАКСИС
\fBfree\fR
.SH ОПИСАНИЕ
\fBfree\fR выводит информацию об общем, используемом и свободном объеме физической оперативной памяти (RAM), а также размере буферов и кэшей ядра, запрашивая данные через системный вызов sysinfo.
.SH "СМ. ТАКЖЕ"
\fBtop\fR(1)
"#,
    },
    ManPageEntry {
        name: "hwclock",
        section: 1,
        desc_en: "query or set the hardware clock (RTC)",
        desc_ru: "управление аппаратными часами CMOS RTC",
        roff_en: r#".TH HWCLOCK 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
hwclock \- query or set the hardware clock (RTC)
.SH SYNOPSIS
\fBhwclock\fR [\fIOPTIONS\fR]...
.SH DESCRIPTION
\fBhwclock\fR is an administration tool for the hardware clock, also known as the Real-Time Clock (RTC) or CMOS clock.
.SH OPTIONS
.TP
\fB\-r\fR, \fB\-\-show\fR
Read the hardware clock and print the time.
.TP
\fB\-s\fR, \fB\-\-hctosys\fR
Set the system time from the hardware clock.
.TP
\fB\-w\fR, \fB\-\-systohc\fR
Set the hardware clock from the current system time.
.TP
\fB\-u\fR, \fB\-\-utc\fR
The hardware clock is kept in UTC.
.TP
\fB\-l\fR, \fB\-\-localtime\fR
The hardware clock is kept in local time.
.TP
\fB\-v\fR, \fB\-\-verbose\fR
Display verbose details about RTC CMOS operations.
.TP
\fB\-h\fR, \fB\-\-help\fR
Show help and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Show version information.
.SH "SEE ALSO"
\fBdate\fR(1)
"#,
        roff_ru: r#".TH HWCLOCK 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
hwclock \- управление аппаратными часами CMOS RTC
.SH СИНТАКСИС
\fBhwclock\fR [\fIОПЦИИ\fR]...
.SH ОПИСАНИЕ
\fBhwclock\fR опрашивает и синхронизирует энергонезависимые аппаратные часы материнской платы (CMOS RTC, Motorola MC146818).
.SH ОПЦИИ
.TP
\fB\-r\fR, \fB\-\-show\fR
Считать и вывести текущее время аппаратных часов RTC.
.TP
\fB\-s\fR, \fB\-\-hctosys\fR
Установить системное время ядра из аппаратных часов RTC.
.TP
\fB\-w\fR, \fB\-\-systohc\fR
Установить аппаратные часы RTC из системного времени ядра.
.TP
\fB\-u\fR, \fB\-\-utc\fR
Аппаратные часы настроены на всемирное координированное время (UTC).
.TP
\fB\-l\fR, \fB\-\-localtime\fR
Аппаратные часы настроены на локальное местное время.
.TP
\fB\-v\fR, \fB\-\-verbose\fR
Подробный вывод состояния регистров CMOS.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести информацию о версии.
.SH "СМ. ТАКЖЕ"
\fBdate\fR(1)
"#,
    },
    ManPageEntry {
        name: "lsdev",
        section: 1,
        desc_en: "display hardware devices, drivers and resources",
        desc_ru: "сводка системных устройств и аппаратных ресурсов",
        roff_en: r#".TH LSDEV 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
lsdev \- display hardware devices, drivers and resources
.SH SYNOPSIS
\fBlsdev\fR [\fIOPTIONS\fR]...
.SH DESCRIPTION
\fBlsdev\fR displays system hardware devices, associated kernel device drivers, assigned interrupts (IRQs), I/O port spaces, and MMIO memory regions.
.SH OPTIONS
.TP
\fB\-\-irq\fR
Show only interrupt requests (IRQ).
.TP
\fB\-\-io\fR
Show only I/O port reservations.
.TP
\fB\-\-mmio\fR
Show only memory mapped I/O (MMIO) ranges.
.TP
\fB\-\-drivers\fR
Show only device-to-driver bindings.
.TP
\fB\-h\fR, \fB\-\-help\fR
Show help and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Show version information.
.SH "SEE ALSO"
\fBlspci\fR(1), \fBlshw\fR(1), \fBlsmod\fR(1)
"#,
        roff_ru: r#".TH LSDEV 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
lsdev \- сводка системных устройств и аппаратных ресурсов
.SH СИНТАКСИС
\fBlsdev\fR [\fIОПЦИИ\fR]...
.SH ОПИСАНИЕ
\fBlsdev\fR отображает распределение ресурсов компьютера: линий прерываний (IRQ), портов ввода-вывода (I/O) и диапазонов памяти MMIO, а также состояние привязанных драйверов.
.SH ОПЦИИ
.TP
\fB\-\-irq\fR
Показать только прерывания IRQ.
.TP
\fB\-\-io\fR
Показать только порты ввода-вывода.
.TP
\fB\-\-mmio\fR
Показать только адреса памяти MMIO.
.TP
\fB\-\-drivers\fR
Показать привязку драйверов ядра к устройствам.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести версию программы.
.SH "СМ. ТАКЖЕ"
\fBlspci\fR(1), \fBlshw\fR(1), \fBlsmod\fR(1)
"#,
    },
    ManPageEntry {
        name: "lshw",
        section: 1,
        desc_en: "list hardware configuration",
        desc_ru: "сводная инвентаризация аппаратного обеспечения",
        roff_en: r#".TH LSHW 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
lshw \- list hardware configuration
.SH SYNOPSIS
\fBlshw\fR [\fIOPTIONS\fR]...
.SH DESCRIPTION
\fBlshw\fR is a comprehensive tool providing detailed information on the hardware configuration of the machine.
.SH OPTIONS
.TP
\fB\-short\fR
Output hardware paths in a compact table.
.TP
\fB\-class\fR \fIclass\fR
Only show the given class of hardware (cpu, memory, disk, network, display, bus).
.TP
\fB\-json\fR
Output hardware tree as a JSON structure.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display help and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Display version information.
.SH "SEE ALSO"
\fBlspci\fR(1), \fBlsdev\fR(1)
"#,
        roff_ru: r#".TH LSHW 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
lshw \- сводная инвентаризация аппаратного обеспечения
.SH СИНТАКСИС
\fBlshw\fR [\fIОПЦИИ\fR]...
.SH ОПИСАНИЕ
\fBlshw\fR \- утилита для формирования подробного каталога оборудования: процессора, оперативной памяти, накопителей, видеокарты, сетевых интерфейсов и шин.
.SH ОПЦИИ
.TP
\fB\-short\fR
Вывести аппаратные компоненты в виде компактной таблицы.
.TP
\fB\-class\fR \fIкласс\fR
Фильтровать компоненты по классу (cpu, memory, disk, network, display, bus).
.TP
\fB\-json\fR
Вывести структуру оборудования в формате JSON.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести версию программы.
.SH "СМ. ТАКЖЕ"
\fBlspci\fR(1), \fBlsdev\fR(1)
"#,
    },
    ManPageEntry {
        name: "lsmod",
        section: 1,
        desc_en: "show the status of modules in the kernel",
        desc_ru: "список загруженных драйверов и модулей ядра",
        roff_en: r#".TH LSMOD 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
lsmod \- show the status of modules in the kernel
.SH SYNOPSIS
\fBlsmod\fR [\fIOPTIONS\fR]...
.SH DESCRIPTION
\fBlsmod\fR is a trivial program which nicely formats the contents of the \fI/proc/modules\fR file, showing which kernel drivers and modules are currently loaded.
.SH OPTIONS
.TP
\fB\-s\fR, \fB\-\-sort\fR
Sort modules by size in descending order.
.TP
\fB\-h\fR, \fB\-\-help\fR
Show help and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Show version information.
.SH "SEE ALSO"
\fBmodinfo\fR(1), \fBlsdev\fR(1)
"#,
        roff_ru: r#".TH LSMOD 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
lsmod \- список загруженных драйверов и модулей ядра
.SH СИНТАКСИС
\fBlsmod\fR [\fIОПЦИИ\fR]...
.SH ОПИСАНИЕ
\fBlsmod\fR считывает информацию из \fI/proc/modules\fR и выводит форматированную таблицу активных драйверов устройств ядра Wunix, их размер и зависимости.
.SH ОПЦИИ
.TP
\fB\-s\fR, \fB\-\-sort\fR
Сортировать модули по размеру памяти.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести информацию о версии.
.SH "СМ. ТАКЖЕ"
\fBmodinfo\fR(1), \fBlsdev\fR(1)
"#,
    },
    ManPageEntry {
        name: "lspci",
        section: 1,
        desc_en: "list all PCI devices",
        desc_ru: "вывод списка всех устройств на шине PCI",
        roff_en: r#".TH LSPCI 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
lspci \- list all PCI devices
.SH SYNOPSIS
\fBlspci\fR [\fIOPTIONS\fR]...
.SH DESCRIPTION
\fBlspci\fR is a utility for displaying information about PCI buses in the system and devices connected to them.
.SH OPTIONS
.TP
\fB\-n\fR
Show numeric vendor and device IDs (vendor:device).
.TP
\fB\-v\fR
Be verbose and display detailed information about BAR addresses, sizes, and IRQ lines.
.TP
\fB\-vv\fR
Be very verbose and display active kernel drivers in use.
.TP
\fB\-t\fR
Show a tree-like diagram containing all buses, bridges, devices and connections between them.
.TP
\fB\-s\fR [[\fIbus\fR]:]\fIslot\fR[.\fIfunc\fR]
Show only devices in the specified bus and slot.
.TP
\fB\-h\fR, \fB\-\-help\fR
Show help and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Show version information.
.SH "SEE ALSO"
\fBlshw\fR(1), \fBlsdev\fR(1)
"#,
        roff_ru: r#".TH LSPCI 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
lspci \- вывод списка всех устройств на шине PCI
.SH СИНТАКСИС
\fBlspci\fR [\fIОПЦИИ\fR]...
.SH ОПИСАНИЕ
\fBlspci\fR \- утилита для отображения сведений о шинах PCI, мостах, хост-адаптерах и подключенных к ним периферийных устройствах.
.SH ОПЦИИ
.TP
\fB\-n\fR
Выводить числовые коды производителей и устройств (vendor:device).
.TP
\fB\-v\fR
Подробный вывод: базовые адреса BAR, диапазоны памяти MMIO, порты ввода-вывода и номера прерываний IRQ.
.TP
\fB\-vv\fR
Расширенный подробный вывод: привязанные драйверы ядра Wunix.
.TP
\fB\-t\fR
Древовидная топология шины PCI.
.TP
\fB\-s\fR [[\fIшина\fR]:]\fIслот\fR[.\fIфункция\fR]
Фильтрация устройств по адресу на шине.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести версию утилиты.
.SH "СМ. ТАКЖЕ"
\fBlshw\fR(1), \fBlsdev\fR(1)
"#,
    },
    ManPageEntry {
        name: "modinfo",
        section: 1,
        desc_en: "show information about a kernel driver or module",
        desc_ru: "вывод метаданных драйверов ядра",
        roff_en: r#".TH MODINFO 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
modinfo \- show information about a kernel driver or module
.SH SYNOPSIS
\fBmodinfo\fR [\fIOPTIONS\fR] \fImodule_name\fR
.SH DESCRIPTION
\fBmodinfo\fR extracts and displays detailed information about a kernel device driver, including its author, description, license, supported hardware aliases, and module parameters.
.SH OPTIONS
.TP
\fB\-F\fR \fIfield\fR
Only print the value of the specified field.
.TP
\fB\-d\fR, \fB\-\-description\fR
Print only the module description.
.TP
\fB\-a\fR, \fB\-\-author\fR
Print only the module author.
.TP
\fB\-l\fR, \fB\-\-license\fR
Print only the module license.
.TP
\fB\-n\fR, \fB\-\-filename\fR
Print only the module source filename.
.TP
\fB\-p\fR, \fB\-\-parameters\fR
Print only module configuration parameters.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display help and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Show version information.
.SH "SEE ALSO"
\fBlsmod\fR(1)
"#,
        roff_ru: r#".TH MODINFO 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
modinfo \- вывод метаданных драйверов ядра
.SH СИНТАКСИС
\fBmodinfo\fR [\fIОПЦИИ\fR] \fIимя_модуля\fR
.SH ОПИСАНИЕ
\fBmodinfo\fR отображает детальные сведения о драйвере ядра: автора, описание, лицензию, идентификаторы поддерживаемых устройств PCI и параметры настройки.
.SH ОПЦИИ
.TP
\fB\-F\fR \fIполе\fR
Вывести только значение указанного поля.
.TP
\fB\-d\fR, \fB\-\-description\fR
Вывести только описание драйвера.
.TP
\fB\-a\fR, \fB\-\-author\fR
Вывести только автора драйвера.
.TP
\fB\-l\fR, \fB\-\-license\fR
Вывести только лицензию.
.TP
\fB\-n\fR, \fB\-\-filename\fR
Вывести только путь к файлу драйвера.
.TP
\fB\-p\fR, \fB\-\-parameters\fR
Вывести только настраиваемые параметры.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести информацию о версии.
.SH "СМ. ТАКЖЕ"
\fBlsmod\fR(1)
"#,
    },
    ManPageEntry {
        name: "pwd",
        section: 1,
        desc_en: "print name of current/working directory",
        desc_ru: "вывод пути к текущему рабочему каталогу",
        roff_en: r#".TH PWD 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
pwd \- print name of current/working directory
.SH SYNOPSIS
\fBpwd\fR
.SH DESCRIPTION
Print the full filename of the current working directory, obtained via the getcwd system call.
.SH "SEE ALSO"
\fBwsh\fR(1), \fBls\fR(1)
"#,
        roff_ru: r#".TH PWD 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
pwd \- вывод пути к текущему рабочему каталогу
.SH СИНТАКСИС
\fBpwd\fR
.SH ОПИСАНИЕ
Выводит абсолютный путь к текущей рабочей директории процесса, полученный от ядра через системный вызов getcwd.
.SH "СМ. ТАКЖЕ"
\fBwsh\fR(1), \fBls\fR(1)
"#,
    },
    ManPageEntry {
        name: "rm",
        section: 1,
        desc_en: "remove files or directories",
        desc_ru: "удаление файлов и каталогов",
        roff_en: r#".TH RM 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
rm \- remove files or directories
.SH SYNOPSIS
\fBrm\fR [\fIOPTION\fR]... \fIFILE\fR...
.SH DESCRIPTION
\fBrm\fR removes each specified file via the unlink system call.
.SH OPTIONS
.TP
\fB\-f\fR, \fB\-\-force\fR
Ignore nonexistent files and arguments, never prompt.
.TP
\fB\-r\fR, \fB\-R\fR, \fB\-\-recursive\fR
Remove directories and their contents recursively.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.SH "SEE ALSO"
\fBtouch\fR(1), \fBls\fR(1)
"#,
        roff_ru: r#".TH RM 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
rm \- удаление файлов и каталогов
.SH СИНТАКСИС
\fBrm\fR [\fIОПЦИИ\fR]... \fIФАЙЛ\fR...
.SH ОПИСАНИЕ
\fBrm\fR удаляет указанные файлы и каталоги из файловой системы через системный вызов unlink.
.SH ОПЦИИ
.TP
\fB\-f\fR, \fB\-\-force\fR
Принудительный режим: игнорировать отсутствующие файлы без вывода ошибок.
.TP
\fB\-r\fR, \fB\-R\fR, \fB\-\-recursive\fR
Рекурсивное удаление каталогов и их содержимого.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH "СМ. ТАКЖЕ"
\fBtouch\fR(1), \fBls\fR(1)
"#,
    },
    ManPageEntry {
        name: "top",
        section: 1,
        desc_en: "display system processes and resource usage",
        desc_ru: "мониторинг запущенных процессов и системных ресурсов",
        roff_en: r#".TH TOP 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
top \- display system processes and resource usage
.SH SYNOPSIS
\fBtop\fR
.SH DESCRIPTION
\fBtop\fR provides an ongoing look at processor activity in real time. It displays a system summary (uptime, process count, memory usage) and a list of processes currently managed by the kernel.
.SH "SEE ALSO"
\fBfree\fR(1)
"#,
        roff_ru: r#".TH TOP 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
top \- мониторинг запущенных процессов и системных ресурсов
.SH СИНТАКСИС
\fBtop\fR
.SH ОПИСАНИЕ
\fBtop\fR выводит в реальном времени системную сводку операционной системы Wunix: время непрерывной работы (uptime), количество активных задач, нагрузку и список выполняющихся процессов с их PID и статусом.
.SH "СМ. ТАКЖЕ"
\fBfree\fR(1)
"#,
    },
    ManPageEntry {
        name: "touch",
        section: 1,
        desc_en: "change file timestamps or create empty files",
        desc_ru: "создание пустых файлов и обновление времени модификации",
        roff_en: r#".TH TOUCH 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
touch \- change file timestamps or create empty files
.SH SYNOPSIS
\fBtouch\fR \fIFILE\fR...
.SH DESCRIPTION
Update the access and modification times of each FILE to the current time. A FILE argument that does not exist is created empty.
.SH "SEE ALSO"
\fBrm\fR(1), \fBls\fR(1)
"#,
        roff_ru: r#".TH TOUCH 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
touch \- создание пустых файлов и обновление времени модификации
.SH СИНТАКСИС
\fBtouch\fR \fIФАЙЛ\fR...
.SH ОПИСАНИЕ
Создает пустой файл по указанному пути (с флагом O_CREAT), либо обновляет время последнего доступа и модификации, если файл уже существует.
.SH "СМ. ТАКЖЕ"
\fBrm\fR(1), \fBls\fR(1)
"#,
    },
    ManPageEntry {
        name: "init",
        section: 8,
        desc_en: "system initialization process and service supervisor (PID 1)",
        desc_ru: "системный супервизор и первый процесс инициализации (PID 1)",
        roff_en: r#".TH INIT 8 "2026-10-30" "Wunix" "System Administration Commands"
.SH NAME
init \- system initialization process and service supervisor (PID 1)
.SH SYNOPSIS
\fBinit\fR
.SH DESCRIPTION
\fBinit\fR is the first user-space program executed by the Wunix kernel during system boot (PID 1). It initializes user environment, changes current directory to /home/user, launches the interactive command shell /bin/wsh, and reaps orphaned child processes.
.SH "SEE ALSO"
\fBwsh\fR(1), \fBwunix\fR(7)
"#,
        roff_ru: r#".TH INIT 8 "2026-10-30" "Wunix" "Команды системного администрирования"
.SH ИМЯ
init \- системный супервизор и первый процесс инициализации (PID 1)
.SH СИНТАКСИС
\fBinit\fR
.SH ОПИСАНИЕ
\fBinit\fR — главный процесс пространства пользователя (PID 1), запускаемый ядром Wunix после завершения ранней загрузки. Он подготавливает базовое пользовательское окружение, переходит в домашний каталог /home/user, запускает интерактивный командный шелл /bin/wsh и отслеживает состояние дочерних процессов.
.SH "СМ. ТАКЖЕ"
\fBwsh\fR(1), \fBwunix\fR(7)
"#,
    },
    ManPageEntry {
        name: "ip",
        section: 8,
        desc_en: "show or manipulate routing, devices and interfaces",
        desc_ru: "просмотр и настройка сетевых интерфейсов и маршрутизации",
        roff_en: r#".TH IP 8 "2026-10-30" "Wunix" "System Administration Commands"
.SH NAME
ip \- show or manipulate routing, devices and interfaces
.SH SYNOPSIS
\fBip\fR [\fIOPTIONS\fR] \fIOBJECT\fR [\fICOMMAND\fR]
.SH DESCRIPTION
\fBip\fR is a network administration tool for inspecting network interfaces, link layer devices, IP addresses, and routing tables.
.SH OBJECTS
.TP
\fBaddr\fR, \fBa\fR
Protocol (IP or IPv6) address on a device.
.TP
\fBlink\fR, \fBl\fR
Network device (Ethernet, loopback, MAC address, MTU).
.TP
\fBroute\fR, \fBr\fR
Routing table entry.
.SH OPTIONS
.TP
\fB\-br\fR, \fB\-brief\fR
Print only summary information in one-line tabular format.
.TP
\fB\-h\fR, \fB\-\-help\fR
Show help and exit.
.SH "SEE ALSO"
\fBping\fR(8), \fBss\fR(8), \fBcurl\fR(1)
"#,
        roff_ru: r#".TH IP 8 "2026-10-30" "Wunix" "Команды системного администрирования"
.SH ИМЯ
ip \- просмотр и настройка сетевых интерфейсов и маршрутизации
.SH СИНТАКСИС
\fBip\fR [\fIОПЦИИ\fR] \fIОБЪЕКТ\fR [\fIКОМАНДА\fR]
.SH ОПИСАНИЕ
\fBip\fR — утилита администрирования сетевой подсистемы для просмотра сетевых интерфейсов, MAC-адресов, назначенных IPv4 адресов и таблицы маршрутизации.
.SH ОБЪЕКТЫ
.TP
\fBaddr\fR, \fBa\fR
Сетевые IPv4/IPv6 адреса интерфейсов.
.TP
\fBlink\fR, \fBl\fR
Сетевые устройства канального уровня (Ethernet, loopback, MAC, MTU).
.TP
\fBroute\fR, \fBr\fR
Таблица маршрутизации пакетов.
.SH ОПЦИИ
.TP
\fB\-br\fR, \fB\-brief\fR
Краткий однострочный табличный вывод.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать справку и выйти.
.SH "СМ. ТАКЖЕ"
\fBping\fR(8), \fBss\fR(8), \fBcurl\fR(1)
"#,
    },
    ManPageEntry {
        name: "lsblk",
        section: 8,
        desc_en: "list block devices",
        desc_ru: "вывод списка блочных устройств хранения данных",
        roff_en: r#".TH LSBLK 8 "2026-10-30" "Wunix" "System Administration Commands"
.SH NAME
lsblk \- list block devices
.SH SYNOPSIS
\fBlsblk\fR [\fIOPTIONS\fR]...
.SH DESCRIPTION
\fBlsblk\fR lists information about all available block devices (hard drives, partitions, ramdisks, NVMe, virtio-blk).
.SH OPTIONS
.TP
\fB\-a\fR, \fB\-\-all\fR
List all devices, including empty ones.
.TP
\fB\-b\fR, \fB\-\-bytes\fR
Print the SIZE column in bytes rather than in a human-readable format.
.TP
\fB\-l\fR, \fB\-\-list\fR
Produce output in the form of a flat list rather than a tree.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display help and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Display version information.
.SH "SEE ALSO"
\fBfdisk\fR(8), \fBmount\fR(8)
"#,
        roff_ru: r#".TH LSBLK 8 "2026-10-30" "Wunix" "Команды системного администрирования"
.SH ИМЯ
lsblk \- вывод списка блочных устройств хранения данных
.SH СИНТАКСИС
\fBlsblk\fR [\fIОПЦИИ\fR]...
.SH ОПИСАНИЕ
\fBlsblk\fR опрашивает и выводит список всех обнаруженных ядром блочных устройств (жесткие диски ATA/AHCI, разделы MBR/GPT, ramdisk, virtio-blk).
.SH ОПЦИИ
.TP
\fB\-a\fR, \fB\-\-all\fR
Отображать все блочные устройства, включая пустые накопители.
.TP
\fB\-b\fR, \fB\-\-bytes\fR
Выводить размер устройства точно в байтах вместо КБ/МБ/ГБ.
.TP
\fB\-l\fR, \fB\-\-list\fR
Выводить данные в виде плоского списка без иерархического дерева.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести версию утилиты.
.SH "СМ. ТАКЖЕ"
\fBfdisk\fR(8), \fBmount\fR(8)
"#,
    },
    ManPageEntry {
        name: "ping",
        section: 8,
        desc_en: "send ICMP ECHO_REQUEST to network hosts",
        desc_ru: "проверка доступности сетевых узлов через пакеты ICMP Echo",
        roff_en: r#".TH PING 8 "2026-10-30" "Wunix" "System Administration Commands"
.SH NAME
ping \- send ICMP ECHO_REQUEST to network hosts
.SH SYNOPSIS
\fBping\fR [\fB\-c\fR \fICOUNT\fR] [\fB\-s\fR \fISIZE\fR] [\fB\-W\fR \fITIMEOUT\fR] [\fB\-4\fR] \fIDESTINATION\fR
.SH DESCRIPTION
\fBping\fR uses the ICMP protocol's mandatory ECHO_REQUEST datagram to elicit an ICMP ECHO_RESPONSE from a host or gateway.
.SH OPTIONS
.TP
\fB\-c\fR \fICOUNT\fR
Stop after sending COUNT ECHO_REQUEST packets.
.TP
\fB\-s\fR \fISIZE\fR
Specify the number of data bytes to be sent. Default is 56 (64 with ICMP header).
.TP
\fB\-W\fR \fITIMEOUT\fR
Time to wait for a response, in seconds. Default is 1 second.
.TP
\fB\-4\fR
Use IPv4 only.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display help and exit.
.SH "SEE ALSO"
\fBip\fR(8), \fBcurl\fR(1)
"#,
        roff_ru: r#".TH PING 8 "2026-10-30" "Wunix" "Команды системного администрирования"
.SH ИМЯ
ping \- проверка доступности сетевых узлов через пакеты ICMP Echo
.SH СИНТАКСИС
\fBping\fR [\fB\-c\fR \fIЧИСЛО\fR] [\fB\-s\fR \fIРАЗМЕР\fR] [\fB\-W\fR \fIТАЙМАУТ\fR] [\fB\-4\fR] \fIХОСТ\fR
.SH ОПИСАНИЕ
\fBping\fR отправляет сетевые пакеты ICMP ECHO_REQUEST удаленному хосту и замеряет время круговой задержки (RTT).
.SH ОПЦИИ
.TP
\fB\-c\fR \fIЧИСЛО\fR
Количество отправляемых эхо-запросов.
.TP
\fB\-s\fR \fIРАЗМЕР\fR
Размер буфера полезных данных в байтах (по умолчанию 56 байт).
.TP
\fB\-W\fR \fIТАЙМАУТ\fR
Время ожидания ответа в секундах (по умолчанию 1 сек).
.TP
\fB\-4\fR
Принудительно использовать IPv4.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать справку и выйти.
.SH "СМ. ТАКЖЕ"
\fBip\fR(8), \fBcurl\fR(1)
"#,
    },
    ManPageEntry {
        name: "ss",
        section: 8,
        desc_en: "another utility to investigate sockets",
        desc_ru: "исследование сетевых сокетов и открытых портов",
        roff_en: r#".TH SS 8 "2026-10-30" "Wunix" "System Administration Commands"
.SH NAME
ss \- another utility to investigate sockets
.SH SYNOPSIS
\fBss\fR [\fIOPTIONS\fR]...
.SH DESCRIPTION
\fBss\fR is used to dump socket statistics. It reads network state from /proc/net/tcp and /proc/net/udp.
.SH OPTIONS
.TP
\fB\-t\fR, \fB\-\-tcp\fR
Display TCP sockets.
.TP
\fB\-u\fR, \fB\-\-udp\fR
Display UDP sockets.
.TP
\fB\-l\fR, \fB\-\-listening\fR
Display only listening sockets.
.TP
\fB\-a\fR, \fB\-\-all\fR
Display both listening and non-listening sockets.
.TP
\fB\-n\fR, \fB\-\-numeric\fR
Do not try to resolve service names.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display help and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Display version information.
.SH "SEE ALSO"
\fBip\fR(8), \fBping\fR(8)
"#,
        roff_ru: r#".TH SS 8 "2026-10-30" "Wunix" "Команды системного администрирования"
.SH ИМЯ
ss \- исследование сетевых сокетов и открытых портов
.SH СИНТАКСИС
\fBss\fR [\fIОПЦИИ\fR]...
.SH ОПИСАНИЕ
\fBss\fR анализирует состояние сетевого стека Wunix, считывая сокеты TCP и UDP из виртуальной файловой системы /proc/net/.
.SH ОПЦИИ
.TP
\fB\-t\fR, \fB\-\-tcp\fR
Показывать только сокеты протокола TCP.
.TP
\fB\-u\fR, \fB\-\-udp\fR
Показывать только сокеты протокола UDP.
.TP
\fB\-l\fR, \fB\-\-listening\fR
Отображать исключительно слушающие сокеты (LISTEN).
.TP
\fB\-a\fR, \fB\-\-all\fR
Показывать все соединения (слушающие и установленные).
.TP
\fB\-n\fR, \fB\-\-numeric\fR
Выводить номера портов числами без преобразования в имена служб.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести версию утилиты.
.SH "СМ. ТАКЖЕ"
\fBip\fR(8), \fBping\fR(8)
"#,
    },
    ManPageEntry {
        name: "umount",
        section: 8,
        desc_en: "unmount file systems",
        desc_ru: "отключение смонтированных файловых систем",
        roff_en: r#".TH UMOUNT 8 "2026-10-30" "Wunix" "System Administration Commands"
.SH NAME
umount \- unmount file systems
.SH SYNOPSIS
\fBumount\fR [\fIOPTIONS\fR] \fIDIRECTORY\fR|\fIDEVICE\fR
.SH DESCRIPTION
\fBumount\fR detaches the file system(s) mentioned from the file hierarchy.
.SH OPTIONS
.TP
\fB\-f\fR, \fB\-\-force\fR
Force an unmount (in case of an unreachable system).
.TP
\fB\-v\fR, \fB\-\-verbose\fR
Verbose mode.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display help and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Display version information.
.SH "SEE ALSO"
\fBmount\fR(8), \fBfdisk\fR(8)
"#,
        roff_ru: r#".TH UMOUNT 8 "2026-10-30" "Wunix" "Команды системного администрирования"
.SH ИМЯ
umount \- отключение смонтированных файловых систем
.SH СИНТАКСИС
\fBumount\fR [\fIОПЦИИ\fR] \fIКАТАЛОГ\fR|\fIУСТРОЙСТВО\fR
.SH ОПИСАНИЕ
\fBumount\fR отключает файловую систему указанного каталога или устройства от общего дерева каталогов с помощью системного вызова umount2.
.SH ОПЦИИ
.TP
\fB\-f\fR, \fB\-\-force\fR
Принудительное размонтирование файловой системы.
.TP
\fB\-v\fR, \fB\-\-verbose\fR
Подробный вывод процесса размонтирования.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести версию утилиты.
.SH "СМ. ТАКЖЕ"
\fBmount\fR(8), \fBfdisk\fR(8)
"#,
    },
    ManPageEntry {
        name: "date",
        section: 1,
        desc_en: "print or set the system date and time",
        desc_ru: "вывод и установка системной даты и времени",
        roff_en: r#".TH DATE 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
date \- print or set the system date and time
.SH SYNOPSIS
\fBdate\fR [\fIOPTION\fR]... [+\fIFORMAT\fR]
.SH DESCRIPTION
Display date and time in the given \fIFORMAT\fR, or set the system date and time.
.SH OPTIONS
.TP
\fB\-d\fR, \fB\-\-date\fR=\fISTRING\fR
Display time described by \fISTRING\fR, not 'now'.
.TP
\fB\-s\fR, \fB\-\-set\fR=\fISTRING\fR
Set time described by \fISTRING\fR.
.TP
\fB\-u\fR, \fB\-\-utc\fR, \fB\-\-universal\fR
Print or set Coordinated Universal Time (UTC).
.TP
\fB\-R\fR, \fB\-\-rfc\-email\fR
Output date and time in RFC 5322 format.
.TP
\fB\-I\fR[\fIFMT\fR], \fB\-\-iso\-8601\fR[=\fIFMT\fR]
Output date/time in ISO 8601 format.
.TP
\fB\-r\fR, \fB\-\-reference\fR=\fIFILE\fR
Display the last modification time of \fIFILE\fR.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Output version information and exit.
.SH "SEE ALSO"
\fBhwclock\fR(1), \fBtimedatectl\fR(1), \fBtime\fR(1)
"#,
        roff_ru: r#".TH DATE 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
date \- вывод и установка системной даты и времени
.SH СИНТАКСИС
\fBdate\fR [\fIОПЦИИ\fR]... [+\fIФОРМАТ\fR]
.SH ОПИСАНИЕ
\fBdate\fR выводит текущую дату и время в заданном \fIФОРМАТЕ\fR или устанавливает системные часы ядра Wunix.
.SH ОПЦИИ
.TP
\fB\-d\fR, \fB\-\-date\fR=\fIСТРОКА\fR
Вывести время, описанное СТРОКОЙ вместо текущего момента.
.TP
\fB\-s\fR, \fB\-\-set\fR=\fIСТРОКА\fR
Установить системное время ядра из заданной СТРОКИ.
.TP
\fB\-u\fR, \fB\-\-utc\fR, \fB\-\-universal\fR
Использовать всемирное координированное время (UTC).
.TP
\fB\-R\fR, \fB\-\-rfc\-email\fR
Вывод в стандартном формате RFC 5322.
.TP
\fB\-I\fR[\fIФОРМАТ\fR], \fB\-\-iso\-8601\fR[=\fIФОРМАТ\fR]
Вывод в формате стандарта ISO 8601.
.TP
\fB\-r\fR, \fB\-\-reference\fR=\fIФАЙЛ\fR
Вывести время модификации указанного файла.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести информацию о версии и выйти.
.SH "СМ. ТАКЖЕ"
\fBhwclock\fR(1), \fBtimedatectl\fR(1), \fBtime\fR(1)
"#,
    },
    ManPageEntry {
        name: "time",
        section: 1,
        desc_en: "time a simple command or give resource usage",
        desc_ru: "хронометраж времени выполнения команды и использования ресурсов",
        roff_en: r#".TH TIME 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
time \- time a simple command or give resource usage
.SH SYNOPSIS
\fBtime\fR [\fIOPTIONS\fR]... \fICOMMAND\fR [\fIARGS\fR]...
.SH DESCRIPTION
Runs the specified \fICOMMAND\fR and prints timing statistics to standard error.
.SH OPTIONS
.TP
\fB\-p\fR, \fB\-\-portability\fR
Use POSIX standard format (real %e, user %U, sys %S).
.TP
\fB\-v\fR, \fB\-\-verbose\fR
Print detailed summary of execution time and resources.
.TP
\fB\-o\fR \fIFILE\fR, \fB\-\-output\fR=\fIFILE\fR
Write output to specified file instead of stderr.
.TP
\fB\-a\fR, \fB\-\-append\fR
Append output to file instead of overwriting.
.TP
\fB\-f\fR \fIFMT\fR, \fB\-\-format\fR=\fIFMT\fR
Custom output format string (%E, %e, %x, %C).
.TP
\fB\-h\fR, \fB\-\-help\fR
Print help and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Print version information.
.SH "SEE ALSO"
\fBdate\fR(1), \fBtop\fR(1)
"#,
        roff_ru: r#".TH TIME 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
time \- хронометраж времени выполнения команды и использования ресурсов
.SH СИНТАКСИС
\fBtime\fR [\fIОПЦИИ\fR]... \fIКОМАНДА\fR [\fIАРГУМЕНТЫ\fR]...
.SH ОПИСАНИЕ
\fBtime\fR запускает команду и выводит точное время её выполнения в поток stderr.
.SH ОПЦИИ
.TP
\fB\-p\fR, \fB\-\-portability\fR
Стандартный формат POSIX (real %e, user %U, sys %S).
.TP
\fB\-v\fR, \fB\-\-verbose\fR
Подробный отчет обо всех системных ресурсах процесса.
.TP
\fB\-o\fR \fIФАЙЛ\fR, \fB\-\-output\fR=\fIФАЙЛ\fR
Перенаправить вывод в указанный файл.
.TP
\fB\-a\fR, \fB\-\-append\fR
Дописывать результаты в конец файла.
.TP
\fB\-f\fR \fIФОРМАТ\fR, \fB\-\-format\fR=\fIФОРМАТ\fR
Пользовательский формат вывода (%E, %e, %x, %C).
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести информацию о версии.
.SH "СМ. ТАКЖЕ"
\fBdate\fR(1), \fBtop\fR(1)
"#,
    },
    ManPageEntry {
        name: "timedatectl",
        section: 1,
        desc_en: "control the system time and date",
        desc_ru: "управление системным временем, датой и часовыми поясами",
        roff_en: r#".TH TIMEDATECTL 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
timedatectl \- control the system time and date
.SH SYNOPSIS
\fBtimedatectl\fR [\fIOPTIONS\fR]... [\fICOMMAND\fR]
.SH DESCRIPTION
Query and change system clock, RTC, and timezone settings.
.SH COMMANDS
.TP
\fBstatus\fR
Show current time settings and status (default).
.TP
\fBshow\fR
Show properties in KEY=VALUE format.
.TP
\fBset\-time\fR \fITIME\fR
Set the system clock ('YYYY-MM-DD HH:MM:SS').
.TP
\fBset\-timezone\fR \fIZONE\fR
Set the system time zone (persisted in /etc/timezone).
.TP
\fBlist\-timezones\fR
List available time zones.
.TP
\fBset\-local\-rtc\fR [\fIBOOL\fR]
Control whether RTC is in local time.
.TP
\fBset\-ntp\fR [\fIBOOL\fR]
Control whether NTP time synchronization is enabled.
.TP
\fBtimesync\-status\fR
Show status of the time synchronization service.
.SH OPTIONS
.TP
\fB\-h\fR, \fB\-\-help\fR
Show help and exit.
.TP
\fB\-\-version\fR
Print version information.
.SH "SEE ALSO"
\fBdate\fR(1), \fBhwclock\fR(1)
"#,
        roff_ru: r#".TH TIMEDATECTL 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
timedatectl \- управление системным временем, датой и часовыми поясами
.SH СИНТАКСИС
\fBtimedatectl\fR [\fIОПЦИИ\fR]... [\fIКОМАНДА\fR]
.SH ОПИСАНИЕ
\fBtimedatectl\fR управляет системными часами ядра, аппаратными часами RTC и часовыми поясами.
.SH КОМАНДЫ
.TP
\fBstatus\fR
Показать текущие настройки времени и статус (по умолчанию).
.TP
\fBshow\fR
Вывести свойства в формате KEY=VALUE.
.TP
\fBset\-time\fR \fIВРЕМЯ\fR
Установить системные часы ядра ('YYYY-MM-DD HH:MM:SS').
.TP
\fBset\-timezone\fR \fIПОЯС\fR
Установить системный часовой пояс (/etc/timezone).
.TP
\fBlist\-timezones\fR
Вывести список доступных часовых поясов.
.TP
\fBset\-local\-rtc\fR [\fI0|1\fR]
Управлять хранением местного времени в RTC.
.TP
\fBset\-ntp\fR [\fI0|1\fR]
Включить или отключить сетевую синхронизацию NTP.
.TP
\fBtimesync\-status\fR
Показать статус службы синхронизации времени Wunix TimeSync.
.SH ОПЦИИ
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.TP
\fB\-\-version\fR
Вывести информацию о версии программы.
.SH "СМ. ТАКЖЕ"
\fBdate\fR(1), \fBhwclock\fR(1), \fBntpdate\fR(1), \fBtimesyncd\fR(8)
"#,
    },
    ManPageEntry {
        name: "ntpdate",
        section: 1,
        desc_en: "set the date and time via NTP",
        desc_ru: "установка и синхронизация даты и времени по протоколу NTP",
        roff_en: r#".TH NTPDATE 1 "2026-11-02" "Wunix" "User Commands"
.SH NAME
ntpdate \- set the date and time via NTP
.SH SYNOPSIS
\fBntpdate\fR [\fIOPTIONS\fR]... [\fISERVER\fR]...
.SH DESCRIPTION
\fBntpdate\fR sets the local date and time by polling Network Time Protocol (NTP) servers.
.SH OPTIONS
.TP
\fB\-q\fR
Query only \- do not set clock.
.TP
\fB\-b\fR
Force stepping time.
.TP
\fB\-u\fR
Use unprivileged port.
.TP
\fB\-s\fR
Silent mode.
.TP
\fB\-v\fR
Verbose output.
.TP
\fB\-h\fR, \fB\-\-help\fR
Show help and exit.
.SH "SEE ALSO"
\fBtimedatectl\fR(1), \fBdate\fR(1), \fBtimesyncd\fR(8)
"#,
        roff_ru: r#".TH NTPDATE 1 "2026-11-02" "Wunix" "Команды пользователя"
.SH ИМЯ
ntpdate \- установка системных часов по протоколу NTP
.SH СИНТАКСИС
\fBntpdate\fR [\fIОПЦИИ\fR]... [\fIСЕРВЕР\fR]...
.SH ОПИСАНИЕ
\fBntpdate\fR опрашивает серверы сетевого времени по протоколу NTP/SNTP (порт 123 UDP) и корректирует системное время ядра.
.SH ОПЦИИ
.TP
\fB\-q\fR
Режим только запроса (не изменять системные часы).
.TP
\fB\-b\fR
Ступенчатая коррекция времени (step time).
.TP
\fB\-u\fR
Использовать непривилегированный исходящий порт UDP.
.TP
\fB\-s\fR
Тихий режим работы.
.TP
\fB\-v\fR
Подробный вывод параметров синхронизации.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH "СМ. ТАКЖЕ"
\fBtimedatectl\fR(1), \fBdate\fR(1), \fBtimesyncd\fR(8)
"#,
    },
    ManPageEntry {
        name: "timesyncd",
        section: 8,
        desc_en: "network time synchronization service",
        desc_ru: "демон автоматической сетевой синхронизации времени Wunix",
        roff_en: r#".TH TIMESYNCD 8 "2026-11-02" "Wunix" "System Administration"
.SH NAME
timesyncd \- network time synchronization daemon
.SH SYNOPSIS
\fBtimesyncd\fR [\fIOPTIONS\fR]...
.SH DESCRIPTION
\fBtimesyncd\fR periodically synchronizes the system clock with NTP servers.
.SH OPTIONS
.TP
\fB\-1\fR, \fB\-\-once\fR
Run synchronization once and exit.
.TP
\fB\-d\fR, \fB\-\-daemon\fR
Run continuously in background daemon mode.
.TP
\fB\-v\fR, \fB\-\-verbose\fR
Verbose logging.
.TP
\fB\-h\fR, \fB\-\-help\fR
Show help and exit.
.SH "SEE ALSO"
\fBtimedatectl\fR(1), \fBntpdate\fR(1), \fBhwclock\fR(1)
"#,
        roff_ru: r#".TH TIMESYNCD 8 "2026-11-02" "Wunix" "Системное администрирование"
.SH ИМЯ
timesyncd \- фоновая служба автоматической сетевой синхронизации времени
.SH СИНТАКСИС
\fBtimesyncd\fR [\fIОПЦИИ\fR]...
.SH ОПИСАНИЕ
\fBtimesyncd\fR автоматически поддерживает точность системного времени Wunix путем регулярного опроса серверов NTP.
.SH ОПЦИИ
.TP
\fB\-1\fR, \fB\-\-once\fR
Выполнить синхронизацию один раз и завершить работу.
.TP
\fB\-d\fR, \fB\-\-daemon\fR
Работать в фоновом режиме службы.
.TP
\fB\-v\fR, \fB\-\-verbose\fR
Подробный вывод процесса синхронизации.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH "СМ. ТАКЖЕ"
\fBtimedatectl\fR(1), \fBntpdate\fR(1), \fBhwclock\fR(1)
"#,
    },
    ManPageEntry {
        name: "shutdown",
        section: 8,
        desc_en: "halt, power-off or reboot the machine",
        desc_ru: "безопасное завершение работы, выключение питания или перезапуск системы",
        roff_en: r#".TH SHUTDOWN 8 "2026-10-30" "Wunix" "System Administration"
.SH NAME
shutdown \- halt, power-off or reboot the machine
.SH SYNOPSIS
\fBshutdown\fR [\fIOPTIONS\fR]... [\fITIME\fR] [\fIWALL\fR...]
.SH DESCRIPTION
Brings the system down cleanly, broadcasting warning messages and syncing disks.
.SH OPTIONS
.TP
\fB\-h\fR, \fB\-P\fR, \fB\-\-poweroff\fR
Power-off the machine (the default).
.TP
\fB\-r\fR, \fB\-\-reboot\fR
Reboot the machine.
.TP
\fB\-H\fR, \fB\-\-halt\fR
Halt the machine without ACPI poweroff.
.TP
\fB\-c\fR, \fB\-\-cancel\fR
Cancel a pending shutdown.
.TP
\fB\-k\fR
Only send wall warning messages, do not actually shut down.
.TP
\fB\-\-no\-wall\fR
Do not send wall broadcast before shutdown.
.TP
\fB\-\-help\fR
Display this help and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Print version information.
.SH "SEE ALSO"
\fBreboot\fR(8), \fBinit\fR(8)
"#,
        roff_ru: r#".TH SHUTDOWN 8 "2026-10-30" "Wunix" "Системное администрирование"
.SH ИМЯ
shutdown \- безопасное завершение работы, выключение питания или перезапуск системы
.SH СИНТАКСИС
\fBshutdown\fR [\fIОПЦИИ\fR]... [\fIВРЕМЯ\fR] [\fIСООБЩЕНИЕ\fR...]
.SH ОПИСАНИЕ
\fBshutdown\fR безопасно переводит операционную систему в состояние остановки или перезапуска.
.SH ОПЦИИ
.TP
\fB\-h\fR, \fB\-P\fR, \fB\-\-poweroff\fR
Выключить питание компьютера (действие по умолчанию).
.TP
\fB\-r\fR, \fB\-\-reboot\fR
Перезагрузить систему.
.TP
\fB\-H\fR, \fB\-\-halt\fR
Остановить процессор без отключения питания ACPI.
.TP
\fB\-c\fR, \fB\-\-cancel\fR
Отменить ранее запланированное выключение.
.TP
\fB\-k\fR
Только отправить предупреждения без остановки системы.
.TP
\fB\-\-no\-wall\fR
Не рассылать широковещательное сообщение.
.TP
\fB\-\-help\fR
Показать эту справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести информацию о версии программы.
.SH "СМ. ТАКЖЕ"
\fBreboot\fR(8), \fBinit\fR(8)
"#,
    },
    ManPageEntry {
        name: "reboot",
        section: 8,
        desc_en: "reboot, halt, or power-off the machine",
        desc_ru: "перезагрузка, остановка или отключение питания системы",
        roff_en: r#".TH REBOOT 8 "2026-10-30" "Wunix" "System Administration"
.SH NAME
reboot \- reboot, halt, or power-off the machine
.SH SYNOPSIS
\fBreboot\fR [\fIOPTIONS\fR]...
.SH DESCRIPTION
Reboots, halts, or powers off the system hardware.
.SH OPTIONS
.TP
\fB\-f\fR, \fB\-\-force\fR
Force immediate reboot without clean shutdown or sync.
.TP
\fB\-p\fR, \fB\-\-poweroff\fR
Power-off the machine instead of rebooting.
.TP
\fB\-d\fR, \fB\-\-halt\fR
Halt the machine instead of rebooting.
.TP
\fB\-w\fR, \fB\-\-wtmp\-only\fR
Only write wtmp record, do not reboot.
.TP
\fB\-n\fR, \fB\-\-no\-sync\fR
Don't sync disks before reboot or halt.
.TP
\fB\-\-no\-wall\fR
Don't send wall broadcast message before reboot.
.TP
\fB\-h\fR, \fB\-\-help\fR
Print help and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Print version information and exit.
.SH "SEE ALSO"
\fBshutdown\fR(8), \fBinit\fR(8)
"#,
        roff_ru: r#".TH REBOOT 8 "2026-10-30" "Wunix" "Системное администрирование"
.SH ИМЯ
reboot \- перезагрузка, остановка или отключение питания системы
.SH СИНТАКСИС
\fBreboot\fR [\fIОПЦИИ\fR]...
.SH ОПИСАНИЕ
\fBreboot\fR выполняет аппаратный перезапуск компьютера.
.SH ОПЦИИ
.TP
\fB\-f\fR, \fB\-\-force\fR
Немедленный принудительный сброс без вызова init и sync.
.TP
\fB\-p\fR, \fB\-\-poweroff\fR
Выключить питание компьютера вместо перезапуска.
.TP
\fB\-d\fR, \fB\-\-halt\fR
Остановить систему вместо перезапуска.
.TP
\fB\-w\fR, \fB\-\-wtmp\-only\fR
Только записать событие в системный журнал wtmp.
.TP
\fB\-n\fR, \fB\-\-no\-sync\fR
Не синхронизировать файловые системы перед сбросом.
.TP
\fB\-\-no\-wall\fR
Не отправлять широковещательное оповещение.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести информацию о версии программы.
.SH "СМ. ТАКЖЕ"
\fBshutdown\fR(8), \fBinit\fR(8)
"#,
    },

    ManPageEntry {
        name: "chmod",
        section: 1,
        desc_en: "change file mode bits (permissions)",
        desc_ru: "изменение прав доступа к файлам",
        roff_en: r#".TH CHMOD 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
chmod \- change file mode bits (permissions)
.SH SYNOPSIS
\fBchmod\fR [\fIOPTION\fR]... \fIMODE\fR \fIFILE\fR...
.SH DESCRIPTION
\fBchmod\fR changes the file mode bits of each given \fIFILE\fR according to \fIMODE\fR, which can be either an octal number or a symbolic representation of changes to make.
.SH OPTIONS
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.SH "MODE SPECIFICATION"
Octal modes consist of 3 or 4 octal digits (e.g. 755, 0644, 4755 for SUID).
.br
Symbolic modes have the form [ugoa...][[+-=][perms...]...], where perms can be zero or more letters from r, w, and x (e.g. +x, -w, u+x, a+r).
.SH EXAMPLES
.TP
chmod 755 /bin/app
.TP
chmod +x script.sh
.TP
chmod 4755 /bin/su
.SH "SEE ALSO"
\fBchown\fR(1), \fBls\fR(1), \fBstat\fR(2)
"#,
        roff_ru: r#".TH CHMOD 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
chmod \- изменение прав доступа к файлам
.SH СИНТАКСИС
\fBchmod\fR [\fIОПЦИИ\fR]... \fIРЕЖИМ\fR \fIФАЙЛ\fR...
.SH ОПИСАНИЕ
\fBchmod\fR изменяет биты прав доступа каждого указанного \fIФАЙЛА\fR в соответствии с заданным \fIРЕЖИМОМ\fR, который может быть восьмеричным числом или символьным выражением.
.SH ОПЦИИ
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH "ФОРМАТ РЕЖИМА"
Восьмеричный режим задается 3 или 4 цифрами от 0 до 7 (например, 755, 0644, 4755 для бита SUID).
.br
Символьный режим имеет вид [ugoa...][[+-=][права...]], где права задаются символами r (чтение), w (запись), x (выполнение), например: +x, -w, u+x, a+r.
.SH ПРИМЕРЫ
.TP
chmod 755 /bin/app
.TP
chmod +x script.sh
.TP
chmod 4755 /bin/su
.SH "СМ. ТАКЖЕ"
\fBchown\fR(1), \fBls\fR(1), \fBstat\fR(2)
"#,
    },
    ManPageEntry {
        name: "chown",
        section: 1,
        desc_en: "change file owner and group",
        desc_ru: "изменение владельца и группы файлов",
        roff_en: r#".TH CHOWN 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
chown \- change file owner and group
.SH SYNOPSIS
\fBchown\fR [\fIOPTION\fR]... [\fIOWNER\fR][:\fIGROUP\fR] \fIFILE\fR...
.SH DESCRIPTION
\fBchown\fR changes the user and/or group ownership of each given \fIFILE\fR. If only an owner (user name or numeric UID) is given, that user becomes the owner of each given file and the file group is unchanged. If the owner is followed by a colon and a group name or numeric GID, the file group is changed as well.
.SH OPTIONS
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.SH EXAMPLES
.TP
chown root /file
.TP
chown root:wheel /file
.TP
chown :wheel /file
.SH "SEE ALSO"
\fBchmod\fR(1), \fBid\fR(1), \fBpasswd\fR(1)
"#,
        roff_ru: r#".TH CHOWN 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
chown \- изменение владельца и группы файлов
.SH СИНТАКСИС
\fBchown\fR [\fIОПЦИИ\fR]... [\fIВЛАДЕЛЕЦ\fR][:\fIГРУППА\fR] \fIФАЙЛ\fR...
.SH ОПИСАНИЕ
\fBchown\fR изменяет владельца и/или группу каждого указанного \fIФАЙЛА\fR. Владелец и группа могут быть указаны в виде имени пользователя/группы или числового идентификатора (UID/GID).
.SH ОПЦИИ
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH ПРИМЕРЫ
.TP
chown root /file
.TP
chown root:wheel /file
.TP
chown :wheel /file
.SH "СМ. ТАКЖЕ"
\fBchmod\fR(1), \fBid\fR(1), \fBpasswd\fR(1)
"#,
    },
    ManPageEntry {
        name: "host",
        section: 1,
        desc_en: "DNS lookup utility",
        desc_ru: "утилита разрешения имен DNS",
        roff_en: r#".TH HOST 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
host \- DNS lookup utility
.SH SYNOPSIS
\fBhost\fR [\fIOPTIONS\fR] \fIDOMAIN_NAME\fR [\fISERVER\fR]
.br
\fBnslookup\fR \fIDOMAIN_NAME\fR [\fIDNS_SERVER\fR]
.SH DESCRIPTION
\fBhost\fR is a simple utility for performing DNS lookups. It is used to convert domain names to IP addresses. When invoked as \fBnslookup\fR, it presents the query and answer in traditional nslookup format. An optional custom DNS server IP address can be supplied.
.SH OPTIONS
.TP
\fB\-h\fR, \fB\-\-help\fR, \fB/?\fR
Display this help message and exit.
.TP
\fB\-V\fR, \fB\-\-version\fR
Display version information and exit.
.SH EXAMPLES
.TP
host ya.ru
.TP
host example.com 10.0.2.3
.TP
nslookup google.com 8.8.8.8
.SH "SEE ALSO"
\fBping\fR(8), \fBip\fR(8), \fBcurl\fR(1)
"#,
        roff_ru: r#".TH HOST 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
host \- утилита разрешения имен DNS (DNS lookup utility)
.SH СИНТАКСИС
\fBhost\fR [\fIОПЦИИ\fR] \fIДОМЕННОЕ_ИМЯ\fR [\fIСЕРВЕР\fR]
.br
\fBnslookup\fR \fIДОМЕННОЕ_ИМЯ\fR [\fIDNS_СЕРВЕР\fR]
.SH ОПИСАНИЕ
\fBhost\fR — утилита для выполнения запросов к системе доменных имен (DNS). Преобразует доменные имена в IP-адреса. При вызове через псевдоним \fBnslookup\fR результат форматируется в классическом стиле утилиты nslookup. При необходимости можно указать альтернативный IP-адрес DNS-сервера.
.SH ОПЦИИ
.TP
\fB\-h\fR, \fB\-\-help\fR, \fB/?\fR
Показать эту справку и выйти.
.TP
\fB\-V\fR, \fB\-\-version\fR
Вывести информацию о версии программы и выйти.
.SH ПРИМЕРЫ
.TP
host ya.ru
.TP
host example.com 10.0.2.3
.TP
nslookup google.com 8.8.8.8
.SH "СМ. ТАКЖЕ"
\fBping\fR(8), \fBip\fR(8), \fBcurl\fR(1)
"#,
    },
    ManPageEntry {
        name: "id",
        section: 1,
        desc_en: "print real and effective user and group IDs",
        desc_ru: "вывод информации об идентификаторах пользователя и групп",
        roff_en: r#".TH ID 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
id \- print real and effective user and group IDs
.SH SYNOPSIS
\fBid\fR [\fIOPTION\fR]... [\fIUSER\fR]
.SH DESCRIPTION
\fBid\fR prints user and group information for the current process, or for the specified \fIUSER\fR name. The output includes numeric UID, user name, GID, group name, and supplementary groups.
.SH OPTIONS
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.SH EXAMPLES
.TP
id
.TP
id root
.SH "SEE ALSO"
\fBwhoami\fR(1), \fBsu\fR(1), \fBpasswd\fR(1)
"#,
        roff_ru: r#".TH ID 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
id \- вывод информации об идентификаторах пользователя и групп
.SH СИНТАКСИС
\fBid\fR [\fIОПЦИИ\fR]... [\fIПОЛЬЗОВАТЕЛЬ\fR]
.SH ОПИСАНИЕ
\fBid\fR выводит информацию об учетной записи текущего пользователя или указанного \fIПОЛЬЗОВАТЕЛЯ\fR. Вывод включает числовой идентификатор пользователя (UID), имя, основной идентификатор группы (GID), имя группы и список дополнительных групп.
.SH ОПЦИИ
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH ПРИМЕРЫ
.TP
id
.TP
id root
.SH "СМ. ТАКЖЕ"
\fBwhoami\fR(1), \fBsu\fR(1), \fBpasswd\fR(1)
"#,
    },
    ManPageEntry {
        name: "ln",
        section: 1,
        desc_en: "make links between files",
        desc_ru: "создание ссылок на файлы",
        roff_en: r#".TH LN 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
ln \- make links between files
.SH SYNOPSIS
\fBln\fR [\fIOPTION\fR]... \fITARGET\fR [\fILINK_NAME\fR]
.SH DESCRIPTION
\fBln\fR creates links to files. With \fB\-s\fR, it creates a symbolic link pointing to TARGET.
.SH OPTIONS
.TP
\fB\-s\fR, \fB\-\-symbolic\fR
Make symbolic links instead of hard links.
.TP
\fB\-f\fR, \fB\-\-force\fR
Remove existing destination files.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.SH "SEE ALSO"
\fBreadlink\fR(1), \fBls\fR(1)
"#,
        roff_ru: r#".TH LN 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
ln \- создание ссылок на файлы
.SH СИНТАКСИС
\fBln\fR [\fIОПЦИИ\fR]... \fIЦЕЛЬ\fR [\fIИМЯ_ССЫЛКИ\fR]
.SH ОПИСАНИЕ
\fBln\fR создаёт ссылки на файлы. При указании ключа \fB\-s\fR создаётся символическая ссылка (symlink), указывающая на целевой путь.
.SH ОПЦИИ
.TP
\fB\-s\fR, \fB\-\-symbolic\fR
Создавать символические ссылки вместо жестких.
.TP
\fB\-f\fR, \fB\-\-force\fR
Удалять существующие целевые файлы перед созданием ссылки.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH "СМ. ТАКЖЕ"
\fBreadlink\fR(1), \fBls\fR(1)
"#,
    },
    ManPageEntry {
        name: "login",
        section: 1,
        desc_en: "begin session on the system",
        desc_ru: "инициализация сеанса входа пользователя в систему",
        roff_en: r#".TH LOGIN 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
login \- begin session on the system
.SH SYNOPSIS
\fBlogin\fR [\fB\-p\fR] [\fB\-f\fR \fIuser\fR] [\fIuser\fR]
.SH DESCRIPTION
\fBlogin\fR is used to establish a new session with the system. It prompts the user for a username and password, displaying masking asterisks (\fB*\fR) on password input instead of hiding characters completely, authenticates credentials against \fB/etc/passwd\fR and \fB/etc/shadow\fR, configures process UID/GID and environment variables (USER, LOGNAME, HOME, SHELL, PATH, TERM), changes directory to the user's home, and starts the login shell.
.SH OPTIONS
.TP
\fB\-f\fR \fIuser\fR
Force login as the specified user without requesting a password (root only).
.TP
\fB\-p\fR
Preserve the existing environment variables.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display help summary and exit.
.SH "SEE ALSO"
\fBpasswd\fR(1), \fBsu\fR(1), \fBwsh\fR(1), \fBinit\fR(8)
"#,
        roff_ru: r#".TH LOGIN 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
login \- инициализация сеанса входа пользователя в систему
.SH СИНТАКСИС
\fBlogin\fR [\fB\-p\fR] [\fB\-f\fR \fIпользователь\fR] [\fIпользователь\fR]
.SH ОПИСАНИЕ
\fBlogin\fR используется для открытия нового пользовательского сеанса. Программа запрашивает имя пользователя и пароль, отображая при вводе пароля маскировочные символы \fB*\fR вместо скрытого ввода, проверяет подлинность по базам данных \fB/etc/passwd\fR и \fB/etc/shadow\fR, устанавливает соответствующие UID/GID процесса, переменные окружения (USER, LOGNAME, HOME, SHELL, PATH, TERM), переходит в домашний каталог пользователя и запускает командный шелл.
.SH ОПЦИИ
.TP
\fB\-f\fR \fIпользователь\fR
Принудительный вход под указанным пользователем без запроса пароля (доступно только суперпользователю root).
.TP
\fB\-p\fR
Сохранять переменные окружения текущего сеанса.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать краткую справку по использованию и выйти.
.SH "СМ. ТАКЖЕ"
\fBpasswd\fR(1), \fBsu\fR(1), \fBwsh\fR(1), \fBinit\fR(8)
"#,
    },
    ManPageEntry {
        name: "mkdir",
        section: 1,
        desc_en: "make directories",
        desc_ru: "создание каталогов",
        roff_en: r#".TH MKDIR 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
mkdir \- make directories
.SH SYNOPSIS
\fBmkdir\fR [\fIOPTION\fR]... \fIDIRECTORY\fR...
.SH DESCRIPTION
\fBmkdir\fR creates the DIRECTORY(ies), if they do not already exist.
.SH OPTIONS
.TP
\fB\-p\fR, \fB\-\-parents\fR
No error if existing, make parent directories as needed.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.SH "SEE ALSO"
\fBrmdir\fR(1), \fBls\fR(1)
"#,
        roff_ru: r#".TH MKDIR 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
mkdir \- создание каталогов
.SH СИНТАКСИС
\fBmkdir\fR [\fIОПЦИИ\fR]... \fIКАТАЛОГ\fR...
.SH ОПИСАНИЕ
\fBmkdir\fR создаёт каталоги с указанными именами, если они ещё не существуют.
.SH ОПЦИИ
.TP
\fB\-p\fR, \fB\-\-parents\fR
Создавать недостающие промежуточные родительские каталоги без ошибки, если каталог уже существует.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH "СМ. ТАКЖЕ"
\fBrmdir\fR(1), \fBls\fR(1)
"#,
    },
    ManPageEntry {
        name: "passwd",
        section: 1,
        desc_en: "change user password",
        desc_ru: "изменение пароля учетной записи пользователя",
        roff_en: r#".TH PASSWD 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
passwd \- change user password
.SH SYNOPSIS
\fBpasswd\fR [\fIOPTION\fR]... [\fIUSER\fR]
.SH DESCRIPTION
\fBpasswd\fR updates the user's authentication tokens (password hash) in \fB/etc/shadow\fR. Normal users may only change the password for their own account, whereas the superuser root may change the password for any account without knowing the current password.
.SH OPTIONS
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.SH EXAMPLES
.TP
passwd
.TP
passwd root
.SH "SEE ALSO"
\fBlogin\fR(1), \fBsu\fR(1), \fBuseradd\fR(8)
"#,
        roff_ru: r#".TH PASSWD 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
passwd \- изменение пароля учетной записи пользователя
.SH СИНТАКСИС
\fBpasswd\fR [\fIОПЦИИ\fR]... [\fIПОЛЬЗОВАТЕЛЬ\fR]
.SH ОПИСАНИЕ
\fBpasswd\fR обновляет пароль (хэш аутентификации) учетной записи в файле \fB/etc/shadow\fR. Обычные пользователи могут менять только собственный пароль (с подтверждением старого), а суперпользователь root имеет право задавать пароль для любой учетной записи.
.SH ОПЦИИ
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH ПРИМЕРЫ
.TP
passwd
.TP
passwd root
.SH "СМ. ТАКЖЕ"
\fBlogin\fR(1), \fBsu\fR(1), \fBuseradd\fR(8)
"#,
    },
    ManPageEntry {
        name: "readlink",
        section: 1,
        desc_en: "print resolved symbolic links or canonical file names",
        desc_ru: "печать значения символической ссылки или канонического имени файла",
        roff_en: r#".TH READLINK 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
readlink \- print resolved symbolic links or canonical file names
.SH SYNOPSIS
\fBreadlink\fR [\fIOPTION\fR]... \fIFILE\fR...
.SH DESCRIPTION
\fBreadlink\fR prints the value of a symbolic link. With \fB\-f\fR, it canonicalizes by following every symlink recursively.
.SH OPTIONS
.TP
\fB\-f\fR, \fB\-\-canonicalize\fR
Canonicalize by following every symlink in every component of the given name recursively.
.TP
\fB\-n\fR, \fB\-\-no-newline\fR
Do not output the trailing delimiter.
.TP
\fB\-q\fR, \fB\-s\fR, \fB\-\-silent\fR, \fB\-\-quiet\fR
Suppress most error messages.
.TP
\fB\-v\fR, \fB\-\-verbose\fR
Report error messages.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.SH "SEE ALSO"
\fBln\fR(1), \fBls\fR(1)
"#,
        roff_ru: r#".TH READLINK 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
readlink \- печать значения символической ссылки или канонического имени файла
.SH СИНТАКСИС
\fBreadlink\fR [\fIОПЦИИ\fR]... \fIФАЙЛ\fR...
.SH ОПИСАНИЕ
\fBreadlink\fR печатает значение символической ссылки (target). При указании ключа канонизации \fB\-f\fR разрешает все промежуточные символические ссылки и нормализует путь.
.SH ОПЦИИ
.TP
\fB\-f\fR, \fB\-\-canonicalize\fR
Канонизировать путь, разрешая все символические ссылки рекурсивно.
.TP
\fB\-n\fR, \fB\-\-no-newline\fR
Не выводить завершающий перевод строки.
.TP
\fB\-q\fR, \fB\-s\fR, \fB\-\-silent\fR, \fB\-\-quiet\fR
Подавлять большинство сообщений об ошибках.
.TP
\fB\-v\fR, \fB\-\-verbose\fR
Сообщать обо всех ошибках.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH "СМ. ТАКЖЕ"
\fBln\fR(1), \fBls\fR(1)
"#,
    },
    ManPageEntry {
        name: "rmdir",
        section: 1,
        desc_en: "remove empty directories",
        desc_ru: "удаление пустых каталогов",
        roff_en: r#".TH RMDIR 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
rmdir \- remove empty directories
.SH SYNOPSIS
\fBrmdir\fR [\fIOPTION\fR]... \fIDIRECTORY\fR...
.SH DESCRIPTION
\fBrmdir\fR removes each DIRECTORY, if it is empty.
.SH OPTIONS
.TP
\fB\-p\fR, \fB\-\-parents\fR
Remove DIRECTORY and its ancestors; e.g., 'rmdir -p a/b' is similar to 'rmdir a/b a'.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.SH "SEE ALSO"
\fBmkdir\fR(1), \fBrm\fR(1)
"#,
        roff_ru: r#".TH RMDIR 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
rmdir \- удаление пустых каталогов
.SH СИНТАКСИС
\fBrmdir\fR [\fIОПЦИИ\fR]... \fIКАТАЛОГ\fR...
.SH ОПИСАНИЕ
\fBrmdir\fR удаляет указанные пустые каталоги. Если каталог содержит файлы или подкаталоги, команда завершается с ошибкой.
.SH ОПЦИИ
.TP
\fB\-p\fR, \fB\-\-parents\fR
Удалять каталог и его предков, если они также пусты.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH "СМ. ТАКЖЕ"
\fBmkdir\fR(1), \fBrm\fR(1)
"#,
    },
    ManPageEntry {
        name: "su",
        section: 1,
        desc_en: "run a command shell with substitute user and group ID",
        desc_ru: "переключение идентификатора пользователя и запуск шелла",
        roff_en: r#".TH SU 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
su \- run a command shell with substitute user and group ID
.SH SYNOPSIS
\fBsu\fR [\fIOPTION\fR]... [\fIUSER\fR]
.SH DESCRIPTION
\fBsu\fR switches the current user session to that of \fIUSER\fR (defaulting to superuser \fBroot\fR). If the caller is not root, \fBsu\fR prompts for the target user's password. Upon successful verification, it changes UID/GID, updates environment variables, and launches the target user's shell.
.SH OPTIONS
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.SH EXAMPLES
.TP
su
.TP
su root
.TP
su user
.SH "SEE ALSO"
\fBsudo\fR(8), \fBlogin\fR(1), \fBid\fR(1), \fBwhoami\fR(1)
"#,
        roff_ru: r#".TH SU 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
su \- переключение идентификатора пользователя и запуск шелла
.SH СИНТАКСИС
\fBsu\fR [\fIОПЦИИ\fR]... [\fIПОЛЬЗОВАТЕЛЬ\fR]
.SH ОПИСАНИЕ
\fBsu\fR переключает текущий сеанс на учетную запись указанного \fIПОЛЬЗОВАТЕЛЯ\fR (по умолчанию \fBroot\fR). Если команда вызвана непривилегированным пользователем, запрашивается пароль целевой учетной записи. При успешной проверке устанавливаются UID/GID целевого пользователя и запускается его командный шелл.
.SH ОПЦИИ
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH ПРИМЕРЫ
.TP
su
.TP
su root
.TP
su user
.SH "СМ. ТАКЖЕ"
\fBsudo\fR(8), \fBlogin\fR(1), \fBid\fR(1), \fBwhoami\fR(1)
"#,
    },
    ManPageEntry {
        name: "system",
        section: 1,
        desc_en: "manage and supervise system services and daemons",
        desc_ru: "управление и супервизия системных служб и демонов",
        roff_en: r#".TH SYSTEM 1 "2026-11-03" "Wunix" "User Commands"
.SH NAME
system \- manage and supervise system services and daemons
.SH SYNOPSIS
\fBsystem\fR \fICOMMAND\fR [\fISERVICE\fR] [\fIOPTIONS\fR]...
.SH DESCRIPTION
\fBsystem\fR is the service supervisor and daemon manager for Wunix.
It provides service lifecycle control (start, stop, restart), status inspection,
autostart management, log viewing, and service definition editing.
.SH COMMANDS
.TP
\fBstart\fR \fISERVICE\fR
Start the specified service.
.TP
\fBstop\fR \fISERVICE\fR
Stop the specified service.
.TP
\fBrestart\fR \fISERVICE\fR
Restart the specified service.
.TP
\fBstatus\fR [\fISERVICE\fR]
Display detailed status of a service (PID, uptime, logs) or list all services if omitted.
.TP
\fBlist\fR
List all registered services and their current states.
.TP
\fBcreate\fR \fISERVICE\fR
Generate a template configuration in /etc/services/ and open it in nano.
.TP
\fBedit\fR \fISERVICE\fR
Open service configuration file in nano.
.TP
\fBenable\fR \fISERVICE\fR
Enable autostart of the service at system boot.
.TP
\fBdisable\fR \fISERVICE\fR
Disable autostart of the service.
.TP
\fBlogs\fR \fISERVICE\fR [\fB-n\fR \fILINES\fR] [\fB-f\fR]
View stdout/stderr logs of the specified service.
.SH "SEE ALSO"
\fBinit\fR(8), \fBtimedatectl\fR(1)
"#,
        roff_ru: r#".TH SYSTEM 1 "2026-11-03" "Wunix" "Команды пользователя"
.SH ИМЯ
system \- управление и супервизия системных служб и демонов
.SH СИНТАКСИС
\fBsystem\fR \fIКОМАНДА\fR [\fIСЛУЖБА\fR] [\fIПАРАМЕТРЫ\fR]...
.SH ОПИСАНИЕ
\fBsystem\fR — менеджер служб и супервизор демонов для операционной системы Wunix.
Обеспечивает управление жизненным циклом служб (start, stop, restart), просмотр подробного статуса,
управление автозапуском, анализ журналов вывода и редактирование конфигурационных файлов.
.SH КОМАНДЫ
.TP
\fBstart\fR \fIСЛУЖБА\fR
Запуск указанной службы.
.TP
\fBstop\fR \fIСЛУЖБА\fR
Остановка указанной службы.
.TP
\fBrestart\fR \fIСЛУЖБА\fR
Перезапуск указанной службы.
.TP
\fBstatus\fR [\fIСЛУЖБА\fR]
Отображение детального цветного статуса службы (PID, время работы, лог) или списка всех служб.
.TP
\fBlist\fR
Вывод сводной таблицы всех зарегистрированных служб и их состояния.
.TP
\fBcreate\fR \fIСЛУЖБА\fR
Генерация шаблона конфигурации в /etc/services/ и открытие в редакторе nano.
.TP
\fBedit\fR \fIСЛУЖБА\fR
Открытие файла конфигурации службы в редакторе nano.
.TP
\fBenable\fR \fIСЛУЖБА\fR
Включение автозапуска службы при загрузке операционной системы.
.TP
\fBdisable\fR \fIСЛУЖБА\fR
Отключение автозапуска службы при загрузке.
.TP
\fBlogs\fR \fIСЛУЖБА\fR [\fB-n\fR \fIСТРОКИ\fR] [\fB-f\fR]
Просмотр журнала вывода службы (stdout/stderr).
.SH "СМ. ТАКЖЕ"
\fBinit\fR(8), \fBtimedatectl\fR(1)
"#,
    },
    ManPageEntry {
        name: "whoami",
        section: 1,
        desc_en: "print effective user name",
        desc_ru: "вывод имени текущего эффективного пользователя",
        roff_en: r#".TH WHOAMI 1 "2026-10-30" "Wunix" "User Commands"
.SH NAME
whoami \- print effective user name
.SH SYNOPSIS
\fBwhoami\fR [\fIOPTION\fR]
.SH DESCRIPTION
\fBwhoami\fR prints the user name associated with the current effective user ID (EUID). It is equivalent to \fBid \-un\fR.
.SH OPTIONS
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.SH "SEE ALSO"
\fBid\fR(1), \fBsu\fR(1), \fBlogin\fR(1)
"#,
        roff_ru: r#".TH WHOAMI 1 "2026-10-30" "Wunix" "Команды пользователя"
.SH ИМЯ
whoami \- вывод имени текущего эффективного пользователя
.SH СИНТАКСИС
\fBwhoami\fR [\fIОПЦИИ\fR]
.SH ОПИСАНИЕ
\fBwhoami\fR выводит имя пользователя, связанное с текущим эффективным идентификатором пользователя (EUID). Эквивалентно вызову команды \fBid \-un\fR.
.SH ОПЦИИ
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH "СМ. ТАКЖЕ"
\fBid\fR(1), \fBsu\fR(1), \fBlogin\fR(1)
"#,
    },
    ManPageEntry {
        name: "sudo",
        section: 8,
        desc_en: "execute a command as superuser root",
        desc_ru: "выполнение команды с привилегиями суперпользователя root",
        roff_en: r#".TH SUDO 8 "2026-10-30" "Wunix" "System Administration Commands"
.SH NAME
sudo \- execute a command as superuser root
.SH SYNOPSIS
\fBsudo\fR [\fIOPTION\fR] \fICOMMAND\fR [\fIARGUMENTS\fR...]
.SH DESCRIPTION
\fBsudo\fR allows a permitted user to execute a command with superuser privileges (as user root). If invoked by an unprivileged user, \fBsudo\fR checks membership in the administrative group (e.g. \fBwheel\fR) and prompts for the user's password before executing the target binary.
.SH OPTIONS
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.SH EXAMPLES
.TP
sudo fdisk /dev/vda
.TP
sudo mount -t vfat /dev/vda1 /mnt
.TP
sudo useradd newuser
.SH "SEE ALSO"
\fBsu\fR(1), \fBpasswd\fR(1), \fBid\fR(1)
"#,
        roff_ru: r#".TH SUDO 8 "2026-10-30" "Wunix" "Команды системного администрирования"
.SH ИМЯ
sudo \- выполнение команды с привилегиями суперпользователя root
.SH СИНТАКСИС
\fBsudo\fR [\fIОПЦИИ\fR] \fIКОМАНДА\fR [\fIАРГУМЕНТЫ\fR...]
.SH ОПИСАНИЕ
\fBsudo\fR позволяет авторизованному пользователю запускать системные команды с правами суперпользователя (root). При вызове непривилегированным пользователем проверяется принадлежность к административной группе \fBwheel\fR и запрашивается пароль текущего пользователя.
.SH ОПЦИИ
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH ПРИМЕРЫ
.TP
sudo fdisk /dev/vda
.TP
sudo mount -t vfat /dev/vda1 /mnt
.TP
sudo useradd newuser
.SH "СМ. ТАКЖЕ"
\fBsu\fR(1), \fBpasswd\fR(1), \fBid\fR(1)
"#,
    },
    ManPageEntry {
        name: "sudoadd",
        section: 8,
        desc_en: "grant or revoke sudo privileges for a user",
        desc_ru: "предоставление или отзыв прав sudo для пользователя",
        roff_en: r#".TH SUDOADD 8 "2026-11-02" "Wunix" "System Administration Commands"
.SH NAME
sudoadd \- grant or revoke sudo privileges for a user
.SH SYNOPSIS
\fBsudoadd\fR [\fIOPTION\fR] \fIUSERNAME\fR
.SH DESCRIPTION
\fBsudoadd\fR is an administrative utility that adds a user account to the \fBwheel\fR group, granting them authorization to execute commands as superuser root via \fBsudo\fR. It also supports revoking sudo access via \fB\-d\fR.
.SH OPTIONS
.TP
\fB\-d\fR, \fB\-\-delete\fR, \fB\-r\fR, \fB\-\-remove\fR
Revoke sudo privileges (remove user from the wheel group).
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.SH EXAMPLES
.TP
sudoadd alex
.TP
sudoadd -d alex
.SH "SEE ALSO"
\fBsudo\fR(8), \fBuseradd\fR(8), \fBuserdel\fR(8), \fBpasswd\fR(1)
"#,
        roff_ru: r#".TH SUDOADD 8 "2026-11-02" "Wunix" "Команды системного администрирования"
.SH ИМЯ
sudoadd \- предоставление или отзыв прав sudo для пользователя
.SH СИНТАКСИС
\fBsudoadd\fR [\fIОПЦИИ\fR] \fIИМЯ_ПОЛЬЗОВАТЕЛЯ\fR
.SH ОПИСАНИЕ
\fBsudoadd\fR \- утилита системного администрирования Wunix, добавляющая пользователя в группу \fBwheel\fR и предоставляющая ему возможность запускать команды с правами root через \fBsudo\fR. Поддерживает отзыв привилегий через опцию \fB\-d\fR.
.SH ОПЦИИ
.TP
\fB\-d\fR, \fB\-\-delete\fR, \fB\-r\fR, \fB\-\-remove\fR
Отозвать права sudo (удалить пользователя из группы wheel).
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH ПРИМЕРЫ
.TP
sudoadd alex
.TP
sudoadd -d alex
.SH "СМ. ТАКЖЕ"
\fBsudo\fR(8), \fBuseradd\fR(8), \fBuserdel\fR(8), \fBpasswd\fR(1)
"#,
    },
    ManPageEntry {
        name: "useradd",
        section: 8,
        desc_en: "create a new user account and home directory",
        desc_ru: "создание новой учетной записи пользователя и домашнего каталога",
        roff_en: r#".TH USERADD 8 "2026-10-30" "Wunix" "System Administration Commands"
.SH NAME
useradd \- create a new user account and home directory
.SH SYNOPSIS
\fBuseradd\fR [\fIOPTION\fR]... \fIUSERNAME\fR
.SH DESCRIPTION
\fBuseradd\fR is a low-level utility for adding new user accounts to the system. It creates entries in \fB/etc/passwd\fR, \fB/etc/shadow\fR, and \fB/etc/group\fR, allocates a unique UID and GID, and creates the user's home directory.
.SH OPTIONS
.TP
\fB\-m\fR, \fB\-\-create\-home\fR
Create the user's home directory (enabled by default).
.TP
\fB\-d\fR, \fB\-\-home\-dir\fR \fIDIR\fR
Explicitly specify the home directory path for the new user.
.TP
\fB\-s\fR, \fB\-\-shell\fR \fISHELL\fR
Login shell for the new user (default is \fB/bin/wsh\fR).
.TP
\fB\-u\fR, \fB\-\-uid\fR \fIUID\fR
Explicit numerical user ID.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.SH EXAMPLES
.TP
useradd developer
.TP
useradd -s /bin/wsh -d /home/admin admin
.SH "SEE ALSO"
\fBuserdel\fR(8), \fBpasswd\fR(1), \fBchown\fR(1), \fBlogin\fR(1)
"#,
        roff_ru: r#".TH USERADD 8 "2026-10-30" "Wunix" "Команды системного администрирования"
.SH ИМЯ
useradd \- создание новой учетной записи пользователя и домашнего каталога
.SH СИНТАКСИС
\fBuseradd\fR [\fIОПЦИИ\fR]... \fIИМЯ_ПОЛЬЗОВАТЕЛЯ\fR
.SH ОПИСАНИЕ
\fBuseradd\fR — утилита системного администрирования для регистрации новых пользователей. Добавляет записи в \fB/etc/passwd\fR, \fB/etc/shadow\fR и \fB/etc/group\fR, автоматически выделяет свободный UID и GID и создает персональный домашний каталог.
.SH ОПЦИИ
.TP
\fB\-m\fR, \fB\-\-create\-home\fR
Создать домашний каталог пользователя (по умолчанию включено).
.TP
\fB\-d\fR, \fB\-\-home\-dir\fR \fIКАТАЛОГ\fR
Задать явный путь к домашнему каталогу.
.TP
\fB\-s\fR, \fB\-\-shell\fR \fIШЕЛЛ\fR
Указать командную оболочку (по умолчанию \fB/bin/wsh\fR).
.TP
\fB\-u\fR, \fB\-\-uid\fR \fIUID\fR
Указать точный числовой идентификатор пользователя UID.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH ПРИМЕРЫ
.TP
useradd developer
.TP
useradd -s /bin/wsh -d /home/admin admin
.SH "СМ. ТАКЖЕ"
\fBuserdel\fR(8), \fBpasswd\fR(1), \fBchown\fR(1), \fBlogin\fR(1)
"#,
    },
    ManPageEntry {
        name: "userdel",
        section: 8,
        desc_en: "delete a user account and related files",
        desc_ru: "удаление учетной записи пользователя и связанных файлов",
        roff_en: r#".TH USERDEL 8 "2026-10-30" "Wunix" "System Administration Commands"
.SH NAME
userdel \- delete a user account and related files
.SH SYNOPSIS
\fBuserdel\fR [\fIOPTION\fR]... \fIUSERNAME\fR
.SH DESCRIPTION
\fBuserdel\fR modifies the system account files, deleting all entries that refer to \fIUSERNAME\fR from \fB/etc/passwd\fR, \fB/etc/shadow\fR, and \fB/etc/group\fR. With \fB\-r\fR, it also removes the user's home directory.
.SH OPTIONS
.TP
\fB\-r\fR, \fB\-\-remove\fR
Remove home directory and mail spool associated with the user account.
.TP
\fB\-h\fR, \fB\-\-help\fR
Display this help and exit.
.SH EXAMPLES
.TP
userdel guest
.TP
userdel -r olduser
.SH "SEE ALSO"
\fBuseradd\fR(8), \fBpasswd\fR(1), \fBrm\fR(1)
"#,
        roff_ru: r#".TH USERDEL 8 "2026-10-30" "Wunix" "Команды системного администрирования"
.SH ИМЯ
userdel \- удаление учетной записи пользователя и связанных файлов
.SH СИНТАКСИС
\fBuserdel\fR [\fIОПЦИИ\fR]... \fIИМЯ_ПОЛЬЗОВАТЕЛЯ\fR
.SH ОПИСАНИЕ
\fBuserdel\fR удаляет учетную запись \fIИМЯ_ПОЛЬЗОВАТЕЛЯ\fR из конфигурационных файлов \fB/etc/passwd\fR, \fB/etc/shadow\fR и групп \fB/etc/group\fR. При указании флага \fB\-r\fR также полностью удаляется домашний каталог пользователя.
.SH ОПЦИИ
.TP
\fB\-r\fR, \fB\-\-remove\fR
Удалить домашний каталог и почтовый ящик пользователя.
.TP
\fB\-h\fR, \fB\-\-help\fR
Показать эту справку и выйти.
.SH ПРИМЕРЫ
.TP
userdel guest
.TP
userdel -r olduser
.SH "СМ. ТАКЖЕ"
\fBuseradd\fR(8), \fBpasswd\fR(1), \fBrm\fR(1)
"#,
    },
];

pub struct ManDatabase;

impl ManDatabase {
    pub const SEARCH_PATHS: &'static [&'static str] = &[
        "/usr/share/man",
        "/usr/local/share/man",
        "/share/man",
        "/etc/man",
    ];

    pub fn find_page(name: &str, section: Option<u8>, lang_is_ru: bool) -> Option<FoundPage> {
        // 1. Поиск на файловой системе
        if let Some(fs_page) = Self::find_on_filesystem(name, section, lang_is_ru) {
            return Some(fs_page);
        }

        // 2. Поиск во встроенной базе данных
        Self::find_in_builtin(name, section, lang_is_ru)
    }

    pub fn find_all_matching(name: &str, lang_is_ru: bool) -> Vec<FoundPage> {
        let mut results = Vec::new();
        for sec in 1..=8 {
            if let Some(p) = Self::find_page(name, Some(sec), lang_is_ru) {
                // Избегаем дубликатов секций
                if !results.iter().any(|existing: &FoundPage| existing.section == p.section) {
                    results.push(p);
                }
            }
        }
        results
    }

    pub fn find_in_builtin(name: &str, section: Option<u8>, lang_is_ru: bool) -> Option<FoundPage> {
        let search_order: &[u8] = match section {
            Some(s) => core::slice::from_ref(match s {
                1 => &1, 2 => &2, 3 => &3, 4 => &4, 5 => &5, 6 => &6, 7 => &7, 8 => &8,
                _ => return None,
            }),
            None => &[1, 8, 2, 3, 4, 5, 6, 7],
        };

        for &sec in search_order {
            for entry in BUILTIN_PAGES {
                if entry.name == name && entry.section == sec {
                    let content = if lang_is_ru {
                        entry.roff_ru
                    } else {
                        entry.roff_en
                    };
                    let path = alloc::format!("/usr/share/man/man{}/{}.{}", sec, name, sec);
                    return Some(FoundPage {
                        name: String::from(entry.name),
                        section: entry.section,
                        path,
                        roff_content: String::from(content),
                    });
                }
            }
        }

        None
    }

    pub fn find_on_filesystem(name: &str, section: Option<u8>, lang_is_ru: bool) -> Option<FoundPage> {
        // Если передан прямой путь к файлу (содержит слэш '/')
        if name.contains('/') {
            if let Some(content) = Self::read_fs_file(name) {
                return Some(FoundPage {
                    name: String::from(name),
                    section: section.unwrap_or(1),
                    path: String::from(name),
                    roff_content: content,
                });
            }
            return None;
        }

        let sections_to_try: Vec<u8> = match section {
            Some(s) => vec![s],
            None => vec![1, 8, 2, 3, 4, 5, 6, 7],
        };

        for base in Self::SEARCH_PATHS {
            for &sec in &sections_to_try {
                // 1) Попытка с активным языком (ru)
                if lang_is_ru {
                    let path = alloc::format!("{}/ru/man{}/{}.{}", base, sec, name, sec);
                    if let Some(content) = Self::read_fs_file(&path) {
                        return Some(FoundPage {
                            name: String::from(name),
                            section: sec,
                            path,
                            roff_content: content,
                        });
                    }
                }

                // 2) Попытка со стандартным путем
                let path = alloc::format!("{}/man{}/{}.{}", base, sec, name, sec);
                if let Some(content) = Self::read_fs_file(&path) {
                    return Some(FoundPage {
                        name: String::from(name),
                        section: sec,
                        path,
                        roff_content: content,
                    });
                }
            }
        }

        None
    }

    pub fn apropos(keyword: &str, lang_is_ru: bool) -> Vec<AproposMatch> {
        let kw_lower = keyword.to_lowercase();
        let mut results = Vec::new();

        for entry in BUILTIN_PAGES {
            let desc = if lang_is_ru { entry.desc_ru } else { entry.desc_en };
            let matches_name = entry.name.to_lowercase().contains(&kw_lower);
            let matches_desc = desc.to_lowercase().contains(&kw_lower);

            if matches_name || matches_desc {
                results.push(AproposMatch {
                    name: String::from(entry.name),
                    section: entry.section,
                    description: String::from(desc),
                });
            }
        }

        results
    }

    pub fn whatis(name: &str, lang_is_ru: bool) -> Vec<AproposMatch> {
        let name_lower = name.to_lowercase();
        let mut results = Vec::new();

        for entry in BUILTIN_PAGES {
            if entry.name.to_lowercase() == name_lower {
                let desc = if lang_is_ru { entry.desc_ru } else { entry.desc_en };
                results.push(AproposMatch {
                    name: String::from(entry.name),
                    section: entry.section,
                    description: String::from(desc),
                });
            }
        }

        results
    }

    fn read_fs_file(path: &str) -> Option<String> {
        #[cfg(test)]
        {
            extern crate std;
            std::fs::read_to_string(path).ok()
        }
        #[cfg(not(test))]
        {
            let mut null_term = [0u8; 256];
            let bytes = path.as_bytes();
            if bytes.len() >= null_term.len() {
                return None;
            }
            null_term[..bytes.len()].copy_from_slice(bytes);
            null_term[bytes.len()] = 0;

            let fd = libwunix::open(&null_term[..bytes.len() + 1], 0, 0);
            if fd < 0 {
                return None;
            }

            let mut content = Vec::new();
            let mut chunk = [0u8; 1024];
            loop {
                let n = libwunix::read(fd as u64, &mut chunk);
                if n == 0 || (n as i64) < 0 {
                    break;
                }
                let count = (n as usize).min(chunk.len());
                content.extend_from_slice(&chunk[..count]);
            }
            let _ = libwunix::close(fd as u64);

            String::from_utf8(content).ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_lookup() {
        let page_en = ManDatabase::find_in_builtin("cat", Some(1), false).unwrap();
        assert_eq!(page_en.name, "cat");
        assert_eq!(page_en.section, 1);
        assert!(page_en.roff_content.contains("concatenate files"));

        let page_ru = ManDatabase::find_in_builtin("cat", Some(1), true).unwrap();
        assert_eq!(page_ru.name, "cat");
        assert_eq!(page_ru.section, 1);
        assert!(page_ru.roff_content.contains("объединение"));
    }

    #[test]
    fn test_apropos_search() {
        let matches = ManDatabase::apropos("shell", false);
        assert!(matches.iter().any(|m| m.name == "wsh"));

        let matches_ru = ManDatabase::apropos("оболочка", true);
        assert!(matches_ru.iter().any(|m| m.name == "wsh"));
    }

    #[test]
    fn test_whatis_search() {
        let matches = ManDatabase::whatis("cat", false);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "cat");
        assert!(matches[0].description.contains("concatenate"));
    }

    #[test]
    fn test_all_programs_present_in_builtin() {
        let all_progs = [
            "cat", "chmod", "chown", "clear", "curl", "date", "echo", "fbset", "fdisk", "free",
            "host", "hwclock", "id", "init", "ip", "ln", "login", "ls", "lsblk", "lsdev",
            "lshw", "lsmod", "lspci", "man", "mkdir", "modinfo", "mount", "passwd", "ping", "pwd",
            "readlink", "reboot", "rm", "rmdir", "shutdown", "ss", "su", "sudo", "sudoadd", "system", "time",
            "timedatectl", "ntpdate", "timesyncd", "top", "touch", "umount", "uname", "useradd", "userdel", "whoami", "wsh",
        ];
        for prog in all_progs {
            assert!(
                ManDatabase::find_in_builtin(prog, None, false).is_some(),
                "Program '{}' missing from builtin man database (EN)",
                prog
            );
            assert!(
                ManDatabase::find_in_builtin(prog, None, true).is_some(),
                "Program '{}' missing from builtin man database (RU)",
                prog
            );
        }
    }
}
