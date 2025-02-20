#![allow(dead_code)]
use ratatui::widgets::{ListState, TableState};

/// A trait that provides scrolling functionality for a scrollable structure.
///
/// # Overview
/// The `Scrollable` trait defines methods for scrolling up and down,
/// with support for both single-step and multi-step (paged) scrolling.
/// Implementors must define at least one of each pair of methods:
/// - `scroll_up()` or `scroll_up_by(usize)`
/// - `scroll_down()` or `scroll_down_by(usize)`
/// - `page_up()` or `page_up_by(usize)`
/// - `page_down()` or `page_down_by(usize)`
///
/// Default implementations ensure that if only one method is implemented,
/// the other will work automatically, preventing redundant code.
///
/// # Required Methods
/// - `scroll_up_by(usize)`: Scrolls up by a given number of steps.
/// - `scroll_down_by(usize)`: Scrolls down by a given number of steps.
/// - `scroll_up()`: Scrolls up by one step (default calls `scroll_up_by(1)`, unless overridden).
/// - `scroll_down()`: Scrolls down by one step (default calls `scroll_down_by(1)`, unless overridden).
/// - `page_up_by(usize)`: Scrolls up by a given number of pages.
/// - `page_down_by(usize)`: Scrolls down by a given number of pages.
/// - `page_up()`: Scrolls up by one page (default calls `page_up_by(1)`, unless overridden).
/// - `page_down()`: Scrolls down by one page (default calls `page_down_by(1)`, unless overridden).
///
/// # Default Methods
/// - `handle_scroll(bool, bool)`: Handles scrolling based on direction (`up`) and paging (`page`).
///
/// # Notes
/// - If neither `scroll_up_by()` nor `scroll_up()` is implemented, calling `scroll_up()` will result in infinite recursion.
/// - The same applies to `scroll_down_by()` and `scroll_down()`.
/// - Implementing at least one method per direction prevents this issue.
///
/// This trait is designed for use in lists, buffers, or any UI elements that require controlled scrolling.
pub trait Scrollable {
    /// Handles scrolling based on direction and paging.
    ///
    /// - `up`: If `true`, scrolls up. If `false`, scrolls down.
    /// - `page`: If `true`, scrolls by a larger amount (default: 10 steps).
    fn handle_scroll(&mut self, up: bool, page: bool) {
        let inc_or_dec = if page { 10 } else { 1 };
        if up {
            self.scroll_up_by(inc_or_dec);
        } else {
            self.scroll_down_by(inc_or_dec);
        }
    }

    /// Scrolls down by a specific number of steps.
    fn scroll_down_by(&mut self, inc_or_dec: usize) {
        for _ in 0..inc_or_dec {
            self.scroll_down();
        }
    }

    /// Scrolls up by a specific number of steps.
    fn scroll_up_by(&mut self, inc_or_dec: usize) {
        for _ in 0..inc_or_dec {
            self.scroll_up();
        }
    }

    /// Scrolls down by one step.
    fn scroll_down(&mut self) {
        self.scroll_down_by(1);
    }

    /// Scrolls up by one step.
    fn scroll_up(&mut self) {
        self.scroll_up_by(1);
    }

    /// Scrolls up by a specific number of pages.
    fn page_up_by(&mut self, pages: u16) {
        for _ in 0..pages {
            self.page_up();
        }
    }

    /// Scrolls down by a specific number of pages.
    fn page_down_by(&mut self, pages: u16) {
        for _ in 0..pages {
            self.page_down();
        }
    }

    /// Scrolls up by a full page (default: 10 steps).
    fn page_up(&mut self) {
        self.scroll_up_by(10);
    }

    /// Scrolls down by a full page (default: 10 steps).
    fn page_down(&mut self) {
        self.scroll_down_by(10);
    }
}

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn new() -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items: Vec::new(),
        }
    }
    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        StatefulList { state, items }
    }

    pub fn get_slice_of_items(&self, start: usize, end: usize) -> &[T] {
        if start < end && self.items.len() >= end {
            &self.items[start..end]
        } else {
            &self.items[..self.items.len()]
        }
    }
}

impl<T> Scrollable for StatefulList<T> {
    // for lists we cycle back to the beginning when we reach the end
    fn scroll_down_by(&mut self, increment: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len().saturating_sub(increment) {
                    0
                } else {
                    i + increment
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    // for lists we cycle back to the end when we reach the beginning
    fn scroll_up_by(&mut self, decrement: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len().saturating_sub(decrement)
                } else {
                    i.saturating_sub(decrement)
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

#[derive(Clone, Debug)]
pub struct StatefulTable<T> {
    pub state: TableState,
    pub items: Vec<T>,
    pub selected_item: usize,
}

impl<T> StatefulTable<T> {
    pub fn new() -> StatefulTable<T> {
        StatefulTable {
            state: TableState::default(),
            items: Vec::new(),
            selected_item: Default::default(),
        }
    }

    pub fn with_items(items: Vec<T>) -> StatefulTable<T> {
        let mut table = StatefulTable::new();
        if !items.is_empty() {
            table.state.select_first();
            table.state.select_first_column();
        }
        table.set_items(items);
        table
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        let item_len = items.len();
        self.items = items;
        if !self.items.is_empty() {
            let i = self.state.selected().map_or(0, |i| {
                if i > 0 && i < item_len {
                    i
                } else if i >= item_len {
                    item_len - 1
                } else {
                    0
                }
            });
            self.state.select(Some(i));
        }
    }
}

impl<T> Scrollable for StatefulTable<T> {
    fn scroll_down_by(&mut self, increment: usize) {
        if let Some(i) = self.state.selected() {
            if (i + increment) < self.items.len() {
                self.selected_item = i + increment;
                self.state.select(Some(self.selected_item));
            } else {
                self.selected_item = self.items.len().saturating_sub(1);
                self.state.select(Some(self.selected_item));
            }
        }
    }

    fn scroll_up_by(&mut self, decrement: usize) {
        if let Some(i) = self.state.selected() {
            if i != 0 {
                self.selected_item = i.saturating_sub(decrement);
                self.state.select(Some(self.selected_item));
            }
        }
    }
}
