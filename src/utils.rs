use anyhow::{Context, Result};
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use human_bytes::human_bytes;
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

use path_absolutize::Absolutize;

use crate::app::{self, APP_NAME};

/// Formats a given path for user-friendly display.
///
/// If the path is within the user's home directory, it replaces the home path with `~`.
/// Otherwise, it returns the absolute path.
///
/// # Arguments
/// * `p` - A path-like object that implements `AsRef<Path>`.
///
/// # Returns
/// A `String` representing the formatted display path.
///
/// # Example
/// ```
/// use std::path::Path;
/// let home = utils::get_user_home_dir().unwrap();
/// let display_path = format_path_for_display(home.join("Documents"));
/// assert_eq!(display_path, "~/Documents");
/// ```
pub fn format_path_for_display<P: AsRef<Path>>(p: P) -> String {
    let p = p.as_ref();
    user_home_dir().map_or_else(
        || absolute_path_as_string(p),
        |home_dir| {
            let abs_path = absolute_path_as_string(p);
            abs_path.replace(&absolute_path_as_string(home_dir), "~")
        },
    )
}

pub fn config_dir() -> PathBuf {
    match dirs::config_dir() {
        Some(data_dir) => data_dir.join(APP_NAME),
        None => PathBuf::new().join(".").join(APP_NAME).join("config"),
    }
}

/// Retrieves the data directory path for the project.
///
/// This function uses the `simple_home_dir` crate to determine the user's home directory
/// and constructs the path to the project's data directory within it. If the home directory
/// is not available, it falls back to a relative path based on the current directory.
///
/// # Returns
///
/// Returns a `PathBuf` representing the data directory path for the project.
///
/// # Note
///
/// Ensure that the `PROJECT_NAME` constant is correctly set before calling this function.
/// The data directory is typically used for storing application-specific data files.
pub fn data_dir() -> PathBuf {
    match dirs::data_dir() {
        Some(data_dir) => data_dir.join(app::APP_NAME),
        None => PathBuf::new().join(".").join(app::APP_NAME).join("data"),
    }
}

/// Get the users home dir.
///
/// # Returns
///
/// If a users home dir exists -> Some([`PathBuf`]) containing the path to the users home dir,
/// otherwise [None]
pub fn user_home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

/// Initialize the application logging
pub fn initialize_logging() -> Result<()> {
    init_logger()?;
    log::info!("[{APP_NAME}] => Start application",);
    log::info!("[{APP_NAME}] => Version   : {}", env!("CARGO_PKG_VERSION"));
    log::info!("[{APP_NAME}] => Running on: {}", os_info::get());
    Ok(())
}

/// Initializes the log writer for debugging purposes.
///
/// This function creates a debug log file with a name containing the project name and
/// a timestamp formatted in the "YYYY-MM-DD_HH_MM_SS" format. The log file is stored
/// in the project's data directory. The logging level is set to debug,
/// and the logs which was created by the `log` crate are
/// written to the debug log file using the `simplelog` crate.
fn init_logger() -> Result<()> {
    let log_file =
        initialize_log_file().with_context(|| "Failed to create application log file")?;
    let config = simplelog::ConfigBuilder::new()
        .set_time_format_rfc3339()
        .build();
    simplelog::WriteLogger::init(simplelog::LevelFilter::Debug, config, log_file)?;
    Ok(())
}

/// Create the log file. If it already exists, make sure it's not over a max
/// size. If it is, move it to a backup path and nuke whatever might be in the
/// backup path.
fn initialize_log_file() -> anyhow::Result<File> {
    const MAX_FILE_SIZE: u64 = 1000 * 1000; // 1MB
    let path = log_file();

    if fs::metadata(&path).is_ok_and(|metadata| metadata.len() > MAX_FILE_SIZE) {
        // Rename new->old, overwriting old. If that fails, just delete new so
        // it doesn't grow indefinitely. Failure shouldn't stop us from logging
        // though
        let _ = fs::rename(&path, log_file_old()).or_else(|_| fs::remove_file(&path));
    }

    let log_file = OpenOptions::new().create(true).append(true).open(path)?;
    Ok(log_file)
}

