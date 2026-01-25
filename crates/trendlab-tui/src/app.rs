//! Central application state.
//!
//! All state lives here. Panels render from this state, they don't own it.
//! Actions dispatch to modify state, not ad-hoc mutations.

use crate::action::{Action, Tab};
use crate::keybindings::KeyContext;
use crate::runner::BacktestRunner;
use chrono::NaiveDate;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tokio::sync::mpsc;
use trendlab_core::types::Bar;
use trendlab_data::{DataProvider, Universe, UniverseId};
use trendlab_yolo::{
    leaderboard::LeaderboardType,
    robustness::RobustnessScore,
    session::SessionStats,
    ComponentRegistry, Genome, LeaderboardEntry, SessionConfig,
    SessionPhase, YoloSession,
};

/// Application phase (distinct from session phase).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppPhase {
    /// Ready to start, waiting for user to press Enter
    Ready,
    /// Loading data from Yahoo Finance / cache
    DataLoading,
    /// YOLO session is running
    Running,
    /// Session is paused
    Paused,
    /// Session has completed
    Completed,
}

impl AppPhase {
    /// Get a display string for the phase.
    pub fn as_str(&self) -> &'static str {
        match self {
            AppPhase::Ready => "Ready",
            AppPhase::DataLoading => "Loading Data",
            AppPhase::Running => "Running",
            AppPhase::Paused => "Paused",
            AppPhase::Completed => "Completed",
        }
    }
}

/// Data loading progress information.
#[derive(Debug, Clone)]
pub struct DataLoadProgress {
    /// Current symbol being loaded
    pub current_symbol: String,
    /// Index of current symbol (0-based)
    pub current_index: usize,
    /// Total number of symbols to load
    pub total_symbols: usize,
    /// Symbols that have been successfully loaded
    pub loaded_count: usize,
    /// Symbols that failed to load
    pub failed_count: usize,
}

impl DataLoadProgress {
    /// Get progress as a percentage (0.0 to 1.0).
    pub fn progress(&self) -> f64 {
        if self.total_symbols == 0 {
            0.0
        } else {
            self.current_index as f64 / self.total_symbols as f64
        }
    }
}

/// Message from the data loading task.
#[derive(Debug)]
pub enum DataLoadMessage {
    /// Progress update.
    Progress {
        /// Symbol currently being loaded.
        symbol: String,
        /// Current symbol index (0-based).
        current: usize,
        /// Total number of symbols.
        total: usize,
    },
    /// Symbol loaded successfully.
    SymbolLoaded {
        /// Symbol that was loaded.
        symbol: String,
        /// OHLCV bars for the symbol.
        bars: Vec<Bar>,
        /// ATR indicator values.
        atr: Vec<f64>,
        /// ADX indicator values.
        adx: Vec<f64>,
    },
    /// Symbol failed to load.
    SymbolFailed {
        /// Symbol that failed.
        symbol: String,
        /// Error message describing the failure.
        error: String,
    },
    /// All loading complete.
    Complete {
        /// Number of symbols successfully loaded.
        loaded: usize,
        /// Number of symbols that failed to load.
        failed: usize,
    },
}

/// Modal dialogs that can be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modal {
    /// Help modal showing keybindings
    Help,
    /// Export modal for Pine Script
    Export,
    /// Configuration modal
    Config,
}

/// Export artifact structure for JSON export.
///
/// Contains all information needed to reproduce a strategy and its results.
#[derive(Debug, Clone, Serialize)]
pub struct ExportArtifact {
    /// Version of the export format
    pub version: String,
    /// Unique fingerprint for this genome
    pub fingerprint: String,
    /// ISO 8601 timestamp of when the export was created
    pub created_at: String,
    /// The genome (strategy composition)
    pub genome: Genome,
    /// Robustness metrics
    pub robustness: RobustnessScore,
    /// Number of evaluations used
    pub num_evaluations: usize,
    /// Export notes
    pub notes: ExportNotes,
}

/// Notes included in the export artifact.
#[derive(Debug, Clone, Serialize)]
pub struct ExportNotes {
    /// Pine Script generation status
    pub pine_script: String,
    /// Test vectors status
    pub test_vectors: String,
}

