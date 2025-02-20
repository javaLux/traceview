#![allow(dead_code)]
use crate::app::AppContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Keys {
    F1,
    F2,
    F5,
    F12,
    Enter,
    Esc,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Char(char),
    AnyChar,
    Tab,
}

impl std::fmt::Display for Keys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Keys::F1 => write!(f, "F1"),
            Keys::F2 => write!(f, "F2"),
            Keys::F5 => write!(f, "F5"),
            Keys::F12 => write!(f, "F12"),
            Keys::Enter => write!(f, "Enter"),
            Keys::Esc => write!(f, "Esc"),
            Keys::Backspace => write!(f, "Backspace"),
            Keys::Delete => write!(f, "Delete"),
            Keys::Up => write!(f, "Up Arrow"),
            Keys::Down => write!(f, "Down Arrow"),
            Keys::Left => write!(f, "Left Arrow"),
            Keys::Right => write!(f, "Right Arrow"),
            Keys::PageUp => write!(f, "PageUp"),
            Keys::PageDown => write!(f, "PageDown"),
            Keys::Char(c) => write!(f, "{}", c),
            Keys::AnyChar => write!(f, "Any Char"),
            Keys::Tab => write!(f, "Tab"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyStroke {
    key_code: Keys,
    modifiers: crossterm::event::KeyModifiers,
}

impl std::fmt::Display for KeyStroke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.modifiers.is_empty() {
            write!(f, "{}", self.key_code)
        } else {
            let modifier: String = self
                .modifiers
                .iter()
                .map(|modifier| match modifier {
                    crossterm::event::KeyModifiers::CONTROL => "Ctrl".to_string(),
                    crossterm::event::KeyModifiers::ALT => "Alt".to_string(),
                    crossterm::event::KeyModifiers::SHIFT => "Shift".to_string(),
                    _ => modifier.to_string(),
                })
                .collect::<Vec<String>>()
                .join("");
            write!(f, "{} + {}", modifier, self.key_code)
        }
    }
}

impl KeyStroke {
    const fn new(key_code: Keys, modifiers: crossterm::event::KeyModifiers) -> Self {
        Self {
            key_code,
            modifiers,
        }
    }

    fn matches(
        &self,
        key_code: &crossterm::event::KeyCode,
        modifier: &crossterm::event::KeyModifiers,
    ) -> bool {
        let match_key_code = match (&self.key_code, key_code) {
            // Case-insensitive comparison when Control is pressed
            (Keys::Char(expected), crossterm::event::KeyCode::Char(actual)) => {
                if modifier.contains(crossterm::event::KeyModifiers::CONTROL) {
                    expected.eq_ignore_ascii_case(actual)
                } else {
                    expected == actual
                }
            }
            (Keys::AnyChar, crossterm::event::KeyCode::Char(_)) => true,
            (Keys::F1, crossterm::event::KeyCode::F(1)) => true,
            (Keys::F2, crossterm::event::KeyCode::F(2)) => true,
            (Keys::F5, crossterm::event::KeyCode::F(5)) => true,
            (Keys::F12, crossterm::event::KeyCode::F(12)) => true,
            (Keys::Enter, crossterm::event::KeyCode::Enter)
            | (Keys::Esc, crossterm::event::KeyCode::Esc)
            | (Keys::Backspace, crossterm::event::KeyCode::Backspace)
            | (Keys::Delete, crossterm::event::KeyCode::Delete)
            | (Keys::Up, crossterm::event::KeyCode::Up)
            | (Keys::Down, crossterm::event::KeyCode::Down)
            | (Keys::Left, crossterm::event::KeyCode::Left)
            | (Keys::Right, crossterm::event::KeyCode::Right)
            | (Keys::PageUp, crossterm::event::KeyCode::PageUp)
            | (Keys::PageDown, crossterm::event::KeyCode::PageDown) => true,
            (Keys::Tab, crossterm::event::KeyCode::Tab) => true,
            _ => false,
        };

        match_key_code && self.modifiers == *modifier
    }

