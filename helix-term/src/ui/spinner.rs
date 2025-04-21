use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

use helix_lsp::LanguageServerId;

// Global counter for active spinners
static ACTIVE_SPINNER_COUNT: AtomicUsize = AtomicUsize::new(0);
static SPINNER_FRAMES: [&str; 8] = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];

#[derive(Default, Debug)]
pub struct ProgressSpinners {
    inner: HashMap<LanguageServerId, Spinner>,
    current_frame: usize,
    last_update: Option<Instant>,
}

impl ProgressSpinners {
    pub fn get(&self, id: LanguageServerId) -> Option<&Spinner> {
        self.inner.get(&id)
    }

    pub fn get_or_create(&mut self, id: LanguageServerId) -> &mut Spinner {
        self.inner.entry(id).or_default()
    }

    pub fn has_active_spinners(&self) -> bool {
        self.inner.values().any(|spinner| !spinner.is_stopped())
    }

    pub fn current_frame(&mut self) -> &str {
        let now = Instant::now();

        if let Some(last) = self.last_update {
            if now.duration_since(last).as_millis() >= 80 {
                self.current_frame = (self.current_frame + 1) % SPINNER_FRAMES.len();
                self.last_update = Some(now);
            }
        } else {
            self.last_update = Some(now);
        }

        SPINNER_FRAMES[self.current_frame]
    }
}

// Helper functions to access the global spinner count
pub fn any_spinner_active() -> bool {
    // Use Acquire when reading
    ACTIVE_SPINNER_COUNT.load(Ordering::Acquire) > 0
}

#[derive(Default, Debug)]
pub struct Spinner {
    start: Option<Instant>,
}

impl Spinner {
    pub fn new() -> Self {
        Self { start: None }
    }

    pub fn start(&mut self) {
        if self.start.is_none() {
            ACTIVE_SPINNER_COUNT.fetch_add(1, Ordering::Release);
        }
        self.start = Some(Instant::now());
    }

    pub fn stop(&mut self) {
        if self.start.is_some() {
            ACTIVE_SPINNER_COUNT.fetch_sub(1, Ordering::Release);
        }
        self.start = None;
    }

    pub fn is_stopped(&self) -> bool {
        self.start.is_none()
    }
}