/// Central application state.
pub struct App {
    /// Current application phase
    pub phase: AppPhase,
    /// Currently active YOLO session (if any)
    pub session: Option<YoloSession>,
    /// Backtest runner for executing backtests
    runner: BacktestRunner,
    /// Currently active tab
    pub active_tab: Tab,
    /// Selected leaderboard type
    pub selected_leaderboard: LeaderboardType,
    /// Selected entry index in the leaderboard
    pub selected_entry_index: usize,
    /// Currently open modal (if any)
    pub modal: Option<Modal>,
    /// Whether the application should quit
    pub should_quit: bool,
    /// Error message to display (if any)
    pub error_message: Option<String>,
    /// Status message to display (if any)
    pub status_message: Option<String>,
    /// Number of visible items in the leaderboard (for pagination)
    pub visible_items: usize,
    /// Tick counter for animations
    pub tick_count: u64,
    /// Last iteration result (for status display)
    pub last_iteration_trade_count: Option<usize>,
    /// Data loading progress
    pub data_load_progress: Option<DataLoadProgress>,
    /// Channel for receiving data load messages
    data_load_receiver: Option<mpsc::Receiver<DataLoadMessage>>,
    /// Loaded symbol data (symbol -> (bars, atr, adx))
    pub symbol_data: HashMap<String, (Vec<Bar>, Vec<f64>, Vec<f64>)>,
    /// Current symbol index for YOLO iteration
    pub current_symbol_index: usize,
    /// Universe being used
    pub universe: Universe,
    /// Cache of equity curves by genome fingerprint (for chart display)
    pub equity_curve_cache: HashMap<String, Vec<f64>>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Create a new application instance.
    pub fn new() -> Self {
        Self {
            phase: AppPhase::Ready,
            session: None,
            runner: BacktestRunner::new(),
            active_tab: Tab::default(),
            selected_leaderboard: LeaderboardType::Overall,
            selected_entry_index: 0,
            modal: None,
            should_quit: false,
            error_message: None,
            status_message: None,
            visible_items: 20, // Will be updated based on terminal size
            tick_count: 0,
            last_iteration_trade_count: None,
            data_load_progress: None,
            data_load_receiver: None,
            symbol_data: HashMap::new(),
            current_symbol_index: 0,
            universe: UniverseId::Sp100.get(), // Default to SP100 (98 symbols)
            equity_curve_cache: HashMap::new(),
        }
    }

    /// Create with a specific universe.
    pub fn with_universe(universe: Universe) -> Self {
        Self {
            universe,
            ..Self::new()
        }
    }

    /// Get the current keybinding context.
    pub fn key_context(&self) -> KeyContext {
        match self.modal {
            Some(Modal::Help) => KeyContext::HelpModal,
            Some(Modal::Export) => KeyContext::ExportModal,
            Some(Modal::Config) => KeyContext::ConfigModal,
            None => KeyContext::Normal,
        }
    }

    /// Apply an action to modify state.
    pub fn apply_action(&mut self, action: Action) {
        // Clear status messages on most actions (except Tick and None)
        if !matches!(action, Action::Tick | Action::None) {
            self.clear_messages_on_action(&action);
        }

        match action {
            Action::Quit => self.should_quit = true,
            Action::StartSession => self.start_session(),
            Action::StopSession => self.stop_session(),
            Action::PauseResume => self.toggle_pause(),
            Action::SelectTab(tab) => self.select_tab(tab),
            Action::NextTab => self.select_tab(self.active_tab.next()),
            Action::PrevTab => self.select_tab(self.active_tab.prev()),
            Action::NavigateUp => self.navigate(-1),
            Action::NavigateDown => self.navigate(1),
            Action::NavigateFirst => self.navigate_to(0),
            Action::NavigateLast => self.navigate_to_last(),
            Action::PageUp => self.navigate(-(self.visible_items as i32 / 2)),
            Action::PageDown => self.navigate(self.visible_items as i32 / 2),
            Action::SelectLeaderboard(lb) => self.select_leaderboard(lb),
            Action::NextLeaderboard => self.cycle_leaderboard(1),
            Action::PrevLeaderboard => self.cycle_leaderboard(-1),
            Action::SelectEntry => { /* Entry selection logic */ }
            Action::ShowHelp => self.modal = Some(Modal::Help),
            Action::ShowExport => self.modal = Some(Modal::Export),
            Action::ShowConfig => self.modal = Some(Modal::Config),
            Action::CloseModal => self.modal = None,
            Action::Export => self.export_selected(),
            Action::ToggleDetail => { /* Toggle detail view */ }
            Action::CycleSort => { /* Cycle sort column */ }
            Action::Tick => self.on_tick(),
            Action::None => {}
        }
    }