pub fn crash_report_file() -> PathBuf {
    let crash_report_file_name = format!(
        "{}-Crash-Report_{}.log",
        app::APP_NAME,
        chrono::Local::now().format("%Y-%m-%dT%H_%M_%S")
    );
    data_dir().join(crash_report_file_name)
}
/// Get the path to the primary log file. **Parent direct may not exist yet,**
/// caller must create it.
pub fn log_file() -> PathBuf {
    data_dir().join(format!("{}.log", APP_NAME))
}

/// Get the path to the backup log file **Parent direct may not exist yet,**
/// caller must create it.
pub fn log_file_old() -> PathBuf {
    data_dir().join(format!("{}.log.old", APP_NAME))
}

/// Creates the application's data directory.
///
/// This function creates the necessary data directories,
/// if they do not exist.
/// # Returns
///
/// Returns a `Result<()>` if the operation succeeds, or an
/// `Err` variant with an associated `std::io::Error` if any error occurs during the
/// process.
pub fn create_data_dir() -> Result<()> {
    let directory = data_dir();
    std::fs::create_dir_all(directory.clone())
        .with_context(|| "Failed to create application data directory")?;
    Ok(())
}

/// Return the passed path as an absolute path, otherwise the passed path
pub fn absolute_path_as_string<P: AsRef<Path>>(path: P) -> String {
    let path = path.as_ref();
    match path.absolutize() {
        Ok(absolute_path) => absolute_path.display().to_string(),
        Err(_) => path.display().to_string(),
    }
}

/// Converts a given size in bytes into a human-readable format.
///
/// # Arguments
///
/// * `bytes` - The size in bytes to be converted into a human-readable format.
///
/// # Returns
///
/// A string representing the human-readable format of the given size.
///
/// # Examples
///
/// ```
///  let size_in_bytes: u64 = 2000;
///  let readable_size = convert_bytes_to_human_readable(size_in_bytes);
///  assert_eq!("2 MB".to_string(), readable_size);
/// ```
pub fn convert_bytes_to_human_readable(bytes: u64) -> String {
    human_bytes(bytes as f64).to_string()
}

/// Extends the default ``clap --version`` with a custom application version message
pub fn version() -> String {
    let authors = env!("CARGO_PKG_AUTHORS").replace(":", ", ");
    let version = env!("CARGO_PKG_VERSION");
    let repo = env!("CARGO_PKG_REPOSITORY");

    let config_dir = format_path_for_display(absolute_path_as_string(config_dir()));
    let data_dir = format_path_for_display(absolute_path_as_string(data_dir()));

    format!(
        "\
    --- developed with ♥ in Rust
    Authors          : {authors}
    Version          : {version}
    Repository       : {repo}

    Config directory : {config_dir}
    Data directory   : {data_dir}
    "
    )
}

/// This function checks if the length of the input string exceeds the specified maximum length.
/// If it does, the string is truncated such that the resulting string (including the appended
/// ellipsis "...") does not exceed the maximum length. If the string length is within the limit,
/// it is returned unchanged.<br>
/// _*Note*_
/// This function count the chars from the input string slice and compare this value with the given maximum length.
/// It does not use the bytes length of the given string slice.
///
/// # Arguments
///
/// * `data` - A string slice (`&str`) that may need to be truncated.
/// * `max_length` - The maximum allowed length for the string, including the ellipsis.
///
/// # Returns
///
/// A new `String` that is either the truncated version of the input string with ellipsis appended,
/// or the original string if it is within the allowed length.
pub fn reduce_string_and_fill_with_dots(data: &str, max_length: usize) -> String {
    if data.chars().count() > max_length {
        let reduce_string = &data[..max_length - 3];
        // Append the three dots
        format!("{}...", reduce_string)
    } else {
        data.to_string()
    }
}

