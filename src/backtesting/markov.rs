// Markov Chain Analysis for Market State Prediction

use std::collections::{HashMap, VecDeque};
use ndarray::Array2;
use crate::types::MarketState;
use crate::backtesting::BacktestConfig;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct MarkovChainAnalyzer {
    // Transition matrix: [from_state][to_state] = probability
    transition_matrix: Array2<f64>,
    state_history: VecDeque<MarketState>,
    state_counts: HashMap<(MarketState, MarketState), usize>,
    total_transitions: HashMap<MarketState, usize>,
    
    // Configuration
    lookback_periods: usize,
    smoothing_factor: f64,
    
    // Predictions
    next_state_probabilities: HashMap<MarketState, f64>,
    confidence_level: f64,
}

impl MarkovChainAnalyzer {
    pub fn new(config: &BacktestConfig) -> Self {
        let mut analyzer = Self {
            transition_matrix: Array2::zeros((3, 3)), // 3 states: TrendingUp, TrendingDown, Ranging
            state_history: VecDeque::with_capacity(config.markov_lookback_periods),
            state_counts: HashMap::new(),
            total_transitions: HashMap::new(),
            lookback_periods: config.markov_lookback_periods,
            smoothing_factor: config.state_transition_smoothing,
            next_state_probabilities: HashMap::new(),
            confidence_level: 0.0,
        };
        
        // Initialize with uniform prior probabilities
        analyzer.initialize_with_priors();
        analyzer
    }

    fn initialize_with_priors(&mut self) {
        // Initialize with reasonable prior probabilities based on market behavior
        // TrendingUp transitions
        self.transition_matrix[[0, 0]] = 0.6; // TrendingUp -> TrendingUp
        self.transition_matrix[[0, 1]] = 0.2; // TrendingUp -> TrendingDown
        self.transition_matrix[[0, 2]] = 0.2; // TrendingUp -> Ranging
        
        // TrendingDown transitions
        self.transition_matrix[[1, 0]] = 0.2; // TrendingDown -> TrendingUp
        self.transition_matrix[[1, 1]] = 0.6; // TrendingDown -> TrendingDown
        self.transition_matrix[[1, 2]] = 0.2; // TrendingDown -> Ranging
        
        // Ranging transitions
        self.transition_matrix[[2, 0]] = 0.25; // Ranging -> TrendingUp
        self.transition_matrix[[2, 1]] = 0.25; // Ranging -> TrendingDown
        self.transition_matrix[[2, 2]] = 0.5;  // Ranging -> Ranging
    }

    pub fn update_with_state(&mut self, new_state: MarketState) -> Option<MarketStatePrediction> {
        // Add transition if we have a previous state
        if let Some(&previous_state) = self.state_history.back() {
            self.record_transition(previous_state, new_state);
        }
        
        // Add new state to history
        self.state_history.push_back(new_state);
        
        // Keep only the lookback period
        if self.state_history.len() > self.lookback_periods {
            self.state_history.pop_front();
        }
        
        // Update transition matrix and make prediction
        self.update_transition_matrix();
        self.predict_next_state()
    }

    fn record_transition(&mut self, from_state: MarketState, to_state: MarketState) {
        *self.state_counts.entry((from_state, to_state)).or_insert(0) += 1;
        *self.total_transitions.entry(from_state).or_insert(0) += 1;
    }

    fn update_transition_matrix(&mut self) {
        // Update transition probabilities using Laplace smoothing
        for from_state in [MarketState::TrendingUp, MarketState::TrendingDown, MarketState::Ranging] {
            let total = self.total_transitions.get(&from_state).unwrap_or(&0);
            
            if *total > 0 {
                for to_state in [MarketState::TrendingUp, MarketState::TrendingDown, MarketState::Ranging] {
                    let count = self.state_counts.get(&(from_state, to_state)).unwrap_or(&0);
                    
                    // Laplace smoothing: (count + alpha) / (total + alpha * num_states)
                    let smoothed_prob = (*count as f64 + self.smoothing_factor) / 
                                      (*total as f64 + self.smoothing_factor * 3.0);
                    
                    let (from_idx, to_idx) = (self.state_to_index(from_state), self.state_to_index(to_state));
                    self.transition_matrix[[from_idx, to_idx]] = smoothed_prob;
                }
            }
        }
    }

