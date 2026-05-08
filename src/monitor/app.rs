use std::collections::HashMap;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use octocrab::Octocrab;
use ratatui::widgets::TableState;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tachyonfx::{Duration as FxDuration, Effect};
use tokio_stream::StreamExt;

use crate::monitor::config::AppConfig;
use crate::monitor::core::{self as monitor_core, build_issue_tree, FilterOptions};
use crate::monitor::types::RepoMonitorResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    FilterInput,
    MilestonePicker,
}

#[derive(Debug, Default)]
pub struct MonitorState {
    pub results: Vec<RepoMonitorResult>,
    pub loading: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EffectKey {
    Startup,
    TabSwitch,
    DataLoaded,
    FilterToggle,
    MilestoneToggle,
}

pub struct App {
    pub monitor_state: Arc<RwLock<MonitorState>>,
    pub selected_tab: usize,
    pub table_state: TableState,
    pub detail_scroll: u16,
    pub should_quit: bool,
    pub input_mode: InputMode,
    pub filter: FilterOptions,
    pub filter_input: String,
    pub filter_cursor: usize,

    pub milestone_list: Vec<String>,
    pub milestone_selected: usize,

    pub throbber_state: throbber_widgets_tui::ThrobberState,
    spinner_tick: u8,
    pub last_tick: FxDuration,
    last_frame_instant: Instant,
    pub effects: HashMap<EffectKey, Effect>,
    prev_loading: bool,

    poll_interval: Duration,
    last_poll: Instant,
    force_refresh: bool,
}

impl App {
    fn new(poll_interval: u64, initial_filter: FilterOptions) -> Self {
        let interval_secs = poll_interval;
        let mut effects = HashMap::new();
        effects.insert(EffectKey::Startup, crate::monitor::ui::effects::startup());
        Self {
            monitor_state: Arc::new(RwLock::new(MonitorState::default())),
            selected_tab: 0,
            table_state: TableState::default(),
            detail_scroll: 0,
            should_quit: false,
            input_mode: InputMode::Normal,
            filter: initial_filter,
            filter_input: String::new(),
            filter_cursor: 0,

            milestone_list: Vec::new(),
            milestone_selected: 0,

            throbber_state: throbber_widgets_tui::ThrobberState::default(),
            spinner_tick: 0,
            last_tick: FxDuration::from_millis(33),
            last_frame_instant: Instant::now(),
            effects,
            prev_loading: false,

            poll_interval: Duration::from_secs(interval_secs),
            last_poll: Instant::now() - Duration::from_secs(interval_secs + 1),
            force_refresh: false,
        }
    }

    pub fn tick_spinner(&mut self) {
        self.spinner_tick = self.spinner_tick.wrapping_add(1);
        if self.spinner_tick.is_multiple_of(4) {
            self.throbber_state.calc_next();
        }
    }

    pub fn next_poll_secs(&self) -> u64 {
        let elapsed = self.last_poll.elapsed();
        self.poll_interval
            .checked_sub(elapsed)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    pub fn current_issue_count(&self) -> usize {
        let state = self.monitor_state.read().unwrap();
        state
            .results
            .get(self.selected_tab)
            .map(|r| {
                let filtered: Vec<_> = r.issues.iter().filter(|i| self.filter.matches(i)).cloned().collect();
                build_issue_tree(&filtered).len()
            })
            .unwrap_or(0)
    }

    fn collect_milestones(&self) -> Vec<String> {
        let state = self.monitor_state.read().unwrap();
        let Some(repo) = state.results.get(self.selected_tab) else {
            return vec![];
        };
        let mut seen = std::collections::HashSet::new();
        let mut list = Vec::new();
        for issue in &repo.issues {
            if let Some(ms) = &issue.milestone {
                if seen.insert(ms.title.clone()) {
                    list.push(ms.title.clone());
                }
            }
        }
        list.sort();
        list
    }

    pub fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            match self.input_mode {
                InputMode::Normal => self.handle_normal_key(key),
                InputMode::FilterInput => self.handle_filter_key(key),
                InputMode::MilestonePicker => self.handle_milestone_key(key),
            }
        }
    }