/// Calculates the percentage of the `numerator` with respect to the `denominator`.
///
/// This function takes two unsigned 64-bit integers, `numerator` and `denominator`, and computes
/// the percentage of the `numerator` relative to the `denominator`. The result is returned as a
/// 64-bit floating-point number (`f64`). If the `denominator` is zero, the function will always return 0.0.
///
/// # Parameters
///
/// - `numerator`: The part value for which the percentage needs to be calculated.
/// - `denominator`: The whole value against which the percentage is calculated.
///
/// # Returns
///
/// - `f64`: The percentage value of `numerator` with respect to `denominator`.
pub fn calculate_percentage_f64(numerator: f64, denominator: f64) -> f64 {
    if denominator > 0.0 {
        (numerator / denominator) * 100.0
    } else {
        0.0
    }
}

/// Converts a `SystemTime` to a human-readable format e.g. 2025-01-14 11:30:45
pub fn system_time_to_readable(time: &std::time::SystemTime) -> String {
    // Convert SystemTime to DateTime<Local>
    let datetime: chrono::DateTime<chrono::Local> = time.to_owned().into();
    // Format the datetime
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Converts a crossterm KeyEvent into a human-readable string
pub fn key_event_to_string(event: KeyEvent) -> String {
    let modifiers_str = modifiers_to_string(event.modifiers);

    match event.code {
        KeyCode::Char(c) => {
            if event.modifiers == KeyModifiers::CONTROL {
                // Sonderfall: Nur Control -> Großbuchstabe
                format!("Ctrl + {}", c.to_ascii_uppercase())
            } else if event.modifiers == KeyModifiers::SHIFT {
                // Shift + Char -> Nur Char (Groß-/Sonderzeichen bereits berücksichtigt)
                c.to_string()
            } else if event.modifiers != KeyModifiers::NONE {
                // Andere Modifier -> Modifier + Char
                format!("{} + {}", modifiers_str, c)
            } else {
                // Kein Modifier -> Nur Char
                c.to_string()
            }
        }
        KeyCode::Left => format_arrow_key("Left Arrow", &modifiers_str),
        KeyCode::Right => format_arrow_key("Right Arrow", &modifiers_str),
        KeyCode::Up => format_arrow_key("Up Arrow", &modifiers_str),
        KeyCode::Down => format_arrow_key("Down Arrow", &modifiers_str),
        _ => {
            if event.modifiers != KeyModifiers::NONE {
                format!("{} + {}", modifiers_str, event.code)
            } else {
                event.code.to_string()
            }
        }
    }
}

/// Helper function for formatting arrow keys with optional modifiers
fn format_arrow_key(key_name: &str, modifiers_str: &str) -> String {
    if modifiers_str.is_empty() {
        key_name.to_string()
    } else {
        format!("{} + {}", modifiers_str, key_name)
    }
}

/// Converts KeyModifiers into a human-readable string separated by " + "
fn modifiers_to_string(modifiers: KeyModifiers) -> String {
    let mut parts = Vec::new();

    if modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if modifiers.contains(KeyModifiers::ALT) {
        parts.push("Alt");
    }
    if modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("Shift");
    }

    parts.join(" + ")
}

/// Computes the length of the given text in characters.
///
/// # Arguments
///
/// * `value` - A string slice (`&str`) representing the text whose length is to be computed.
///
/// # Returns
///
/// * A `u16` representing the number of characters in the input text.
///
/// # Example
///
/// ```
/// let length = compute_text_length("Hello, world!");
/// assert_eq!(length, 13);
/// ```
pub fn compute_text_length(value: &str) -> u16 {
    value.chars().count() as u16
}

pub fn copy_to_clipboard(value: &str) -> Result<()> {
    let mut clipboard = ClipboardContext::new()
        .map_err(|e| anyhow::anyhow!(e).context("Failed to access the clipboard"))?;

    clipboard
        .set_contents(value.to_string())
        .map_err(|e| anyhow::anyhow!(e).context("Failed to SET content to clipboard"))?;

    let content = clipboard
        .get_contents()
        .map_err(|e| anyhow::anyhow!(e).context("Failed to GET content from clipboard"))?;

    // check if the current clipboard content equal to the given value
    if content != value {
        Err(anyhow::anyhow!(
            "Failed to copy content: [{}] to clipboard",
            value
        ))
    } else {
        Ok(())
    }
}