    pub fn matches_event(&self, event: &crossterm::event::KeyEvent) -> bool {
        self.matches(&event.code, &event.modifiers)
    }
}

/// Mapping to keep track about a pressed key and the associated command
/// description dependent on the given app context
#[derive(Debug, Clone)]
struct CommandDesc {
    desc: &'static str,
    contexts: &'static [AppContext],
}

/// Represents a specific key binding for the help page
#[derive(Debug, Clone)]
pub struct KeyBinding {
    /// Main key
    key_stroke: KeyStroke,
    /// Alternate key
    alt: Option<KeyStroke>,
    /// key description for the help page
    help_desc: &'static str,
    /// Help page contexts, used to display in the help row
    help_contexts: &'static [AppContext],
    /// Mapping between the command description and the associated app context
    /// Used in the footer widget to display a description for each keystroke
    command_desc: Option<&'static [CommandDesc]>,
}

pub const DEFAULT_KEY_BINDING: [KeyBinding; 23] = [
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::F1, crossterm::event::KeyModifiers::NONE),
        alt: None,
        help_desc: "Show the Help-Page",
        help_contexts: &[AppContext::All],
        command_desc: Some(&[CommandDesc {
            desc: "Show help page",
            contexts: &[
                AppContext::Explorer,
                AppContext::Search,
                AppContext::Results,
            ],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::F2, crossterm::event::KeyModifiers::NONE),
        alt: None,
        help_desc: "Show the About-Page",
        help_contexts: &[AppContext::All],
        command_desc: Some(&[CommandDesc {
            desc: "Show about page",
            contexts: &[
                AppContext::Explorer,
                AppContext::Search,
                AppContext::Results,
            ],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::F5, crossterm::event::KeyModifiers::NONE),
        alt: None,
        help_desc: "Refresh the current working directory",
        help_contexts: &[AppContext::Explorer],
        command_desc: Some(&[CommandDesc {
            desc: "Refresh dir",
            contexts: &[AppContext::Explorer],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::F12, crossterm::event::KeyModifiers::NONE),
        alt: None,
        help_desc: "Export search results as JSON, default location is the app data directory",
        help_contexts: &[AppContext::Results],
        command_desc: Some(&[CommandDesc {
            desc: "Export as JSON",
            contexts: &[AppContext::Results],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Enter, crossterm::event::KeyModifiers::NONE),
        alt: None,
        help_desc: "Open directory, if any | Submit search",
        help_contexts: &[AppContext::Explorer, AppContext::Search],
        command_desc: Some(&[
            CommandDesc {
                desc: "Change dir",
                contexts: &[AppContext::Explorer],
            },
            CommandDesc {
                desc: "Submit search",
                contexts: &[AppContext::Search],
            },
        ]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Backspace, crossterm::event::KeyModifiers::NONE),
        alt: None,
        help_desc: "Go to parent directory, if any | To delete search input",
        help_contexts: &[AppContext::Explorer, AppContext::Search],
        command_desc: Some(&[
            CommandDesc {
                desc: "Change dir",
                contexts: &[AppContext::Explorer],
            },
            CommandDesc {
                desc: " ",
                contexts: &[AppContext::Search],
            },
        ]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Delete, crossterm::event::KeyModifiers::NONE),
        alt: None,
        help_desc: "To delete search input",
        help_contexts: &[AppContext::Search],
        command_desc: Some(&[CommandDesc {
            desc: " ",
            contexts: &[AppContext::Search],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Tab, crossterm::event::KeyModifiers::NONE),
        alt: None,
        help_desc: "Switch Search-Mode [Flat, Deep]",
        help_contexts: &[AppContext::Search],
        command_desc: Some(&[CommandDesc {
            desc: "Switch Search-Mode",
            contexts: &[AppContext::Search],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Char('Q'), crossterm::event::KeyModifiers::CONTROL),
        alt: None,
        help_contexts: &[AppContext::All],
        help_desc: "Quit the app",
        command_desc: None,
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Char('T'), crossterm::event::KeyModifiers::CONTROL),
        alt: None,
        help_desc: "Change the app theme [Dark, Indigo, Light, Dracula]",
        help_contexts: &[AppContext::All],
        command_desc: Some(&[CommandDesc {
            desc: "Toggle theme",
            contexts: &[
                AppContext::Explorer,
                AppContext::Search,
                AppContext::Results,
            ],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Char('O'), crossterm::event::KeyModifiers::CONTROL),
        alt: None,
        help_desc: "Enable/Disable the system overview",
        help_contexts: &[AppContext::All],
        command_desc: Some(&[CommandDesc {
            desc: "Enable/Disable system overview",
            contexts: &[
                AppContext::Explorer,
                AppContext::Search,
                AppContext::Results,
            ],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Char('C'), crossterm::event::KeyModifiers::CONTROL),
        alt: None,
        help_desc: "Copy path of the selected file/directory to clipboard",
        help_contexts: &[AppContext::Explorer, AppContext::Results],
        command_desc: Some(&[CommandDesc {
            desc: "Copy path to clipboard",
            contexts: &[AppContext::Explorer, AppContext::Results],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Char('V'), crossterm::event::KeyModifiers::CONTROL),
        alt: None,
        help_desc: "Paste content from clipboard into the input field",
        help_contexts: &[AppContext::Search],
        command_desc: Some(&[CommandDesc {
            desc: "Paste content",
            contexts: &[AppContext::Search],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Char('F'), crossterm::event::KeyModifiers::CONTROL),
        alt: None,
        help_desc: "Open search to search the current directory for file/directory names",
        help_contexts: &[AppContext::Explorer],
        command_desc: Some(&[CommandDesc {
            desc: "Open search",
            contexts: &[AppContext::Explorer],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Char('U'), crossterm::event::KeyModifiers::CONTROL),
        alt: None,
        help_desc: "Go to the home directory, if any",
        help_contexts: &[AppContext::Explorer],
        command_desc: Some(&[CommandDesc {
            desc: "Go to home dir",
            contexts: &[AppContext::Explorer],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Char('A'), crossterm::event::KeyModifiers::CONTROL),
        alt: None,
        help_desc: "Show metadata for a file or directory, if any",
        help_contexts: &[AppContext::Explorer, AppContext::Results],
        command_desc: Some(&[CommandDesc {
            desc: "Show metadata",
            contexts: &[AppContext::Explorer, AppContext::Results],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::AnyChar, crossterm::event::KeyModifiers::NONE),
        alt: None,
        help_desc: "Select the next file/directory using the initial letter",
        help_contexts: &[AppContext::Explorer],
        command_desc: Some(&[CommandDesc {
            desc: " ",
            contexts: &[AppContext::Explorer],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Up, crossterm::event::KeyModifiers::NONE),
        alt: None,
        help_desc: "Move up to select an item | Moves backward through input history, if any",
        help_contexts: &[
            AppContext::Explorer,
            AppContext::Results,
            AppContext::Search,
        ],
        command_desc: Some(&[CommandDesc {
            desc: " ",
            contexts: &[
                AppContext::Explorer,
                AppContext::Results,
                AppContext::Search,
            ],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Down, crossterm::event::KeyModifiers::NONE),
        alt: None,
        help_desc: "Move down to select an item | Moves forward through input history, if any",
        help_contexts: &[
            AppContext::Explorer,
            AppContext::Results,
            AppContext::Search,
        ],
        command_desc: Some(&[CommandDesc {
            desc: " ",
            contexts: &[
                AppContext::Explorer,
                AppContext::Results,
                AppContext::Search,
            ],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Left, crossterm::event::KeyModifiers::NONE),
        alt: None,
        help_desc: "Move the cursor to the left in the input field",
        help_contexts: &[AppContext::Search],
        command_desc: Some(&[CommandDesc {
            desc: " ",
            contexts: &[AppContext::Search],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::Right, crossterm::event::KeyModifiers::NONE),
        alt: None,
        help_desc: "Move the cursor to the right in the input field",
        help_contexts: &[AppContext::Search],
        command_desc: Some(&[CommandDesc {
            desc: " ",
            contexts: &[AppContext::Search],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::PageUp, crossterm::event::KeyModifiers::NONE),
        alt: None,
        help_desc: "Move page up until the first item is reached",
        help_contexts: &[AppContext::Explorer, AppContext::Results],
        command_desc: Some(&[CommandDesc {
            desc: " ",
            contexts: &[AppContext::Explorer, AppContext::Results],
        }]),
    },
    KeyBinding {
        key_stroke: KeyStroke::new(Keys::PageDown, crossterm::event::KeyModifiers::NONE),
        alt: None,
        help_desc: "Move page down until the last item is reached",
        help_contexts: &[AppContext::Explorer, AppContext::Results],
        command_desc: Some(&[CommandDesc {
            desc: " ",
            contexts: &[AppContext::Explorer, AppContext::Results],
        }]),
    },
];

/// Get the key bindings in a custom table row format
pub fn get_help_docs() -> Vec<Vec<String>> {
    DEFAULT_KEY_BINDING.iter().map(help_row).collect()
}

/*
Convert each key binding to a table row
*/
fn help_row(item: &KeyBinding) -> Vec<String> {
    let context_str = item
        .help_contexts
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<String>>() // Collect into Vec<String>
        .join("|"); // Join elements with "|"

    vec![
        if item.alt.is_some() {
            format!("{} | {}", item.key_stroke, item.alt.clone().unwrap())
        } else {
            format!("{}", item.key_stroke)
        },
        context_str,
        String::from(item.help_desc),
    ]
}

// Get the command description for a specific key event, if any
pub fn get_command_description(
    key_event: &crossterm::event::KeyEvent,
    app_context: &AppContext,
) -> Option<String> {
    DEFAULT_KEY_BINDING
        .iter()
        .filter_map(|key_binding| {
            key_binding.command_desc.and_then(|desc| {
                if key_binding.key_stroke.matches_event(key_event)
                    || key_binding
                        .alt
                        .as_ref()
                        .is_some_and(|alt| alt.matches_event(key_event))
                {
                    desc.iter()
                        .find(|c| c.contexts.contains(app_context))
                        .map(|c| c.desc.to_string())
                } else {
                    None
                }
            })
        })
        .next()
}

#[cfg(test)]
mod test {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_key_display() {
        assert_eq!(Keys::F1.to_string(), "F1");
        assert_eq!(Keys::Char('a').to_string(), "a");
        assert_eq!(Keys::AnyChar.to_string(), "Any Char");
    }

    #[test]
    fn test_keystroke_display() {
        let ks = KeyStroke::new(Keys::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(ks.to_string(), "Ctrl + c");

        let ks2 = KeyStroke::new(Keys::Enter, KeyModifiers::NONE);
        assert_eq!(ks2.to_string(), "Enter");
    }

    #[test]
    fn test_keystroke_match() {
        let ks = KeyStroke::new(Keys::Char('a'), KeyModifiers::CONTROL);
        let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        assert!(ks.matches_event(&event));
    }

    #[test]
    fn test_keystroke_not_match() {
        let ks = KeyStroke::new(Keys::Char('a'), KeyModifiers::CONTROL);
        let event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(!ks.matches_event(&event));
    }

    #[test]
    fn test_key_binding_single_context() {
        let input = KeyBinding {
            key_stroke: KeyStroke::new(Keys::Char('q'), KeyModifiers::CONTROL),
            alt: None,
            help_desc: "Quit the app",
            help_contexts: &[AppContext::All],
            command_desc: None,
        };

        let expected = &["Ctrl + q", "All Contexts", "Quit the app"];

        assert_eq!(help_row(&input), expected);
    }

    #[test]
    fn test_key_binding_multi_context() {
        let input = KeyBinding {
            key_stroke: KeyStroke::new(Keys::Char('A'), crossterm::event::KeyModifiers::CONTROL),
            alt: None,
            help_contexts: &[AppContext::Explorer, AppContext::Results],
            help_desc: "Show metadata for a file or directory, if any",
            command_desc: None,
        };

        let expected = &[
            "Ctrl + A",
            "Explorer|Result",
            "Show metadata for a file or directory, if any",
        ];

        assert_eq!(help_row(&input), expected);
    }

    #[test]
    fn test_is_command_description_1() {
        let key_event = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        let desc = get_command_description(&key_event, &AppContext::Explorer);
        assert_eq!(desc, Some("Show help page".into()));
    }

    #[test]
    fn test_is_command_description_2() {
        let key_event2 = KeyEvent::new(KeyCode::Char('F'), KeyModifiers::CONTROL);
        let desc2 = get_command_description(&key_event2, &AppContext::Explorer);
        assert_eq!(desc2, Some("Open search".into()));
    }

    #[test]
    fn test_is_command_description_3() {
        let key_event = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::CONTROL);
        let desc = get_command_description(&key_event, &AppContext::Explorer);
        assert_eq!(desc, Some("Show metadata".into()));
        let desc = get_command_description(&key_event, &AppContext::Results);
        assert_eq!(desc, Some("Show metadata".into()));
    }

    #[test]
    fn test_is_command_description_4() {
        let key_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let desc = get_command_description(&key_event, &AppContext::Explorer);
        assert_eq!(desc, Some("Change dir".into()));
    }

    #[test]
    fn test_is_command_description_5() {
        let key_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let desc = get_command_description(&key_event, &AppContext::Search);
        assert_eq!(desc, Some("Submit search".into()));
    }

    #[test]
    fn test_not_command_description_1() {
        let key_event2 = KeyEvent::new(KeyCode::Char('E'), KeyModifiers::NONE);
        let desc2 = get_command_description(&key_event2, &AppContext::Search);
        assert_eq!(desc2, None);
    }

    #[test]
    fn test_not_command_description_2() {
        let key_event2 = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);
        let desc2 = get_command_description(&key_event2, &AppContext::Explorer);
        assert_eq!(desc2, None);
    }

    #[test]
    fn test_exact_match_1() {
        let ks = KeyStroke::new(Keys::Enter, KeyModifiers::NONE);
        assert!(ks.matches(&KeyCode::Enter, &KeyModifiers::NONE));
    }

    #[test]
    fn test_exact_match_2() {
        let ks = KeyStroke::new(Keys::Backspace, KeyModifiers::NONE);
        assert!(ks.matches(&KeyCode::Backspace, &KeyModifiers::NONE));
    }

    #[test]
    fn test_exact_match_3() {
        let ks = KeyStroke::new(Keys::Char('O'), KeyModifiers::CONTROL);
        assert!(ks.matches(&KeyCode::Char('O'), &KeyModifiers::CONTROL));
    }

    #[test]
    fn test_mismatch_modifier() {
        let ks = KeyStroke::new(Keys::Enter, KeyModifiers::SHIFT);
        assert!(!ks.matches(&KeyCode::Enter, &KeyModifiers::NONE));
    }

    #[test]
    fn test_char_match() {
        let ks = KeyStroke::new(Keys::Char('a'), KeyModifiers::NONE);
        assert!(ks.matches(&KeyCode::Char('a'), &KeyModifiers::NONE));
    }

    #[test]
    fn test_char_mismatch() {
        let ks = KeyStroke::new(Keys::Char('b'), KeyModifiers::NONE);
        assert!(!ks.matches(&KeyCode::Char('a'), &KeyModifiers::NONE));
    }

    #[test]
    fn test_any_char_match() {
        let ks = KeyStroke::new(Keys::AnyChar, KeyModifiers::NONE);
        assert!(ks.matches(&KeyCode::Char('x'), &KeyModifiers::NONE));
        assert!(ks.matches(&KeyCode::Char('1'), &KeyModifiers::NONE));
    }

    #[test]
    fn test_special_key_match() {
        let ks = KeyStroke::new(Keys::Esc, KeyModifiers::NONE);
        assert!(ks.matches(&KeyCode::Esc, &KeyModifiers::NONE));
    }
}