    fn handle_normal_key(&mut self, key: crossterm::event::KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                let count = self.current_issue_count();
                if count > 0 {
                    let next = self
                        .table_state
                        .selected()
                        .map(|i| (i + 1).min(count - 1))
                        .unwrap_or(0);
                    self.table_state.select(Some(next));
                    self.detail_scroll = 0;
                }
            }
            (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                let next = self
                    .table_state
                    .selected()
                    .map(|i| i.saturating_sub(1))
                    .unwrap_or(0);
                self.table_state.select(Some(next));
                self.detail_scroll = 0;
            }
            (KeyCode::Tab, _) => {
                let tab_count = self.monitor_state.read().unwrap().results.len();
                if tab_count > 0 {
                    self.selected_tab = (self.selected_tab + 1) % tab_count;
                    self.table_state.select(Some(0));
                    self.detail_scroll = 0;
                    self.effects
                        .insert(EffectKey::TabSwitch, crate::monitor::ui::effects::tab_switch());
                }
            }
            (KeyCode::BackTab, _) => {
                let tab_count = self.monitor_state.read().unwrap().results.len();
                if tab_count > 0 {
                    self.selected_tab = (self.selected_tab + tab_count - 1) % tab_count;
                    self.table_state.select(Some(0));
                    self.detail_scroll = 0;
                    self.effects
                        .insert(EffectKey::TabSwitch, crate::monitor::ui::effects::tab_switch());
                }
            }
            (KeyCode::Char('r'), _) => {
                self.force_refresh = true;
            }
            (KeyCode::PageDown, _) => {
                self.detail_scroll = self.detail_scroll.saturating_add(3);
            }
            (KeyCode::PageUp, _) => {
                self.detail_scroll = self.detail_scroll.saturating_sub(3);
            }
            (KeyCode::Char('/'), _) => {
                self.input_mode = InputMode::FilterInput;
                self.filter_input.clear();
                self.filter_cursor = 0;
                self.effects
                    .insert(EffectKey::FilterToggle, crate::monitor::ui::effects::filter_toggle());
            }
            (KeyCode::Char('m'), _) => {
                let milestones = self.collect_milestones();
                self.milestone_list = milestones;
                self.milestone_selected = 0;
                self.input_mode = InputMode::MilestonePicker;
                self.effects
                    .insert(EffectKey::MilestoneToggle, crate::monitor::ui::effects::filter_toggle());
            }
            (KeyCode::Char('C'), _) => {
                if !self.filter.is_empty() {
                    self.filter = FilterOptions::default();
                    self.filter_input.clear();
                    self.force_refresh = true;
                    self.table_state.select(Some(0));
                }
            }
            _ => {}
        }
    }

    fn handle_filter_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                let text = self.filter_input.trim().to_string();
                if !text.is_empty() {
                    self.filter.keywords = vec![text];
                } else {
                    self.filter.keywords.clear();
                }
                self.filter_input.clear();
                self.filter_cursor = 0;
                self.input_mode = InputMode::Normal;
                self.force_refresh = true;
                self.table_state.select(Some(0));
            }
            KeyCode::Esc => {
                self.filter_input.clear();
                self.filter_cursor = 0;
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                if self.filter_cursor > 0 {
                    self.filter_input.remove(self.filter_cursor - 1);
                    self.filter_cursor -= 1;
                }
            }
            KeyCode::Left => {
                self.filter_cursor = self.filter_cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                self.filter_cursor = (self.filter_cursor + 1).min(self.filter_input.len());
            }
            KeyCode::Char(c) => {
                self.filter_input.insert(self.filter_cursor, c);
                self.filter_cursor += 1;
            }
            _ => {}
        }
    }

    fn handle_milestone_key(&mut self, key: crossterm::event::KeyEvent) {
        let total = 1 + self.milestone_list.len();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.milestone_selected = (self.milestone_selected + 1).min(total - 1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.milestone_selected = self.milestone_selected.saturating_sub(1);
            }
            KeyCode::Enter => {
                if self.milestone_selected == 0 {
                    self.filter.milestone = None;
                } else {
                    let ms = self.milestone_list[self.milestone_selected - 1].clone();
                    self.filter.milestone = Some(ms);
                }
                self.input_mode = InputMode::Normal;
                self.force_refresh = true;
                self.table_state.select(Some(0));
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            _ => {}
        }
    }
}

pub async fn run(client: Octocrab, config: AppConfig, initial_filter: FilterOptions) -> Result<()> {
    let mut app = App::new(config.poll_interval, initial_filter);
    app.table_state.select(Some(0));

    let terminal = ratatui::init();
    let result = run_inner(&mut app, terminal, client, config).await;
    ratatui::restore();
    result
}

async fn run_inner(
    app: &mut App,
    mut terminal: ratatui::DefaultTerminal,
    client: Octocrab,
    config: AppConfig,
) -> Result<()> {
    let render_interval = Duration::from_millis(33);
    let mut render_ticker = tokio::time::interval(render_interval);
    let mut events = crossterm::event::EventStream::new();

    loop {
        let should_poll = app.force_refresh || app.last_poll.elapsed() >= app.poll_interval;

        if should_poll {
            app.force_refresh = false;
            app.last_poll = Instant::now();

            {
                let mut state = app.monitor_state.write().unwrap();
                state.loading = true;
            }

            let state_ref = Arc::clone(&app.monitor_state);
            let client_clone = client.clone();
            let repos = config.repositories.clone();
            let filter = app.filter.clone();
            tokio::spawn(async move {
                let results = monitor_core::fetch_all(&client_clone, &repos, None, &filter)
                    .await
                    .unwrap_or_default();
                let mut state = state_ref.write().unwrap();
                state.results = results;
                state.loading = false;
            });
        }

        tokio::select! {
            _ = render_ticker.tick() => {
                app.last_tick = FxDuration::from_millis(
                    app.last_frame_instant.elapsed().as_millis() as u32
                );
                app.last_frame_instant = Instant::now();

                app.tick_spinner();

                terminal.draw(|frame| crate::monitor::ui::render(frame, app))?;

                app.effects.retain(|_, e| e.running());

                let is_loading = { app.monitor_state.read().unwrap().loading };
                if app.prev_loading && !is_loading {
                    app.effects
                        .insert(EffectKey::DataLoaded, crate::monitor::ui::effects::data_loaded());
                }
                app.prev_loading = is_loading;
            }
            Some(Ok(event)) = events.next() => {
                app.handle_event(event);
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