pub fn paste_from_clipboard() -> Result<String> {
    let mut clipboard = ClipboardContext::new()
        .map_err(|e| anyhow::anyhow!(e).context("Failed to access the clipboard"))?;
    let content = clipboard
        .get_contents()
        .map_err(|e| anyhow::anyhow!(e).context("Failed to GET content from clipboard"))?;
    Ok(content)
}

pub fn extract_part(text: &str, search: &str) -> Option<String> {
    if let Some(start) = text.find(search) {
        text.get(start..start + search.len()).map(|s| s.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests_key_event_to_string {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    /// Helper to create KeyEvent easily
    fn key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    #[test]
    fn test_char_without_modifiers() {
        // Char without modifiers -> Just the char
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Char('a'), KeyModifiers::NONE)),
            "a"
        );
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Char('1'), KeyModifiers::NONE)),
            "1"
        );
    }

    #[test]
    fn test_char_with_shift() {
        // Char with Shift -> Just the char (uppercase or special character)
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Char('a'), KeyModifiers::SHIFT)),
            "a"
        ); // Shift + a -> a (because char is handled externally for case)
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Char('A'), KeyModifiers::SHIFT)),
            "A"
        ); // Shift + A -> A
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Char('?'), KeyModifiers::SHIFT)),
            "?"
        ); // Shift + ? -> ?
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Char('1'), KeyModifiers::SHIFT)),
            "1"
        ); // Shift + 1 -> 1
    }

    #[test]
    fn test_char_with_control_only() {
        // Char with Control -> "Ctrl + <UPPERCASE_CHAR>"
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Char('a'), KeyModifiers::CONTROL)),
            "Ctrl + A"
        );
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Char('z'), KeyModifiers::CONTROL)),
            "Ctrl + Z"
        );
    }

    #[test]
    fn test_char_with_control_and_shift() {
        // Char with Control + Shift -> "Ctrl + Shift + <char>" (char stays lowercase)
        assert_eq!(
            key_event_to_string(key_event(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT
            )),
            "Ctrl + Shift + a"
        );
    }

    #[test]
    fn test_char_with_alt() {
        // Char with Alt -> "Alt + <char>"
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Char('a'), KeyModifiers::ALT)),
            "Alt + a"
        );
    }

    #[test]
    fn test_char_with_multiple_modifiers() {
        // Char with Control + Alt -> "Ctrl + Alt + <char>"
        assert_eq!(
            key_event_to_string(key_event(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )),
            "Ctrl + Alt + a"
        );

        // Char with Control + Alt + Shift -> "Ctrl + Alt + Shift + <char>"
        assert_eq!(
            key_event_to_string(key_event(
                KeyCode::Char('b'),
                KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT
            )),
            "Ctrl + Alt + Shift + b"
        );
    }

    #[test]
    fn test_arrow_keys_without_modifiers() {
        // Arrow keys without modifiers -> Just arrow name
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Left, KeyModifiers::NONE)),
            "Left Arrow"
        );
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Right, KeyModifiers::NONE)),
            "Right Arrow"
        );
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Up, KeyModifiers::NONE)),
            "Up Arrow"
        );
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Down, KeyModifiers::NONE)),
            "Down Arrow"
        );
    }

    #[test]
    fn test_arrow_keys_with_modifiers() {
        // Arrow keys with modifiers -> "<Modifiers> + Arrow"
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Left, KeyModifiers::CONTROL)),
            "Ctrl + Left Arrow"
        );
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Right, KeyModifiers::ALT)),
            "Alt + Right Arrow"
        );
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Up, KeyModifiers::SHIFT)),
            "Shift + Up Arrow"
        );
        assert_eq!(
            key_event_to_string(key_event(
                KeyCode::Down,
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )),
            "Ctrl + Alt + Down Arrow"
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn test_enter_key() {
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Enter, KeyModifiers::NONE)),
            "Enter"
        );

        // Enter key with modifiers -> "<Modifiers> + Return"
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Enter, KeyModifiers::CONTROL)),
            "Ctrl + Enter"
        );
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Enter, KeyModifiers::ALT)),
            "Alt + Enter"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_enter_key_mac_os() {
        // Enter key -> Should display "Return" under macOS conventions
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Enter, KeyModifiers::NONE)),
            "Return"
        );

        // Enter key with modifiers -> "<Modifiers> + Return"
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Enter, KeyModifiers::CONTROL)),
            "Ctrl + Return"
        );
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Enter, KeyModifiers::ALT)),
            "Alt + Return"
        );
    }

    #[test]
    fn test_other_keys_with_modifiers() {
        // Other keys (e.g., Tab, Backspace) with modifiers -> "<Modifiers> + Key"
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Tab, KeyModifiers::CONTROL)),
            "Ctrl + Tab"
        );

        #[cfg(not(target_os = "macos"))]
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Backspace, KeyModifiers::ALT)),
            "Alt + Backspace"
        );

        #[cfg(target_os = "macos")]
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Backspace, KeyModifiers::ALT)),
            "Alt + Delete"
        );
    }

    #[test]
    fn test_no_modifiers_other_keys() {
        // Other keys without modifiers -> Just key name
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Tab, KeyModifiers::NONE)),
            "Tab"
        );
        #[cfg(not(target_os = "macos"))]
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Backspace, KeyModifiers::NONE)),
            "Backspace"
        );
        #[cfg(target_os = "macos")]
        assert_eq!(
            key_event_to_string(key_event(KeyCode::Backspace, KeyModifiers::NONE)),
            "Delete"
        );
    }
}

