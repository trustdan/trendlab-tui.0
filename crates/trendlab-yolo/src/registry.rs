//! Component registry for creating component instances from IDs.
//!
//! The registry maintains a catalog of available components and can construct
//! instances from genome configurations. This enables structural Monte Carlo
//! sampling where we swap components dynamically.

use crate::error::YoloError;
use crate::genome::{ComponentConfig, ComponentId, Genome};
use std::collections::HashMap;
use trendlab_core::param::{ParamDef, ParamValue};
use trendlab_core::traits::{ExecutionModel, PositionManager, SignalFilter, SignalGenerator};
use trendlab_core::{
    AdxFilter, AroonCrossover, AtrTrailingStop, BollingerBreakout, BreakevenThenTrail,
    ChandelierExit, DonchianBreakout, DonchianExit, FiftyTwoWeekBreakout, FixedStop,
    KeltnerBreakout, KeltnerExit, MaCrossover, MaRegimeFilter, MaxHoldingPeriod, Momentum,
    NextOpenFill, NoFilter, ParabolicSar, PercentTrailing, RocMomentum, SarExit, Strategy,
    Supertrend, TimeDecayStop, TrendFlip, VolatilityFilter,
};

/// Type alias for signal generator factory functions.
type SignalGeneratorFactory =
    Box<dyn Fn(&HashMap<String, ParamValue>) -> Box<dyn SignalGenerator> + Send + Sync>;

/// Type alias for position manager factory functions.
type PositionManagerFactory =
    Box<dyn Fn(&HashMap<String, ParamValue>) -> Box<dyn PositionManager> + Send + Sync>;

/// Type alias for execution model factory functions.
type ExecutionModelFactory =
    Box<dyn Fn(&HashMap<String, ParamValue>) -> Box<dyn ExecutionModel> + Send + Sync>;

/// Type alias for signal filter factory functions.
type SignalFilterFactory =
    Box<dyn Fn(&HashMap<String, ParamValue>) -> Box<dyn SignalFilter> + Send + Sync>;

/// Metadata about a registered component.
#[derive(Clone)]
pub struct ComponentMeta {
    /// Component identifier
    pub id: ComponentId,
    /// Human-readable display name
    pub display_name: String,
    /// Brief description
    pub description: String,
    /// Parameter specifications
    pub params: Vec<ParamDef>,
}

/// Registry for creating component instances.
///
/// The registry maintains factories for all known component types and can
/// construct instances from genome configurations. This is the bridge between
/// the abstract genome representation and concrete component implementations.
///
/// # Usage
///
/// ```ignore
/// let registry = ComponentRegistry::with_defaults();
///
/// // Create a strategy from a genome
/// let strategy = registry.create_strategy(&genome)?;
///
/// // Get available signal generators
/// for sg in registry.signal_generators() {
///     println!("{}: {}", sg.id, sg.description);
/// }
/// ```
pub struct ComponentRegistry {
    signal_generators: HashMap<String, (ComponentMeta, SignalGeneratorFactory)>,
    position_managers: HashMap<String, (ComponentMeta, PositionManagerFactory)>,
    execution_models: HashMap<String, (ComponentMeta, ExecutionModelFactory)>,
    signal_filters: HashMap<String, (ComponentMeta, SignalFilterFactory)>,
}

