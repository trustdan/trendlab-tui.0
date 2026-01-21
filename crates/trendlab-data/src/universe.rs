//! Symbol universe management.
//!
//! Provides predefined universes (US30, SP100, etc.) and custom universe support.

use serde::{Deserialize, Serialize};

/// A universe of tradeable symbols.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Universe {
    /// Universe name (e.g., "US30", "SP100").
    pub name: String,
    /// Description of the universe.
    pub description: String,
    /// Symbols in the universe.
    pub symbols: Vec<String>,
}

impl Universe {
    /// Create a custom universe.
    pub fn custom(name: impl Into<String>, symbols: Vec<String>) -> Self {
        Self {
            name: name.into(),
            description: "Custom universe".to_string(),
            symbols,
        }
    }

    /// US30 (Dow Jones Industrial Average components).
    pub fn us30() -> Self {
        Self {
            name: "US30".to_string(),
            description: "Dow Jones Industrial Average components".to_string(),
            symbols: vec![
                "AAPL", "AMGN", "AXP", "BA", "CAT", "CRM", "CSCO", "CVX", "DIS", "DOW",
                "GS", "HD", "HON", "IBM", "INTC", "JNJ", "JPM", "KO", "MCD", "MMM",
                "MRK", "MSFT", "NKE", "PG", "TRV", "UNH", "V", "VZ", "WBA", "WMT",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }

    /// SP100 (S&P 100 components).
    pub fn sp100() -> Self {
        Self {
            name: "SP100".to_string(),
            description: "S&P 100 components".to_string(),
            symbols: vec![
                "AAPL", "ABBV", "ABT", "ACN", "ADBE", "AIG", "AMD", "AMGN", "AMZN", "AXP",
                "BA", "BAC", "BK", "BKNG", "BLK", "BMY", "C", "CAT", "CHTR", "CL",
                "CMCSA", "COF", "COP", "COST", "CRM", "CSCO", "CVS", "CVX", "DE", "DHR",
                "DIS", "DOW", "DUK", "EMR", "EXC", "F", "FDX", "GD", "GE", "GILD",
                "GM", "GOOG", "GOOGL", "GS", "HD", "HON", "IBM", "INTC", "JNJ", "JPM",
                "KHC", "KO", "LIN", "LLY", "LMT", "LOW", "MA", "MCD", "MDLZ", "MDT",
                "MET", "META", "MMM", "MO", "MRK", "MS", "MSFT", "NEE", "NFLX", "NKE",
                "NVDA", "ORCL", "PEP", "PFE", "PG", "PM", "PYPL", "QCOM", "RTX", "SBUX",
                "SCHW", "SO", "SPG", "T", "TGT", "TMO", "TMUS", "TRV", "TXN", "UNH",
                "UNP", "UPS", "USB", "V", "VZ", "WFC", "WMT", "XOM",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }

    /// Futures universe (common futures symbols).
    pub fn futures() -> Self {
        Self {
            name: "FUTURES".to_string(),
            description: "Common futures contracts".to_string(),
            symbols: vec![
                "ES=F",  // E-mini S&P 500
                "NQ=F",  // E-mini NASDAQ 100
                "YM=F",  // E-mini Dow
                "RTY=F", // E-mini Russell 2000
                "CL=F",  // Crude Oil
                "GC=F",  // Gold
                "SI=F",  // Silver
                "HG=F",  // Copper
                "NG=F",  // Natural Gas
                "ZC=F",  // Corn
                "ZS=F",  // Soybeans
                "ZW=F",  // Wheat
                "6E=F",  // Euro FX
                "6J=F",  // Japanese Yen
                "6B=F",  // British Pound
                "ZN=F",  // 10-Year T-Note
                "ZB=F",  // 30-Year T-Bond
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }

    /// ETF universe (major ETFs).
    pub fn etfs() -> Self {
        Self {
            name: "ETFS".to_string(),
            description: "Major ETFs".to_string(),
            symbols: vec![
                "SPY", "QQQ", "IWM", "DIA", "EEM", "VTI", "VEA", "VWO", "GLD", "SLV",
                "TLT", "IEF", "LQD", "HYG", "XLF", "XLK", "XLE", "XLV", "XLI", "XLY",
                "XLP", "XLU", "XLB", "XLRE", "XBI", "IBB", "VNQ", "USO", "UNG", "DBC",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }

    /// Small test universe for development.
    pub fn test() -> Self {
        Self {
            name: "TEST".to_string(),
            description: "Small test universe".to_string(),
            symbols: vec!["AAPL", "MSFT", "GOOGL", "AMZN", "META"]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }

    /// Single symbol universe for focused testing.
    pub fn single(symbol: impl Into<String>) -> Self {
        let sym = symbol.into();
        Self {
            name: format!("SINGLE:{}", sym),
            description: format!("Single symbol: {}", sym),
            symbols: vec![sym],
        }
    }

    /// Number of symbols in the universe.
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Check if the universe is empty.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Check if a symbol is in the universe.
    pub fn contains(&self, symbol: &str) -> bool {
        self.symbols.iter().any(|s| s.eq_ignore_ascii_case(symbol))
    }

    /// Iterate over symbols.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.symbols.iter().map(|s| s.as_str())
    }
}

/// Predefined universe identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UniverseId {
    /// Dow Jones Industrial Average (30 stocks).
    Us30,
    /// S&P 100 (100 stocks).
    Sp100,
    /// Common futures contracts.
    Futures,
    /// Major ETFs.
    Etfs,
    /// Small test universe.
    Test,
}

impl UniverseId {
    /// Get the Universe for this ID.
    pub fn get(&self) -> Universe {
        match self {
            UniverseId::Us30 => Universe::us30(),
            UniverseId::Sp100 => Universe::sp100(),
            UniverseId::Futures => Universe::futures(),
            UniverseId::Etfs => Universe::etfs(),
            UniverseId::Test => Universe::test(),
        }
    }

    /// Get the display name.
    pub fn name(&self) -> &'static str {
        match self {
            UniverseId::Us30 => "US30",
            UniverseId::Sp100 => "SP100",
            UniverseId::Futures => "FUTURES",
            UniverseId::Etfs => "ETFS",
            UniverseId::Test => "TEST",
        }
    }

    /// All available universe IDs.
    pub fn all() -> &'static [UniverseId] {
        &[
            UniverseId::Us30,
            UniverseId::Sp100,
            UniverseId::Futures,
            UniverseId::Etfs,
            UniverseId::Test,
        ]
    }
}

impl std::fmt::Display for UniverseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_us30_size() {
        let u = Universe::us30();
        assert_eq!(u.len(), 30);
        assert!(u.contains("AAPL"));
        assert!(u.contains("aapl")); // case insensitive
    }

    #[test]
    fn test_sp100_size() {
        let u = Universe::sp100();
        assert!(u.len() >= 90); // Allow for some delisting
        assert!(u.contains("MSFT"));
    }

    #[test]
    fn test_custom_universe() {
        let u = Universe::custom("MyUniverse", vec!["FOO".into(), "BAR".into()]);
        assert_eq!(u.name, "MyUniverse");
        assert_eq!(u.len(), 2);
        assert!(u.contains("FOO"));
    }

    #[test]
    fn test_single_universe() {
        let u = Universe::single("TSLA");
        assert_eq!(u.len(), 1);
        assert!(u.contains("TSLA"));
    }

    #[test]
    fn test_universe_id_roundtrip() {
        for id in UniverseId::all() {
            let universe = id.get();
            assert!(!universe.is_empty());
        }
    }
}
