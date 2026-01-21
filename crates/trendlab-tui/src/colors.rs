//! Semantic color system for consistent visual meaning.
//!
//! Colors convey meaning, not just aesthetics:
//! - Green: success, gains, completion
//! - Red: danger, losses, errors
//! - Yellow: warnings, warmup phase
//! - Blue: info, exploitation phase
//! - Gray: muted, disabled, secondary

use ratatui::style::Color;

/// Semantic color palette for the TUI.
///
/// These colors are chosen to convey meaning across all panels.
/// The palette is colorblind-friendly with sufficient contrast.
pub struct Colors;

impl Colors {
    // === Primary Semantic Colors ===

    /// Success color - green for gains, completed, valid
    pub const fn success() -> Color {
        Color::Rgb(0, 200, 83) // Bright green
    }

    /// Danger color - red for losses, errors, invalid
    pub const fn danger() -> Color {
        Color::Rgb(255, 82, 82) // Bright red
    }

    /// Warning color - yellow for warnings, warmup phase
    pub const fn warning() -> Color {
        Color::Rgb(255, 193, 7) // Amber yellow
    }

    /// Info color - blue for information, exploitation phase
    pub const fn info() -> Color {
        Color::Rgb(66, 165, 245) // Light blue
    }

    /// Muted color - gray for disabled, secondary text
    pub const fn muted() -> Color {
        Color::Rgb(128, 128, 128) // Medium gray
    }

    /// Accent color - for highlights, selection
    pub const fn accent() -> Color {
        Color::Rgb(156, 39, 176) // Purple accent
    }

    // === UI Element Colors ===

    /// Panel border when focused
    pub const fn border_focused() -> Color {
        Color::Rgb(92, 159, 255) // Bright blue
    }

    /// Panel border when unfocused
    pub const fn border_unfocused() -> Color {
        Color::Rgb(74, 74, 74) // Dim gray
    }

    /// Selection background
    pub const fn selection_bg() -> Color {
        Color::Rgb(0, 206, 209) // Cyan
    }

    /// Selection text
    pub const fn selection_fg() -> Color {
        Color::Black
    }

    /// Tab active background
    pub const fn tab_active() -> Color {
        Color::Rgb(92, 159, 255)
    }

    /// Tab inactive text
    pub const fn tab_inactive() -> Color {
        Color::Rgb(128, 128, 128)
    }

    // === Phase-Specific Colors ===

    /// Not started phase
    pub const fn phase_not_started() -> Color {
        Color::Rgb(128, 128, 128) // Gray
    }

    /// Warmup phase
    pub const fn phase_warmup() -> Color {
        Color::Rgb(255, 193, 7) // Yellow
    }

    /// Exploitation phase
    pub const fn phase_exploitation() -> Color {
        Color::Rgb(66, 165, 245) // Blue
    }

    /// Completed phase
    pub const fn phase_completed() -> Color {
        Color::Rgb(0, 200, 83) // Green
    }

    // === Metric Colors ===

    /// Color for positive Sharpe ratio (> 0.3)
    pub const fn sharpe_good() -> Color {
        Color::Rgb(0, 200, 83) // Green
    }

    /// Color for neutral Sharpe ratio (0 to 0.3)
    pub const fn sharpe_neutral() -> Color {
        Color::Rgb(255, 193, 7) // Yellow
    }

    /// Color for negative Sharpe ratio (< 0)
    pub const fn sharpe_bad() -> Color {
        Color::Rgb(255, 82, 82) // Red
    }

    /// Color based on Sharpe value
    pub fn for_sharpe(sharpe: f64) -> Color {
        if sharpe > 0.3 {
            Self::sharpe_good()
        } else if sharpe >= 0.0 {
            Self::sharpe_neutral()
        } else {
            Self::sharpe_bad()
        }
    }

    /// Color based on drawdown percentage (negative value)
    pub fn for_drawdown(dd: f64) -> Color {
        let abs_dd = dd.abs();
        if abs_dd > 0.30 {
            Self::danger()
        } else if abs_dd > 0.15 {
            Self::warning()
        } else {
            Self::muted()
        }
    }

    /// Color based on win rate (0.0 to 1.0)
    pub fn for_win_rate(rate: f64) -> Color {
        if rate > 0.50 {
            Self::success()
        } else if rate > 0.40 {
            Self::warning()
        } else {
            Self::danger()
        }
    }

    /// Color based on return percentage
    pub fn for_return(ret: f64) -> Color {
        if ret > 0.0 {
            Self::success()
        } else if ret == 0.0 {
            Self::muted()
        } else {
            Self::danger()
        }
    }

    // === Status Indicator Colors ===

    /// Data cached status (green dot)
    pub const fn status_cached() -> Color {
        Color::Rgb(0, 200, 83)
    }

    /// Data stale status (yellow dot)
    pub const fn status_stale() -> Color {
        Color::Rgb(255, 193, 7)
    }

    /// Data missing status (red dot)
    pub const fn status_missing() -> Color {
        Color::Rgb(255, 82, 82)
    }

    /// Running spinner (cyan)
    pub const fn status_running() -> Color {
        Color::Rgb(0, 206, 209)
    }

    // === Help Panel Colors ===

    /// Keyboard shortcut color
    pub const fn help_key() -> Color {
        Color::Rgb(0, 200, 83) // Green
    }

    /// Section header color
    pub const fn help_header() -> Color {
        Color::Rgb(255, 0, 255) // Magenta
    }

    /// Normal body text
    pub const fn help_body() -> Color {
        Color::White
    }

    // === Progress Bar Colors ===

    /// Progress bar filled portion
    pub const fn progress_filled() -> Color {
        Color::Rgb(0, 206, 209) // Cyan
    }

    /// Progress bar empty portion
    pub const fn progress_empty() -> Color {
        Color::Rgb(64, 64, 64) // Dark gray
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sharpe_colors() {
        assert_eq!(Colors::for_sharpe(0.5), Colors::sharpe_good());
        assert_eq!(Colors::for_sharpe(0.1), Colors::sharpe_neutral());
        assert_eq!(Colors::for_sharpe(-0.2), Colors::sharpe_bad());
    }

    #[test]
    fn test_drawdown_colors() {
        assert_eq!(Colors::for_drawdown(-0.35), Colors::danger());
        assert_eq!(Colors::for_drawdown(-0.20), Colors::warning());
        assert_eq!(Colors::for_drawdown(-0.10), Colors::muted());
    }

    #[test]
    fn test_win_rate_colors() {
        assert_eq!(Colors::for_win_rate(0.55), Colors::success());
        assert_eq!(Colors::for_win_rate(0.45), Colors::warning());
        assert_eq!(Colors::for_win_rate(0.35), Colors::danger());
    }
}