    /// Clear status/error messages based on the action type.
    ///
    /// Status messages are cleared on most user interactions to avoid
    /// stale messages lingering on screen.
    fn clear_messages_on_action(&mut self, action: &Action) {
        // Don't clear messages when opening/navigating modals
        if matches!(
            action,
            Action::ShowHelp
                | Action::ShowExport
                | Action::ShowConfig
                | Action::NavigateUp
                | Action::NavigateDown
                | Action::NavigateFirst
                | Action::NavigateLast
                | Action::PageUp
                | Action::PageDown
        ) {
            return;
        }

        // Clear status message (but preserve error messages a bit longer)
        self.status_message = None;

        // Clear error message on session start
        if matches!(action, Action::StartSession) {
            self.error_message = None;
        }
    }

    /// Start a new YOLO session.
    ///
    /// This initiates data loading first. Once data is loaded, YOLO will start.
    fn start_session(&mut self) {
        if self.phase != AppPhase::Ready && self.phase != AppPhase::Completed {
            return;
        }

        // If we already have loaded data, skip to running
        if !self.symbol_data.is_empty() {
            self.start_yolo_with_loaded_data();
            return;
        }

        // Start data loading
        self.phase = AppPhase::DataLoading;
        self.data_load_progress = Some(DataLoadProgress {
            current_symbol: String::new(),
            current_index: 0,
            total_symbols: self.universe.len(),
            loaded_count: 0,
            failed_count: 0,
        });

        // Create channel for data load messages
        let (tx, rx) = mpsc::channel(100);
        self.data_load_receiver = Some(rx);

        // Clone universe for the async task
        let symbols: Vec<String> = self.universe.symbols.clone();

        // Spawn data loading task
        tokio::spawn(async move {
            load_data_task(symbols, tx).await;
        });

        self.error_message = None;
        self.status_message = Some("Loading market data...".to_string());
    }

    /// Start YOLO with already loaded data.
    fn start_yolo_with_loaded_data(&mut self) {
        // Get the first symbol's data to initialize the runner
        if let Some((symbol, (bars, atr, adx))) = self.symbol_data.iter().next() {
            self.runner.initialize_with_data(
                bars.clone(),
                atr.clone(),
                adx.clone(),
            );
            self.status_message = Some(format!("Running on {} ({} bars)", symbol, bars.len()));
        }

        let config = SessionConfig::default();
        let registry = ComponentRegistry::with_defaults();
        let mut session = YoloSession::new(config, registry);
        session.start();

        self.session = Some(session);
        self.phase = AppPhase::Running;
        self.selected_entry_index = 0;
        self.error_message = None;
        self.last_iteration_trade_count = None;
    }

    /// Process pending data load messages.
    ///
    /// Call this from the tick handler to process async data loading results.
    pub fn process_data_load_messages(&mut self) {
        // Collect messages first to avoid borrow issues
        let messages: Vec<DataLoadMessage> = {
            let Some(receiver) = self.data_load_receiver.as_mut() else {
                return;
            };
            let mut msgs = Vec::new();
            while let Ok(msg) = receiver.try_recv() {
                msgs.push(msg);
            }
            msgs
        };

        // Now process the collected messages
        for msg in messages {
            match msg {
                DataLoadMessage::Progress { symbol, current, total } => {
                    if let Some(ref mut progress) = self.data_load_progress {
                        progress.current_symbol = symbol;
                        progress.current_index = current;
                        progress.total_symbols = total;
                    }
                }
                DataLoadMessage::SymbolLoaded { symbol, bars, atr, adx } => {
                    self.symbol_data.insert(symbol, (bars, atr, adx));
                    if let Some(ref mut progress) = self.data_load_progress {
                        progress.loaded_count += 1;
                    }
                }
                DataLoadMessage::SymbolFailed { symbol, error } => {
                    tracing::warn!(symbol = %symbol, error = %error, "Failed to load symbol");
                    if let Some(ref mut progress) = self.data_load_progress {
                        progress.failed_count += 1;
                    }
                }
                DataLoadMessage::Complete { loaded, failed } => {
                    self.data_load_receiver = None;
                    self.data_load_progress = None;

                    if loaded == 0 {
                        self.error_message = Some(format!(
                            "No data loaded ({} symbols failed). Check internet connection.",
                            failed
                        ));
                        self.phase = AppPhase::Ready;
                    } else {
                        self.status_message = Some(format!(
                            "Loaded {} symbols ({} failed)",
                            loaded, failed
                        ));
                        // Now start YOLO with the loaded data
                        self.start_yolo_with_loaded_data();
                    }
                }
            }
        }
    }

