use anyhow::{Error, Result};
use std::ops::{Deref, DerefMut};

use futures::{FutureExt, StreamExt};
use ratatui::backend::CrosstermBackend as Backend;
use ratatui::crossterm::{
    cursor,
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event as CrosstermEvent, KeyEvent, MouseEvent,
    },
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

/// Terminal input events
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    Init,
    Quit,
    Error(String),
    Closed,
    AppTick,
    RenderTick,
    FocusGained,
    FocusLost,
    Paste(String),
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

/// Terminal user interface
pub struct Tui {
    pub terminal: ratatui::Terminal<Backend<std::io::Stdout>>,
    pub task: JoinHandle<()>,
    pub cancellation_token: CancellationToken,
    pub event_receiver: UnboundedReceiver<Event>,
    pub event_sender: UnboundedSender<Event>,
    pub frame_rate: f64,
    pub tick_rate: f64,
    pub mouse: bool,
    pub paste: bool,
}

impl Tui {
    /// Constructs a new instance of [`Tui`].
    pub fn new() -> Result<Self> {
        let tick_rate = Default::default();
        let frame_rate = Default::default();
        let terminal = ratatui::Terminal::new(Backend::new(std::io::stdout()))?;
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let cancellation_token = CancellationToken::new();
        let task = tokio::spawn(async {
            std::future::pending::<()>().await;
        });
        let mouse = false;
        let paste = false;
        Ok(Self {
            terminal,
            task,
            cancellation_token,
            event_receiver,
            event_sender,
            frame_rate,
            tick_rate,
            mouse,
            paste,
        })
    }

    /// Set a new Tick-Rate fpr the Event-Handler
    pub fn tick_rate(mut self, tick_rate: f64) -> Self {
        self.tick_rate = tick_rate;
        self
    }

    /// Set a new Frame-Rate fpr the Event-Handler
    pub fn frame_rate(mut self, frame_rate: f64) -> Self {
        self.frame_rate = frame_rate;
        self
    }

    pub fn _mouse(mut self, mouse: bool) -> Self {
        self.mouse = mouse;
        self
    }

    pub fn _paste(mut self, paste: bool) -> Self {
        self.paste = paste;
        self
    }

    /// Start the Terminal-User-Interface event loop, to process user events such as
    /// keystrokes or mouse clicks/movement
    pub fn start(&mut self) {
        let tick_delay = std::time::Duration::from_secs_f64(1.0 / self.tick_rate);
        let render_delay = std::time::Duration::from_secs_f64(1.0 / self.frame_rate);
        self.cancel();
        self.cancellation_token = CancellationToken::new();
        let _cancellation_token = self.cancellation_token.clone();
        let _event_tx = self.event_sender.clone();
        self.task = tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_delay);
            let mut render_interval = tokio::time::interval(render_delay);
            _event_tx
                .send(Event::Init)
                .expect("Unable to send TUI-Init-Event over channel");
            loop {
                let tick_delay = tick_interval.tick();
                let render_delay = render_interval.tick();
                let crossterm_event = reader.next().fuse();
                tokio::select! {
                  _ = _cancellation_token.cancelled() => {
                    break;
                  }
                  maybe_event = crossterm_event => {
                    match maybe_event {
                      Some(Ok(evt)) => {
                        match evt {
                          CrosstermEvent::Key(key) => {
                            handle_key_event(&_event_tx, key).await;
                          },
                          CrosstermEvent::Mouse(mouse) => {
                            _event_tx.send(Event::Mouse(mouse)).expect("Unable to send TUI-Mouse-Event over channel");
                          },
                          CrosstermEvent::Resize(x, y) => {
                            _event_tx.send(Event::Resize(x, y)).expect("Unable to send TUI-Resize-Event over channel");
                          },
                          CrosstermEvent::FocusLost => {
                            _event_tx.send(Event::FocusLost).expect("Unable to send TUI-FocusLost-Event over channel");
                          },
                          CrosstermEvent::FocusGained => {
                            _event_tx.send(Event::FocusGained).expect("Unable to send TUI-FocusGained-Event over channel");
                          },
                          CrosstermEvent::Paste(s) => {
                            _event_tx.send(Event::Paste(s)).expect("Unable to send TUI-Paste-Event over channel");
                          },
                        }
                      }
                      Some(Err(crossterm_err)) => {
                        _event_tx.send(Event::Error(format!("{:?}", crossterm_err))).expect("Unable to send TUI-Error-Event over channel");
                      }
                      None => {},
                    }
                  },
                  _ = tick_delay => {
                      _event_tx.send(Event::AppTick).expect("Unable to send TUI-AppTick-Event over channel");
                  },
                  _ = render_delay => {
                      _event_tx.send(Event::RenderTick).expect("Unable to send TUI-RenderTick-Event over channel");
                  },
                }
            }
        });
    }

    pub fn stop(&self) {
        self.cancel();
        let mut counter = 0;

        while !self.task.is_finished() {
            counter += 1;
            std::thread::sleep(std::time::Duration::from_millis(1));
            if counter > 50 {
                self.task.abort();
            }
            if counter > 500 {
                log::error!(
                    "Unable to abort TUI-Event-Task in 500 milliseconds for unknown reason"
                );
                break;
            }
        }
    }

    pub fn enter(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stdout(), EnterAlternateScreen, cursor::Hide)?;
        if self.mouse {
            crossterm::execute!(std::io::stdout(), EnableMouseCapture)?;
        }
        if self.paste {
            crossterm::execute!(std::io::stdout(), EnableBracketedPaste)?;
        }
        self.start();
        Ok(())
    }

    pub fn exit(&mut self) -> Result<()> {
        self.stop();
        if crossterm::terminal::is_raw_mode_enabled()? {
            self.flush()?;
            if self.paste {
                crossterm::execute!(std::io::stdout(), DisableBracketedPaste)?;
            }
            if self.mouse {
                crossterm::execute!(std::io::stdout(), DisableMouseCapture)?;
            }
            crossterm::execute!(std::io::stdout(), LeaveAlternateScreen, cursor::Show)?;
            crossterm::terminal::disable_raw_mode()?;
        }
        Ok(())
    }

    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    pub async fn next(&mut self) -> Result<Event> {
        self.event_receiver.recv().await.ok_or({
            Error::msg("An TUI-Event error occurred - Unable to get the next terminal user event")
        })
    }
}

impl Deref for Tui {
    type Target = ratatui::Terminal<Backend<std::io::Stdout>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for Tui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

// Right key event handling on windows platforms.
// Crossterm Backend also emits key release and repeat events on Windows,
// so it's important to check that the Key-Event type is KeyEventKind::Press before proceeding.
#[cfg(target_os = "windows")]
async fn handle_key_event(event_tx: &mpsc::UnboundedSender<Event>, key_event: KeyEvent) {
    if key_event.kind == crossterm::event::KeyEventKind::Press {
        event_tx
            .send(Event::Key(key_event))
            .expect("Unable to send TUI-Key-Event over channel");
    }
}

#[cfg(not(target_os = "windows"))]
async fn handle_key_event(event_tx: &mpsc::UnboundedSender<Event>, key_event: KeyEvent) {
    event_tx
        .send(Event::Key(key_event))
        .expect("Unable to send TUI-Key-Event over channel");
}
