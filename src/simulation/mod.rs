// Simulation Engine Module
// Provides realistic local exchange simulation using live Kraken data

pub mod order_book;
pub mod matching_engine;
pub mod execution_simulator;
pub mod simulation_engine;
pub mod adapter;

pub use order_book::{LocalOrderBook, OrderBookSnapshot, OrderBookUpdate};
pub use matching_engine::{OrderMatchingEngine, MatchResult, FillInfo};
pub use execution_simulator::{ExecutionSimulator, ExecutionResult, SlippageModel};
pub use simulation_engine::{SimulationEngine, SimulationConfig};
pub use adapter::SimulationAdapter;