impl ComponentRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            signal_generators: HashMap::new(),
            position_managers: HashMap::new(),
            execution_models: HashMap::new(),
            signal_filters: HashMap::new(),
        }
    }

    /// Create a registry with all default components.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register_defaults();
        registry
    }

    /// Register all default components.
    fn register_defaults(&mut self) {
        self.register_default_signal_generators();
        self.register_default_position_managers();
        self.register_default_execution_models();
        self.register_default_signal_filters();
    }

    fn register_default_signal_generators(&mut self) {
        // 1. Donchian Breakout
        self.register_signal_generator(
            ComponentMeta {
                id: ComponentId::new("donchian_breakout"),
                display_name: "Donchian Breakout".to_string(),
                description: "Classic channel breakout signal".to_string(),
                params: DonchianBreakout::default().parameter_spec(),
            },
            Box::new(|params| {
                let lookback = params
                    .get("lookback")
                    .and_then(|v| v.as_int())
                    .unwrap_or(20) as usize;
                let long_only = params
                    .get("long_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Box::new(DonchianBreakout::new(lookback, long_only))
            }),
        );

        // 2. 52-Week Breakout
        self.register_signal_generator(
            ComponentMeta {
                id: ComponentId::new("fifty_two_week_breakout"),
                display_name: "52-Week Breakout".to_string(),
                description: "Annual high/low breakout (Turtle style)".to_string(),
                params: FiftyTwoWeekBreakout::default().parameter_spec(),
            },
            Box::new(|params| {
                let lookback = params
                    .get("lookback")
                    .and_then(|v| v.as_int())
                    .unwrap_or(252) as usize;
                let long_only = params
                    .get("long_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                Box::new(FiftyTwoWeekBreakout::new(lookback, long_only))
            }),
        );

        // 3. MA Crossover
        self.register_signal_generator(
            ComponentMeta {
                id: ComponentId::new("ma_crossover"),
                display_name: "MA Crossover".to_string(),
                description: "Moving average golden/death cross".to_string(),
                params: MaCrossover::default().parameter_spec(),
            },
            Box::new(|params| {
                let fast = params
                    .get("fast_period")
                    .and_then(|v| v.as_int())
                    .unwrap_or(10) as usize;
                let slow = params
                    .get("slow_period")
                    .and_then(|v| v.as_int())
                    .unwrap_or(50) as usize;
                let long_only = params
                    .get("long_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Box::new(MaCrossover::new(fast, slow, long_only))
            }),
        );

        // 4. Supertrend
        self.register_signal_generator(
            ComponentMeta {
                id: ComponentId::new("supertrend"),
                display_name: "Supertrend".to_string(),
                description: "ATR-based trend following".to_string(),
                params: Supertrend::default().parameter_spec(),
            },
            Box::new(|params| {
                let multiplier = params
                    .get("atr_multiplier")
                    .and_then(|v| v.as_float())
                    .unwrap_or(3.0);
                let long_only = params
                    .get("long_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Box::new(Supertrend::new(multiplier, long_only))
            }),
        );

        // 5. Momentum (TSMOM)
        self.register_signal_generator(
            ComponentMeta {
                id: ComponentId::new("momentum"),
                display_name: "Momentum (TSMOM)".to_string(),
                description: "Time-series momentum".to_string(),
                params: Momentum::default().parameter_spec(),
            },
            Box::new(|params| {
                let lookback = params
                    .get("lookback")
                    .and_then(|v| v.as_int())
                    .unwrap_or(252) as usize;
                let threshold = params
                    .get("threshold")
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.0);
                let long_only = params
                    .get("long_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Box::new(Momentum::new(lookback, threshold, long_only))
            }),
        );

        // 6. Bollinger Breakout
        self.register_signal_generator(
            ComponentMeta {
                id: ComponentId::new("bollinger_breakout"),
                display_name: "Bollinger Breakout".to_string(),
                description: "Bollinger Bands breakout".to_string(),
                params: BollingerBreakout::default().parameter_spec(),
            },
            Box::new(|params| {
                let period = params
                    .get("period")
                    .and_then(|v| v.as_int())
                    .unwrap_or(20) as usize;
                let std_dev = params
                    .get("std_dev")
                    .and_then(|v| v.as_float())
                    .unwrap_or(2.0);
                let long_only = params
                    .get("long_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Box::new(BollingerBreakout::new(period, std_dev, long_only))
            }),
        );

        // 7. Keltner Breakout
        self.register_signal_generator(
            ComponentMeta {
                id: ComponentId::new("keltner_breakout"),
                display_name: "Keltner Breakout".to_string(),
                description: "Keltner Channel breakout".to_string(),
                params: KeltnerBreakout::default().parameter_spec(),
            },
            Box::new(|params| {
                let ema_period = params
                    .get("ema_period")
                    .and_then(|v| v.as_int())
                    .unwrap_or(20) as usize;
                let atr_mult = params
                    .get("atr_multiplier")
                    .and_then(|v| v.as_float())
                    .unwrap_or(2.0);
                let long_only = params
                    .get("long_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Box::new(KeltnerBreakout::new(ema_period, atr_mult, long_only))
            }),
        );

        // 8. Parabolic SAR
        self.register_signal_generator(
            ComponentMeta {
                id: ComponentId::new("parabolic_sar"),
                display_name: "Parabolic SAR".to_string(),
                description: "Parabolic Stop and Reverse".to_string(),
                params: ParabolicSar::default().parameter_spec(),
            },
            Box::new(|params| {
                let af_start = params
                    .get("af_start")
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.02);
                let af_max = params
                    .get("af_max")
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.20);
                let long_only = params
                    .get("long_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Box::new(ParabolicSar::new(af_start, af_max, long_only))
            }),
        );

        // 9. ROC Momentum
        self.register_signal_generator(
            ComponentMeta {
                id: ComponentId::new("roc_momentum"),
                display_name: "ROC Momentum".to_string(),
                description: "Rate of Change momentum".to_string(),
                params: RocMomentum::default().parameter_spec(),
            },
            Box::new(|params| {
                let period = params
                    .get("period")
                    .and_then(|v| v.as_int())
                    .unwrap_or(14) as usize;
                let threshold = params
                    .get("threshold")
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.0);
                let long_only = params
                    .get("long_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Box::new(RocMomentum::new(period, threshold, long_only))
            }),
        );

        // 10. Aroon Crossover
        self.register_signal_generator(
            ComponentMeta {
                id: ComponentId::new("aroon_crossover"),
                display_name: "Aroon Crossover".to_string(),
                description: "Aroon indicator crossover".to_string(),
                params: AroonCrossover::default().parameter_spec(),
            },
            Box::new(|params| {
                let period = params
                    .get("period")
                    .and_then(|v| v.as_int())
                    .unwrap_or(25) as usize;
                let long_only = params
                    .get("long_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Box::new(AroonCrossover::new(period, long_only))
            }),
        );

        // 11. Trend Flip
        self.register_signal_generator(
            ComponentMeta {
                id: ComponentId::new("trend_flip"),
                display_name: "Trend Flip".to_string(),
                description: "Trend reversal detection".to_string(),
                params: TrendFlip::default().parameter_spec(),
            },
            Box::new(|params| {
                let ma_period = params
                    .get("ma_period")
                    .and_then(|v| v.as_int())
                    .unwrap_or(50) as usize;
                let confirm_bars = params
                    .get("confirmation_bars")
                    .and_then(|v| v.as_int())
                    .unwrap_or(3) as usize;
                let long_only = params
                    .get("long_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Box::new(TrendFlip::new(ma_period, confirm_bars, long_only))
            }),
        );
    }

    fn register_default_position_managers(&mut self) {
        // 1. ATR Trailing Stop
        self.register_position_manager(
            ComponentMeta {
                id: ComponentId::new("atr_trailing_stop"),
                display_name: "ATR Trailing Stop".to_string(),
                description: "Volatility-adjusted trailing stop".to_string(),
                params: AtrTrailingStop::default().parameter_spec(),
            },
            Box::new(|params| {
                let multiplier = params
                    .get("atr_multiplier")
                    .and_then(|v| v.as_float())
                    .unwrap_or(3.0);
                Box::new(AtrTrailingStop::new(multiplier))
            }),
        );

        // 2. Chandelier Exit
        self.register_position_manager(
            ComponentMeta {
                id: ComponentId::new("chandelier_exit"),
                display_name: "Chandelier Exit".to_string(),
                description: "ATR trailing from lookback high/low".to_string(),
                params: ChandelierExit::default().parameter_spec(),
            },
            Box::new(|params| {
                let lookback = params
                    .get("lookback")
                    .and_then(|v| v.as_int())
                    .unwrap_or(22) as usize;
                let atr_multiplier = params
                    .get("atr_multiplier")
                    .and_then(|v| v.as_float())
                    .unwrap_or(3.0);
                Box::new(ChandelierExit::new(lookback, atr_multiplier))
            }),
        );

        // 3. Percent Trailing
        self.register_position_manager(
            ComponentMeta {
                id: ComponentId::new("percent_trailing"),
                display_name: "Percent Trailing".to_string(),
                description: "Simple percentage-based trailing stop".to_string(),
                params: PercentTrailing::default().parameter_spec(),
            },
            Box::new(|params| {
                let trail_percent = params
                    .get("trail_percent")
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.05);
                Box::new(PercentTrailing::new(trail_percent))
            }),
        );

        // 4. Fixed Stop
        self.register_position_manager(
            ComponentMeta {
                id: ComponentId::new("fixed_stop"),
                display_name: "Fixed Stop".to_string(),
                description: "Fixed stop loss and optional take profit".to_string(),
                params: FixedStop::default().parameter_spec(),
            },
            Box::new(|params| {
                let stop_percent = params
                    .get("stop_percent")
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.02);
                let target_percent = params
                    .get("target_percent")
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.0);
                Box::new(FixedStop::new(stop_percent, target_percent))
            }),
        );

        // 5. Max Holding Period
        self.register_position_manager(
            ComponentMeta {
                id: ComponentId::new("max_holding_period"),
                display_name: "Max Holding Period".to_string(),
                description: "Time-based exit after N bars".to_string(),
                params: MaxHoldingPeriod::default().parameter_spec(),
            },
            Box::new(|params| {
                let max_bars = params
                    .get("max_bars")
                    .and_then(|v| v.as_int())
                    .unwrap_or(20) as usize;
                Box::new(MaxHoldingPeriod::new(max_bars))
            }),
        );

        // 6. Time Decay Stop
        self.register_position_manager(
            ComponentMeta {
                id: ComponentId::new("time_decay_stop"),
                display_name: "Time Decay Stop".to_string(),
                description: "Stop tightens over holding period".to_string(),
                params: TimeDecayStop::default().parameter_spec(),
            },
            Box::new(|params| {
                let initial_atr_mult = params
                    .get("initial_atr_mult")
                    .and_then(|v| v.as_float())
                    .unwrap_or(3.0);
                let final_atr_mult = params
                    .get("final_atr_mult")
                    .and_then(|v| v.as_float())
                    .unwrap_or(1.0);
                let decay_bars = params
                    .get("decay_bars")
                    .and_then(|v| v.as_int())
                    .unwrap_or(20) as usize;
                Box::new(TimeDecayStop::new(initial_atr_mult, final_atr_mult, decay_bars))
            }),
        );

        // 7. Breakeven Then Trail
        self.register_position_manager(
            ComponentMeta {
                id: ComponentId::new("breakeven_then_trail"),
                display_name: "Breakeven Then Trail".to_string(),
                description: "Move to breakeven, then trail".to_string(),
                params: BreakevenThenTrail::default().parameter_spec(),
            },
            Box::new(|params| {
                let initial_stop_atr = params
                    .get("initial_stop_atr")
                    .and_then(|v| v.as_float())
                    .unwrap_or(2.0);
                let breakeven_threshold = params
                    .get("breakeven_threshold")
                    .and_then(|v| v.as_float())
                    .unwrap_or(1.5);
                let trail_atr = params
                    .get("trail_atr")
                    .and_then(|v| v.as_float())
                    .unwrap_or(1.5);
                Box::new(BreakevenThenTrail::new(initial_stop_atr, breakeven_threshold, trail_atr))
            }),
        );

        // 8. Keltner Exit
        self.register_position_manager(
            ComponentMeta {
                id: ComponentId::new("keltner_exit"),
                display_name: "Keltner Exit".to_string(),
                description: "Exit when price returns to channel".to_string(),
                params: KeltnerExit::default().parameter_spec(),
            },
            Box::new(|params| {
                let ema_period = params
                    .get("ema_period")
                    .and_then(|v| v.as_int())
                    .unwrap_or(20) as usize;
                let atr_multiplier = params
                    .get("atr_multiplier")
                    .and_then(|v| v.as_float())
                    .unwrap_or(2.0);
                let disaster_atr = params
                    .get("disaster_atr")
                    .and_then(|v| v.as_float())
                    .unwrap_or(4.0);
                Box::new(KeltnerExit::new(ema_period, atr_multiplier, disaster_atr))
            }),
        );

        // 9. Donchian Exit
        self.register_position_manager(
            ComponentMeta {
                id: ComponentId::new("donchian_exit"),
                display_name: "Donchian Exit".to_string(),
                description: "Exit on opposite channel break".to_string(),
                params: DonchianExit::default().parameter_spec(),
            },
            Box::new(|params| {
                let exit_lookback = params
                    .get("exit_lookback")
                    .and_then(|v| v.as_int())
                    .unwrap_or(10) as usize;
                Box::new(DonchianExit::new(exit_lookback))
            }),
        );

        // 10. SAR Exit
        self.register_position_manager(
            ComponentMeta {
                id: ComponentId::new("sar_exit"),
                display_name: "SAR Exit".to_string(),
                description: "Parabolic SAR trailing stop".to_string(),
                params: SarExit::default().parameter_spec(),
            },
            Box::new(|params| {
                let af_start = params
                    .get("af_start")
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.02);
                let af_max = params
                    .get("af_max")
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.20);
                Box::new(SarExit::new(af_start, af_max))
            }),
        );
    }

    fn register_default_execution_models(&mut self) {
        // Next Open Fill
        self.register_execution_model(
            ComponentMeta {
                id: ComponentId::new("next_open_fill"),
                display_name: "Next Open Fill".to_string(),
                description: "Fill at next bar's open with slippage".to_string(),
                params: NextOpenFill::default().parameter_spec(),
            },
            Box::new(|params| {
                let slippage_bps = params
                    .get("slippage_bps")
                    .and_then(|v| v.as_float())
                    .unwrap_or(5.0);
                let commission = params
                    .get("commission")
                    .and_then(|v| v.as_float())
                    .unwrap_or(1.0);
                Box::new(NextOpenFill::new(slippage_bps, commission))
            }),
        );

        // TODO: Add more execution models
        // - Close Fill
        // - Stop Order Fill
        // - Limit Order Fill
    }

    fn register_default_signal_filters(&mut self) {
        // No Filter (passthrough)
        self.register_signal_filter(
            ComponentMeta {
                id: ComponentId::new("no_filter"),
                display_name: "No Filter".to_string(),
                description: "Allow all signals".to_string(),
                params: vec![],
            },
            Box::new(|_params| Box::new(NoFilter)),
        );

        // ADX Trend Filter
        self.register_signal_filter(
            ComponentMeta {
                id: ComponentId::new("adx_filter"),
                display_name: "ADX Filter".to_string(),
                description: "Trend strength gating".to_string(),
                params: AdxFilter::default().parameter_spec(),
            },
            Box::new(|params| {
                let min_adx = params
                    .get("min_adx")
                    .and_then(|v| v.as_float())
                    .unwrap_or(25.0);
                let exit_threshold = params
                    .get("exit_threshold")
                    .and_then(|v| v.as_float())
                    .unwrap_or(15.0);
                Box::new(AdxFilter::new(min_adx, exit_threshold))
            }),
        );

        // MA Regime Filter
        self.register_signal_filter(
            ComponentMeta {
                id: ComponentId::new("ma_regime"),
                display_name: "MA Regime".to_string(),
                description: "Price vs MA alignment".to_string(),
                params: MaRegimeFilter::default().parameter_spec(),
            },
            Box::new(|params| {
                let period = params
                    .get("period")
                    .and_then(|v| v.as_int())
                    .unwrap_or(200) as usize;
                let require_alignment = params
                    .get("require_alignment")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                Box::new(MaRegimeFilter::new(period, require_alignment))
            }),
        );

        // Volatility Filter
        self.register_signal_filter(
            ComponentMeta {
                id: ComponentId::new("volatility_filter"),
                display_name: "Volatility Filter".to_string(),
                description: "ATR regime gating".to_string(),
                params: VolatilityFilter::default().parameter_spec(),
            },
            Box::new(|params| {
                let lookback = params
                    .get("lookback")
                    .and_then(|v| v.as_int())
                    .unwrap_or(20) as usize;
                let low_threshold = params
                    .get("low_threshold")
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.7);
                let high_threshold = params
                    .get("high_threshold")
                    .and_then(|v| v.as_float())
                    .unwrap_or(1.5);
                let allow_high = params
                    .get("allow_high")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Box::new(VolatilityFilter::new(
                    lookback,
                    low_threshold,
                    high_threshold,
                    true, // always allow low vol
                    allow_high,
                ))
            }),
        );
    }

    /// Register a signal generator factory.
    pub fn register_signal_generator(
        &mut self,
        meta: ComponentMeta,
        factory: SignalGeneratorFactory,
    ) {
        self.signal_generators
            .insert(meta.id.0.clone(), (meta, factory));
    }

    /// Register a position manager factory.
    pub fn register_position_manager(
        &mut self,
        meta: ComponentMeta,
        factory: PositionManagerFactory,
    ) {
        self.position_managers
            .insert(meta.id.0.clone(), (meta, factory));
    }

    /// Register an execution model factory.
    pub fn register_execution_model(
        &mut self,
        meta: ComponentMeta,
        factory: ExecutionModelFactory,
    ) {
        self.execution_models
            .insert(meta.id.0.clone(), (meta, factory));
    }

    /// Register a signal filter factory.
    pub fn register_signal_filter(&mut self, meta: ComponentMeta, factory: SignalFilterFactory) {
        self.signal_filters
            .insert(meta.id.0.clone(), (meta, factory));
    }

    /// Get all registered signal generator IDs.
    pub fn signal_generator_ids(&self) -> Vec<&ComponentId> {
        self.signal_generators
            .values()
            .map(|(meta, _)| &meta.id)
            .collect()
    }

    /// Get all registered position manager IDs.
    pub fn position_manager_ids(&self) -> Vec<&ComponentId> {
        self.position_managers
            .values()
            .map(|(meta, _)| &meta.id)
            .collect()
    }

    /// Get all registered execution model IDs.
    pub fn execution_model_ids(&self) -> Vec<&ComponentId> {
        self.execution_models
            .values()
            .map(|(meta, _)| &meta.id)
            .collect()
    }

    /// Get all registered signal filter IDs.
    pub fn signal_filter_ids(&self) -> Vec<&ComponentId> {
        self.signal_filters
            .values()
            .map(|(meta, _)| &meta.id)
            .collect()
    }

    /// Get metadata for all signal generators.
    pub fn signal_generators(&self) -> Vec<&ComponentMeta> {
        self.signal_generators.values().map(|(m, _)| m).collect()
    }

    /// Get metadata for all position managers.
    pub fn position_managers(&self) -> Vec<&ComponentMeta> {
        self.position_managers.values().map(|(m, _)| m).collect()
    }

    /// Get metadata for all execution models.
    pub fn execution_models(&self) -> Vec<&ComponentMeta> {
        self.execution_models.values().map(|(m, _)| m).collect()
    }

    /// Get metadata for all signal filters.
    pub fn signal_filters(&self) -> Vec<&ComponentMeta> {
        self.signal_filters.values().map(|(m, _)| m).collect()
    }

    /// Get parameter spec for a component.
    pub fn get_params(&self, id: &ComponentId) -> Option<Vec<ParamDef>> {
        if let Some((meta, _)) = self.signal_generators.get(&id.0) {
            return Some(meta.params.clone());
        }
        if let Some((meta, _)) = self.position_managers.get(&id.0) {
            return Some(meta.params.clone());
        }
        if let Some((meta, _)) = self.execution_models.get(&id.0) {
            return Some(meta.params.clone());
        }
        if let Some((meta, _)) = self.signal_filters.get(&id.0) {
            return Some(meta.params.clone());
        }
        None
    }

    /// Create a signal generator from configuration.
    pub fn create_signal_generator(
        &self,
        config: &ComponentConfig,
    ) -> Result<Box<dyn SignalGenerator>, YoloError> {
        let (_, factory) = self
            .signal_generators
            .get(&config.id.0)
            .ok_or_else(|| YoloError::UnknownComponent(config.id.0.clone()))?;

        Ok(factory(&config.params))
    }

    /// Create a position manager from configuration.
    pub fn create_position_manager(
        &self,
        config: &ComponentConfig,
    ) -> Result<Box<dyn PositionManager>, YoloError> {
        let (_, factory) = self
            .position_managers
            .get(&config.id.0)
            .ok_or_else(|| YoloError::UnknownComponent(config.id.0.clone()))?;

        Ok(factory(&config.params))
    }

    /// Create an execution model from configuration.
    pub fn create_execution_model(
        &self,
        config: &ComponentConfig,
    ) -> Result<Box<dyn ExecutionModel>, YoloError> {
        let (_, factory) = self
            .execution_models
            .get(&config.id.0)
            .ok_or_else(|| YoloError::UnknownComponent(config.id.0.clone()))?;

        Ok(factory(&config.params))
    }

    /// Create a signal filter from configuration.
    pub fn create_signal_filter(
        &self,
        config: &ComponentConfig,
    ) -> Result<Box<dyn SignalFilter>, YoloError> {
        let (_, factory) = self
            .signal_filters
            .get(&config.id.0)
            .ok_or_else(|| YoloError::UnknownComponent(config.id.0.clone()))?;

        Ok(factory(&config.params))
    }

    /// Create a complete Strategy from a Genome.
    ///
    /// This is the main entry point for turning genome configurations into
    /// runnable strategies.
    pub fn create_strategy(&self, genome: &Genome) -> Result<Strategy, YoloError> {
        let signal_generator = self.create_signal_generator(&genome.signal_generator)?;
        let position_manager = self.create_position_manager(&genome.position_manager)?;
        let execution_model = self.create_execution_model(&genome.execution_model)?;

        let signal_filter = if let Some(ref filter_config) = genome.signal_filter {
            // Skip "no_filter" to use None
            if filter_config.id.0 != "no_filter" {
                Some(self.create_signal_filter(filter_config)?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Strategy::new(
            signal_generator,
            position_manager,
            execution_model,
            signal_filter,
        ))
    }

    /// Count total structural combinations.
    ///
    /// This is the product of available components (excluding no_filter as separate).
    pub fn structural_space_size(&self) -> usize {
        let sg_count = self.signal_generators.len();
        let pm_count = self.position_managers.len();
        let em_count = self.execution_models.len();
        // +1 for "no filter" option
        let sf_count = self.signal_filters.len().max(1);

        sg_count * pm_count * em_count * sf_count
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trendlab_core::ParamValue;

    #[test]
    fn test_default_registry_has_components() {
        let registry = ComponentRegistry::with_defaults();

        assert!(!registry.signal_generator_ids().is_empty());
        assert!(!registry.position_manager_ids().is_empty());
        assert!(!registry.execution_model_ids().is_empty());
        assert!(!registry.signal_filter_ids().is_empty());
    }

    #[test]
    fn test_create_signal_generator() {
        let registry = ComponentRegistry::with_defaults();

        let config = ComponentConfig::new("donchian_breakout")
            .param("lookback", ParamValue::Int(30))
            .param("long_only", ParamValue::Bool(true));

        let sg = registry.create_signal_generator(&config).unwrap();
        assert_eq!(sg.name(), "DonchianBreakout");
        assert_eq!(sg.warmup_bars(), 30);
    }

    #[test]
    fn test_create_position_manager() {
        let registry = ComponentRegistry::with_defaults();

        let config = ComponentConfig::new("atr_trailing_stop")
            .param("atr_multiplier", ParamValue::Float(2.5));

        let pm = registry.create_position_manager(&config).unwrap();
        assert_eq!(pm.name(), "ATRTrailingStop");
    }

    #[test]
    fn test_create_strategy_from_genome() {
        let registry = ComponentRegistry::with_defaults();

        let genome = Genome::new(
            ComponentConfig::new("donchian_breakout"),
            ComponentConfig::new("atr_trailing_stop"),
            ComponentConfig::new("next_open_fill"),
            None,
        );

        let strategy = registry.create_strategy(&genome).unwrap();
        let names = strategy.component_names();

        assert_eq!(names.signal_generator, "DonchianBreakout");
        assert_eq!(names.position_manager, "ATRTrailingStop");
        assert_eq!(names.execution_model, "NextOpenFill");
        assert!(names.signal_filter.is_none());
    }

    #[test]
    fn test_unknown_component_error() {
        let registry = ComponentRegistry::with_defaults();

        let config = ComponentConfig::new("nonexistent");
        let result = registry.create_signal_generator(&config);

        assert!(result.is_err());
        match result {
            Err(YoloError::UnknownComponent(_)) => {}
            _ => panic!("Expected UnknownComponent error"),
        }
    }

    #[test]
    fn test_get_params() {
        let registry = ComponentRegistry::with_defaults();

        let params = registry
            .get_params(&ComponentId::new("donchian_breakout"))
            .unwrap();

        assert!(!params.is_empty());
        assert!(params.iter().any(|p| p.name == "lookback"));
    }

    #[test]
    fn test_structural_space_size() {
        let registry = ComponentRegistry::with_defaults();

        // Should be >= 1 with default components
        assert!(registry.structural_space_size() >= 1);
    }
}