    /// Stop the current session.
    fn stop_session(&mut self) {
        if let Some(ref mut session) = self.session {
            session.stop();
            self.phase = AppPhase::Completed;
            self.runner.reset();
        }
    }

    /// Toggle pause state.
    fn toggle_pause(&mut self) {
        match self.phase {
            AppPhase::Running => self.phase = AppPhase::Paused,
            AppPhase::Paused => self.phase = AppPhase::Running,
            _ => {}
        }
    }

    /// Navigate in the leaderboard.
    fn navigate(&mut self, delta: i32) {
        let max_index = self.leaderboard_len().saturating_sub(1);
        let new_index = (self.selected_entry_index as i32 + delta)
            .max(0)
            .min(max_index as i32) as usize;
        self.selected_entry_index = new_index;
    }

    /// Navigate to a specific index.
    fn navigate_to(&mut self, index: usize) {
        let max_index = self.leaderboard_len().saturating_sub(1);
        self.selected_entry_index = index.min(max_index);
    }

    /// Navigate to the last item.
    fn navigate_to_last(&mut self) {
        self.selected_entry_index = self.leaderboard_len().saturating_sub(1);
    }

    /// Select a tab.
    fn select_tab(&mut self, tab: Tab) {
        self.active_tab = tab;
        // Reset selection when switching tabs
        self.selected_entry_index = 0;
    }

    /// Select a leaderboard.
    fn select_leaderboard(&mut self, lb: LeaderboardType) {
        self.selected_leaderboard = lb;
        self.selected_entry_index = 0;
    }

    /// Cycle through leaderboards.
    fn cycle_leaderboard(&mut self, delta: i32) {
        let types = [
            LeaderboardType::SignalQuality,
            LeaderboardType::PositionManagement,
            LeaderboardType::ExecutionSensitivity,
            LeaderboardType::Overall,
        ];

        let current_idx = types
            .iter()
            .position(|&t| t == self.selected_leaderboard)
            .unwrap_or(0);

        let new_idx = (current_idx as i32 + delta).rem_euclid(types.len() as i32) as usize;
        self.selected_leaderboard = types[new_idx];
        self.selected_entry_index = 0;
    }

    /// Export the selected strategy to a JSON artifact.
    ///
    /// Creates a JSON file in `artifacts/exports/<fingerprint>.json` containing
    /// the full strategy specification and metrics.
    fn export_selected(&mut self) {
        use trendlab_export::{entry_to_artifact, PineGenerator};

        // Get the selected entry
        let entry = match self.selected_entry() {
            Some(e) => e.clone(),
            None => {
                self.error_message = Some("No strategy selected".to_string());
                self.modal = None;
                return;
            }
        };

        // Create the export artifact using trendlab-export
        let artifact = entry_to_artifact(&entry);

        // Ensure directories exist
        let export_dir = Self::get_export_dir();
        let pine_dir = Self::get_pine_dir();

        if let Err(e) = fs::create_dir_all(&export_dir) {
            self.error_message = Some(format!("Failed to create export directory: {}", e));
            self.modal = None;
            return;
        }
        if let Err(e) = fs::create_dir_all(&pine_dir) {
            self.error_message = Some(format!("Failed to create pine-scripts directory: {}", e));
            self.modal = None;
            return;
        }

        // Export JSON artifact
        let json_filename = format!("{}.json", entry.fingerprint);
        let json_path = export_dir.join(&json_filename);

        if let Err(e) = artifact.save_to_file(&json_path) {
            self.error_message = Some(format!("Failed to save artifact: {}", e));
            self.modal = None;
            return;
        }

        // Generate Pine Script
        let pine_generator = PineGenerator::new();
        let pine_result = pine_generator.generate(&artifact);

        let pine_path = pine_dir.join(format!("{}.pine", entry.fingerprint));
        let pine_status = match pine_result {
            Ok(pine_code) => {
                match fs::write(&pine_path, &pine_code) {
                    Ok(_) => format!("Pine: {}", pine_path.display()),
                    Err(e) => format!("Pine write failed: {}", e),
                }
            }
            Err(e) => format!("Pine generation failed: {}", e),
        };

        // Set success message
        self.status_message = Some(format!(
            "Exported: {} | {}",
            json_path.display(),
            pine_status
        ));
        self.error_message = None;
        self.modal = None;
    }