    fn predict_next_state(&mut self) -> Option<MarketStatePrediction> {
        if let Some(&current_state) = self.state_history.back() {
            let current_idx = self.state_to_index(current_state);
            
            // Get probabilities for next state
            let prob_trending_up = self.transition_matrix[[current_idx, 0]];
            let prob_trending_down = self.transition_matrix[[current_idx, 1]];
            let prob_ranging = self.transition_matrix[[current_idx, 2]];
            
            self.next_state_probabilities.clear();
            self.next_state_probabilities.insert(MarketState::TrendingUp, prob_trending_up);
            self.next_state_probabilities.insert(MarketState::TrendingDown, prob_trending_down);
            self.next_state_probabilities.insert(MarketState::Ranging, prob_ranging);
            
            // Calculate confidence as the entropy of the distribution
            let entropy = -[prob_trending_up, prob_trending_down, prob_ranging]
                .iter()
                .filter(|&&p| p > 0.0)
                .map(|&p| p * p.ln())
                .sum::<f64>();
            
            self.confidence_level = 1.0 - (entropy / 3.0_f64.ln()); // Normalize by max entropy
            
            // Find most likely next state
            let most_likely_state = if prob_trending_up >= prob_trending_down && prob_trending_up >= prob_ranging {
                MarketState::TrendingUp
            } else if prob_trending_down >= prob_ranging {
                MarketState::TrendingDown
            } else {
                MarketState::Ranging
            };
            
            Some(MarketStatePrediction {
                current_state,
                predicted_state: most_likely_state,
                probabilities: self.next_state_probabilities.clone(),
                confidence: self.confidence_level,
                sample_size: self.total_transitions.get(&current_state).copied().unwrap_or(0),
            })
        } else {
            None
        }
    }

    pub fn get_adaptive_grid_spacing(&self, base_spacing: f64, _current_state: MarketState) -> f64 {
        if let Some(probabilities) = self.get_next_state_probabilities() {
            // Calculate expected volatility based on state predictions
            let trending_prob = probabilities.get(&MarketState::TrendingUp).unwrap_or(&0.0) + 
                              probabilities.get(&MarketState::TrendingDown).unwrap_or(&0.0);
            let ranging_prob = probabilities.get(&MarketState::Ranging).unwrap_or(&0.0);
            
            // Adjust spacing based on predicted market regime
            if trending_prob > 0.6 {
                // High probability of trending: wider spacing
                base_spacing * 1.5
            } else if *ranging_prob > 0.6 {
                // High probability of ranging: normal spacing
                base_spacing
            } else {
                // Uncertain regime: slightly wider spacing for safety
                base_spacing * 1.2
            }
        } else {
            base_spacing
        }
    }

    pub fn should_adjust_risk(&self, current_risk_level: f64) -> Option<f64> {
        if let Some(probabilities) = self.get_next_state_probabilities() {
            let volatility_states_prob = probabilities.get(&MarketState::TrendingUp).unwrap_or(&0.0) + 
                                       probabilities.get(&MarketState::TrendingDown).unwrap_or(&0.0);
            
            // Reduce risk if high probability of volatile states
            if volatility_states_prob > 0.7 && self.confidence_level > 0.6 {
                Some(current_risk_level * 0.7) // Reduce risk by 30%
            } else if *probabilities.get(&MarketState::Ranging).unwrap_or(&0.0) > 0.7 {
                Some(current_risk_level * 1.1) // Slightly increase risk in ranging markets
            } else {
                None // No adjustment needed
            }
        } else {
            None
        }
    }

    pub fn get_next_state_probabilities(&self) -> Option<&HashMap<MarketState, f64>> {
        if self.next_state_probabilities.is_empty() {
            None
        } else {
            Some(&self.next_state_probabilities)
        }
    }

    pub fn get_confidence_level(&self) -> f64 {
        self.confidence_level
    }

    pub fn get_transition_matrix(&self) -> &Array2<f64> {
        &self.transition_matrix
    }

    fn state_to_index(&self, state: MarketState) -> usize {
        match state {
            MarketState::TrendingUp => 0,
            MarketState::TrendingDown => 1,
            MarketState::Ranging => 2,
        }
    }

    pub fn get_statistics(&self) -> MarkovStatistics {
        let total_samples = self.state_history.len();
        let mut state_distribution = HashMap::new();
        
        for &state in &self.state_history {
            *state_distribution.entry(state).or_insert(0) += 1;
        }
        
        // Convert counts to percentages
        let mut state_percentages = HashMap::new();
        for (state, count) in state_distribution {
            state_percentages.insert(state, count as f64 / total_samples as f64 * 100.0);
        }
        
        MarkovStatistics {
            total_samples,
            state_distribution: state_percentages,
            confidence_level: self.confidence_level,
            transition_matrix: self.transition_matrix.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarketStatePrediction {
    pub current_state: MarketState,
    pub predicted_state: MarketState,
    pub probabilities: HashMap<MarketState, f64>,
    pub confidence: f64,
    pub sample_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkovStatistics {
    pub total_samples: usize,
    pub state_distribution: HashMap<MarketState, f64>,
    pub confidence_level: f64,
    pub transition_matrix: Array2<f64>,
}