#[cfg(test)]
mod tests_common {
    use super::*;

    #[test]
    fn test_extract_part_found() {
        let text = String::from("Hello World");
        let search = "World";
        assert_eq!(extract_part(&text, search), Some("World".into()));
    }

    #[test]
    fn test_extract_part_not_found() {
        let text = String::from("Hello World");
        let search = "Rust";
        assert_eq!(extract_part(&text, search), None);
    }

    #[test]
    fn test_extract_part_case_sensitive() {
        let text = String::from("Hello World");
        let search = "world"; // Different case
        assert_eq!(extract_part(&text, search), None);
    }

    #[test]
    fn test_extract_part_partial_match() {
        let text = String::from("Hello World");
        let search = "Wor";
        assert_eq!(extract_part(&text, search), Some("Wor".into()));
    }

    #[test]
    fn test_extract_part_full_string() {
        let text = String::from("Hello World");
        let search = "Hello World";
        assert_eq!(extract_part(&text, search), Some("Hello World".into()));
    }

    #[test]
    fn test_extract_part_empty_search() {
        let text = String::from("Hello World");
        let search = "";
        assert_eq!(extract_part(&text, search), Some("".into()));
    }

    #[test]
    fn test_extract_part_empty_text() {
        let text = String::from("");
        let search = "Hello";
        assert_eq!(extract_part(&text, search), None);
    }

    #[test]
    fn test_extract_part_unicode() {
        let text = String::from("Здравствуйте мир");
        let search = "мир"; // Unicode substring
        assert_eq!(extract_part(&text, search), Some("мир".into()));
    }

    #[test]
    fn test_reduce_string() {
        let input = "A".repeat(122);

        let expected = format!("{}...", "A".repeat(17));

        assert_eq!(reduce_string_and_fill_with_dots(&input, 20), expected);
    }

    #[test]
    fn test_calculate_percentage_f64_1() {
        let numerator = 50.0_f64;
        let denominator = 100.0_f64;
        let expected = 50_f64;

        assert_eq!(calculate_percentage_f64(numerator, denominator), expected);
    }

    #[test]
    fn test_calculate_percentage_f64_2() {
        let numerator = 43.0_f64;
        let denominator = 699.0_f64;
        let expected = 6.151645207439199_f64;

        assert_eq!(calculate_percentage_f64(numerator, denominator), expected);
    }

    #[test]
    fn test_calculate_percentage_f64_3() {
        let numerator = 43.0_f64;
        let denominator = 0.0_f64;
        let expected = 0_f64;

        assert_eq!(calculate_percentage_f64(numerator, denominator), expected);
    }
}