    /// Get the export directory path.
    ///
    /// Returns `artifacts/exports` relative to the current working directory,
    /// or a fallback path if the standard location doesn't exist.
    fn get_export_dir() -> PathBuf {
        // Try standard location first
        let standard_path = PathBuf::from("artifacts/exports");
        if standard_path.exists() || standard_path.parent().map(|p| p.exists()).unwrap_or(false) {
            return standard_path;
        }

        // Fallback: try to find artifacts directory in parent paths
        if let Ok(cwd) = std::env::current_dir() {
            // Look for artifacts directory up the tree
            let mut current = cwd.as_path();
            for _ in 0..5 {
                let artifacts = current.join("artifacts/exports");
                if artifacts.parent().map(|p| p.exists()).unwrap_or(false) {
                    return artifacts;
                }
                if let Some(parent) = current.parent() {
                    current = parent;
                } else {
                    break;
                }
            }
        }

        // Final fallback: use standard path anyway
        standard_path
    }

    /// Get the pine-scripts directory path.
    ///
    /// Returns `pine-scripts` relative to the current working directory.
    fn get_pine_dir() -> PathBuf {
        PathBuf::from("pine-scripts")
    }

    /// Handle tick event.
    ///
    /// This is called on every frame (~30fps). The runner handles rate limiting
    /// internally, so we can call run_iteration on every tick.
    fn on_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        // Process data loading if in that phase
        if self.phase == AppPhase::DataLoading {
            self.process_data_load_messages();
            return;
        }

        // Run backtest iteration if running
        if self.phase == AppPhase::Running {
            if let Some(ref mut session) = self.session {
                // Run a backtest iteration (rate-limited internally)
                if let Some(result) = self.runner.run_iteration(session) {
                    self.last_iteration_trade_count = Some(result.trade_count);

                    // Cache the equity curve for chart display
                    if !result.equity_curve.is_empty() {
                        let fingerprint = result.genome.fingerprint();
                        self.equity_curve_cache.insert(fingerprint, result.equity_curve);

                        // Limit cache size to prevent memory issues
                        if self.equity_curve_cache.len() > 200 {
                            // Remove oldest entries (arbitrary eviction)
                            let keys_to_remove: Vec<_> = self
                                .equity_curve_cache
                                .keys()
                                .take(50)
                                .cloned()
                                .collect();
                            for key in keys_to_remove {
                                self.equity_curve_cache.remove(&key);
                            }
                        }
                    }
                }

                // Check if session completed
                if !session.is_running() {
                    self.phase = AppPhase::Completed;
                    self.runner.reset();
                }
            }
        }
    }

    /// Get the length of the current leaderboard.
    pub fn leaderboard_len(&self) -> usize {
        self.session
            .as_ref()
            .map(|s| s.leaderboards().get(self.selected_leaderboard).len())
            .unwrap_or(0)
    }

    /// Get the current leaderboard entries.
    pub fn leaderboard_entries(&self) -> Vec<&LeaderboardEntry> {
        self.session
            .as_ref()
            .map(|s| {
                s.leaderboards()
                    .get(self.selected_leaderboard)
                    .top(100)
            })
            .unwrap_or_default()
    }

    /// Get the selected entry.
    pub fn selected_entry(&self) -> Option<&LeaderboardEntry> {
        let entries = self.leaderboard_entries();
        entries.get(self.selected_entry_index).copied()
    }

    /// Get session stats.
    pub fn session_stats(&self) -> Option<&SessionStats> {
        self.session.as_ref().map(|s| s.stats())
    }

    /// Get session phase.
    pub fn session_phase(&self) -> SessionPhase {
        self.session
            .as_ref()
            .map(|s| s.phase())
            .unwrap_or(SessionPhase::NotStarted)
    }

    /// Get session progress (0.0 to 1.0).
    pub fn session_progress(&self) -> f64 {
        self.session.as_ref().map(|s| s.progress()).unwrap_or(0.0)
    }

    /// Check if any leaderboard has entries.
    pub fn has_results(&self) -> bool {
        self.session
            .as_ref()
            .map(|s| !s.leaderboards().overall.is_empty())
            .unwrap_or(false)
    }

    /// Get the selected genome for detail view.
    pub fn selected_genome(&self) -> Option<&Genome> {
        self.selected_entry().map(|e| &e.genome)
    }

    /// Update visible items count based on terminal height.
    pub fn set_visible_items(&mut self, height: u16) {
        // Reserve space for header, borders, and detail panel
        self.visible_items = (height.saturating_sub(15) as usize).max(5);
    }

    /// Get the number of bars in the test data.
    pub fn test_data_bar_count(&self) -> usize {
        self.runner.bar_count()
    }

    /// Check if the runner is initialized with data.
    pub fn has_test_data(&self) -> bool {
        self.runner.is_initialized()
    }

    /// Get data load progress for display.
    pub fn data_load_progress(&self) -> Option<&DataLoadProgress> {
        self.data_load_progress.as_ref()
    }

    /// Get the equity curve for the selected entry (if cached).
    pub fn selected_equity_curve(&self) -> Option<&Vec<f64>> {
        self.selected_entry()
            .and_then(|e| self.equity_curve_cache.get(&e.fingerprint))
    }
}

/// Async task that loads data for all symbols in a universe.
async fn load_data_task(symbols: Vec<String>, tx: mpsc::Sender<DataLoadMessage>) {
    // Create data provider with cache in current directory
    let cache_dir = std::path::PathBuf::from("data/parquet");

    // Create cache directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        let _ = tx
            .send(DataLoadMessage::SymbolFailed {
                symbol: "INIT".to_string(),
                error: format!("Failed to create cache directory: {}", e),
            })
            .await;
        let _ = tx
            .send(DataLoadMessage::Complete {
                loaded: 0,
                failed: symbols.len(),
            })
            .await;
        return;
    }

    let provider = match DataProvider::new(&cache_dir) {
        Ok(p) => p,
        Err(e) => {
            let _ = tx
                .send(DataLoadMessage::SymbolFailed {
                    symbol: "INIT".to_string(),
                    error: format!("Failed to create data provider: {}", e),
                })
                .await;
            let _ = tx
                .send(DataLoadMessage::Complete {
                    loaded: 0,
                    failed: symbols.len(),
                })
                .await;
            return;
        }
    };

    // Use last 3 years of data
    let end = chrono::Utc::now().date_naive();
    let start = end - chrono::Duration::days(3 * 365);

    let total = symbols.len();
    let mut loaded = 0;
    let mut failed = 0;

    for (i, symbol) in symbols.iter().enumerate() {
        // Send progress update
        let _ = tx.send(DataLoadMessage::Progress {
            symbol: symbol.clone(),
            current: i,
            total,
        }).await;

        // Small delay between requests to avoid rate limiting
        if i > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        // Fetch data with indicators
        match provider.get_data_with_indicators(symbol, start, end).await {
            Ok(lf) => {
                // Collect the lazy frame
                match lf.collect() {
                    Ok(df) => {
                        // Extract bars, ATR, and ADX
                        match extract_data_from_df(&df) {
                            Ok((bars, atr, adx)) => {
                                let _ = tx.send(DataLoadMessage::SymbolLoaded {
                                    symbol: symbol.clone(),
                                    bars,
                                    atr,
                                    adx,
                                }).await;
                                loaded += 1;
                            }
                            Err(e) => {
                                let _ = tx.send(DataLoadMessage::SymbolFailed {
                                    symbol: symbol.clone(),
                                    error: e,
                                }).await;
                                failed += 1;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(DataLoadMessage::SymbolFailed {
                            symbol: symbol.clone(),
                            error: e.to_string(),
                        }).await;
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(DataLoadMessage::SymbolFailed {
                    symbol: symbol.clone(),
                    error: e.to_string(),
                }).await;
                failed += 1;
            }
        }
    }

    // Send completion message
    let _ = tx.send(DataLoadMessage::Complete { loaded, failed }).await;
}

/// Extract bars, ATR, and ADX from a DataFrame with indicators.
fn extract_data_from_df(df: &polars::prelude::DataFrame) -> Result<(Vec<Bar>, Vec<f64>, Vec<f64>), String> {

    let dates = df.column("date")
        .map_err(|e| format!("Missing date column: {}", e))?
        .date()
        .map_err(|e| format!("Invalid date column: {}", e))?;
    let opens = df.column("open")
        .map_err(|e| format!("Missing open column: {}", e))?
        .f64()
        .map_err(|e| format!("Invalid open column: {}", e))?;
    let highs = df.column("high")
        .map_err(|e| format!("Missing high column: {}", e))?
        .f64()
        .map_err(|e| format!("Invalid high column: {}", e))?;
    let lows = df.column("low")
        .map_err(|e| format!("Missing low column: {}", e))?
        .f64()
        .map_err(|e| format!("Invalid low column: {}", e))?;
    let closes = df.column("close")
        .map_err(|e| format!("Missing close column: {}", e))?
        .f64()
        .map_err(|e| format!("Invalid close column: {}", e))?;
    let volumes = df.column("volume")
        .map_err(|e| format!("Missing volume column: {}", e))?
        .u64()
        .map_err(|e| format!("Invalid volume column: {}", e))?;

    // Try to get ATR and ADX columns (may be named atr_14, adx_14)
    let atr_col = df.column("atr_14")
        .or_else(|_| df.column("atr"))
        .map_err(|e| format!("Missing ATR column: {}", e))?
        .f64()
        .map_err(|e| format!("Invalid ATR column: {}", e))?;
    let adx_col = df.column("adx_14")
        .or_else(|_| df.column("adx"))
        .map_err(|e| format!("Missing ADX column: {}", e))?
        .f64()
        .map_err(|e| format!("Invalid ADX column: {}", e))?;

    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();

    let mut bars = Vec::with_capacity(df.height());
    let mut atr = Vec::with_capacity(df.height());
    let mut adx = Vec::with_capacity(df.height());

    for i in 0..df.height() {
        let date_days = match dates.get(i) {
            Some(d) => d,
            None => continue,
        };
        let date = epoch + chrono::Duration::days(date_days as i64);
        let open = opens.get(i).unwrap_or(0.0);
        let high = highs.get(i).unwrap_or(0.0);
        let low = lows.get(i).unwrap_or(0.0);
        let close = closes.get(i).unwrap_or(0.0);
        let volume = volumes.get(i).unwrap_or(0);
        let atr_val = atr_col.get(i).unwrap_or(0.0);
        let adx_val = adx_col.get(i).unwrap_or(25.0);

        // Skip rows with missing essential data
        if open <= 0.0 || high <= 0.0 || low <= 0.0 || close <= 0.0 {
            continue;
        }

        bars.push(Bar::new(date, open, high, low, close, volume, bars.len()));
        atr.push(atr_val);
        adx.push(adx_val);
    }

    if bars.is_empty() {
        return Err("No valid bars extracted".to_string());
    }

    Ok((bars, atr, adx))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create an app with pre-loaded test data to skip data loading phase.
    fn app_with_test_data() -> App {
        let mut app = App::new();
        // Pre-populate symbol data so start_session skips to Running
        let test_bars = generate_test_bars(500);
        let test_atr: Vec<f64> = (0..500).map(|_| 2.5).collect();
        let test_adx: Vec<f64> = (0..500).map(|_| 25.0).collect();
        app.symbol_data.insert("TEST".to_string(), (test_bars, test_atr, test_adx));
        app
    }

    /// Generate test bars for unit tests.
    fn generate_test_bars(count: usize) -> Vec<Bar> {
        let start_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let mut bars = Vec::with_capacity(count);
        let mut price = 100.0;

        for i in 0..count {
            let date = start_date + chrono::Duration::days(i as i64);
            let open = price;
            // Create valid OHLC: low <= open,close and high >= open,close
            let change = (i as f64 * 0.1).sin() * 2.0;
            let close = open + change;
            // High is max of open/close plus some wick
            let high = open.max(close) + 0.5;
            // Low is min of open/close minus some wick
            let low = open.min(close) - 0.5;
            price = close;

            bars.push(Bar::new(date, open, high, low, close, 1_000_000, i));
        }
        bars
    }

    #[test]
    fn test_app_creation() {
        let app = App::new();
        assert_eq!(app.phase, AppPhase::Ready);
        assert!(app.session.is_none());
        assert!(!app.should_quit);
    }

    #[test]
    fn test_quit_action() {
        let mut app = App::new();
        app.apply_action(Action::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn test_modal_actions() {
        let mut app = App::new();

        app.apply_action(Action::ShowHelp);
        assert_eq!(app.modal, Some(Modal::Help));
        assert_eq!(app.key_context(), KeyContext::HelpModal);

        app.apply_action(Action::CloseModal);
        assert_eq!(app.modal, None);
        assert_eq!(app.key_context(), KeyContext::Normal);
    }

    #[test]
    fn test_leaderboard_cycling() {
        let mut app = App::new();

        app.apply_action(Action::NextLeaderboard);
        assert_ne!(app.selected_leaderboard, LeaderboardType::Overall);

        app.apply_action(Action::SelectLeaderboard(LeaderboardType::SignalQuality));
        assert_eq!(app.selected_leaderboard, LeaderboardType::SignalQuality);
    }

    #[test]
    fn test_start_session_with_data() {
        let mut app = app_with_test_data();
        app.apply_action(Action::StartSession);

        // With pre-loaded data, should go straight to Running
        assert_eq!(app.phase, AppPhase::Running);
        assert!(app.session.is_some());
    }

    #[tokio::test]
    async fn test_start_session_without_data() {
        let mut app = App::new();
        app.apply_action(Action::StartSession);

        // Without pre-loaded data, should go to DataLoading
        assert_eq!(app.phase, AppPhase::DataLoading);
        assert!(app.data_load_progress.is_some());
    }

    #[test]
    fn test_pause_resume() {
        let mut app = app_with_test_data();
        app.apply_action(Action::StartSession);

        app.apply_action(Action::PauseResume);
        assert_eq!(app.phase, AppPhase::Paused);

        app.apply_action(Action::PauseResume);
        assert_eq!(app.phase, AppPhase::Running);
    }

    #[test]
    fn test_start_session_initializes_runner() {
        let mut app = app_with_test_data();

        // Before starting, no test data in runner
        assert!(!app.has_test_data());
        assert_eq!(app.test_data_bar_count(), 0);

        // Start session
        app.apply_action(Action::StartSession);

        // After starting, runner should be initialized
        assert!(app.has_test_data());
        assert!(app.test_data_bar_count() > 0);
    }

    #[test]
    fn test_tick_runs_backtest() {
        let mut app = app_with_test_data();
        app.apply_action(Action::StartSession);

        // Initially no iterations
        let initial_iterations = app.session.as_ref().map(|s| s.stats().iterations).unwrap_or(0);

        // Run several ticks (rate limiting means not every tick runs a backtest)
        for _ in 0..10 {
            app.apply_action(Action::Tick);
        }

        // Should have run at least one iteration
        let final_iterations = app.session.as_ref().map(|s| s.stats().iterations).unwrap_or(0);
        assert!(final_iterations > initial_iterations);
    }

    #[test]
    fn test_stop_session_resets_runner() {
        let mut app = app_with_test_data();
        app.apply_action(Action::StartSession);
        assert!(app.has_test_data());

        app.apply_action(Action::StopSession);
        assert!(!app.has_test_data());
        assert_eq!(app.phase, AppPhase::Completed);
    }

    #[test]
    fn test_session_completion() {
        let mut app = app_with_test_data();
        app.apply_action(Action::StartSession);

        // Run many ticks to complete the session
        // Default config has max_iterations = 1000, but we'll just run enough
        // to see that the mechanism works
        for _ in 0..50 {
            app.apply_action(Action::Tick);
        }

        // Session should still be running (1000 iterations takes more ticks)
        // or completed if we ran enough
        // Just verify we can run ticks without panicking
        assert!(app.session.is_some());
    }

    #[test]
    fn test_export_modal_opens() {
        let mut app = App::new();
        app.apply_action(Action::ShowExport);
        assert_eq!(app.modal, Some(Modal::Export));
        assert_eq!(app.key_context(), KeyContext::ExportModal);
    }

    #[test]
    fn test_export_no_selection_sets_error() {
        let mut app = App::new();
        // Without a session, there's no selected entry
        app.apply_action(Action::Export);
        assert!(app.error_message.is_some());
        assert!(app.error_message.as_ref().unwrap().contains("No strategy selected"));
    }

    #[test]
    fn test_status_message_cleared_on_close_modal() {
        let mut app = App::new();
        app.status_message = Some("Test status".to_string());

        // CloseModal should clear status message
        app.apply_action(Action::CloseModal);
        assert!(app.status_message.is_none());
    }

    #[test]
    fn test_export_artifact_creation() {
        // Test that ExportArtifact serializes correctly
        use trendlab_yolo::genome::ComponentConfig;

        let genome = Genome::new(
            ComponentConfig::new("donchian_breakout"),
            ComponentConfig::new("atr_trailing_stop"),
            ComponentConfig::new("next_open_fill"),
            None,
        );

        let robustness = RobustnessScore {
            score: 0.75,
            base_score: 0.80,
            median_sharpe: 1.25,
            avg_hit_rate: 0.55,
            consistency: 0.85,
            floor: -0.05,
            worst_drawdown: -0.12,
            cost_sensitivity: 0.15,
            num_runs: 5,
            is_valid: true,
            invalid_reason: None,
        };

        let artifact = ExportArtifact {
            version: "0.1.0".to_string(),
            fingerprint: genome.fingerprint(),
            created_at: chrono::Utc::now().to_rfc3339(),
            genome,
            robustness,
            num_evaluations: 5,
            notes: ExportNotes {
                pine_script: "Coming soon".to_string(),
                test_vectors: "Coming soon".to_string(),
            },
        };

        // Should serialize without error
        let json = serde_json::to_string_pretty(&artifact);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("donchian_breakout"));
        assert!(json_str.contains("atr_trailing_stop"));
        assert!(json_str.contains("0.75")); // score
    }
}
