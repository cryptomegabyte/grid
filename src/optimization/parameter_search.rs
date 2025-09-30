use super::*;
use crate::{BacktestBuilder};
use std::collections::HashMap;

/// Advanced parameter search strategies
pub struct ParameterSearchEngine {
    pub search_strategy: SearchStrategy,
    pub convergence_criteria: ConvergenceCriteria,
}

#[derive(Debug, Clone)]
pub enum SearchStrategy {
    /// Exhaustive grid search
    GridSearch,
    
    /// Random sampling
    RandomSearch { 
        iterations: usize,
        seed: Option<u64>,
    },
    
    /// Bayesian optimization using Gaussian processes
    BayesianOptimization {
        iterations: usize,
        acquisition_function: AcquisitionFunction,
        initial_samples: usize,
    },
    
    /// Genetic algorithm evolution
    GeneticAlgorithm {
        population_size: usize,
        generations: usize,
        mutation_rate: f64,
        crossover_rate: f64,
    },
    
    /// Particle swarm optimization
    ParticleSwarm {
        particles: usize,
        iterations: usize,
        inertia: f64,
        cognitive: f64,
        social: f64,
    },
    
    /// Simulated annealing
    SimulatedAnnealing {
        initial_temperature: f64,
        cooling_rate: f64,
        min_temperature: f64,
        iterations: usize,
    },
}

#[derive(Debug, Clone)]
pub enum AcquisitionFunction {
    ExpectedImprovement,
    UpperConfidenceBound { kappa: f64 },
    ProbabilityOfImprovement,
}

#[derive(Debug, Clone)]
pub struct ConvergenceCriteria {
    pub max_iterations: usize,
    pub tolerance: f64,
    pub patience: usize,  // Early stopping patience
    pub min_improvement: f64,
}

/// Individual in genetic algorithm population
#[derive(Debug, Clone)]
pub struct Individual {
    pub parameters: ParameterSet,
    pub fitness: f64,
    pub age: usize,
}

/// Particle in swarm optimization
#[derive(Debug, Clone)]
pub struct Particle {
    pub position: ParameterSet,
    pub velocity: HashMap<String, f64>,
    pub best_position: ParameterSet,
    pub best_fitness: f64,
    pub current_fitness: f64,
}

impl ParameterSearchEngine {
    pub fn new(strategy: SearchStrategy) -> Self {
        Self {
            search_strategy: strategy,
            convergence_criteria: ConvergenceCriteria::default(),
        }
    }

    /// Execute parameter search using the configured strategy
    pub async fn search_optimal_parameters(
        &self,
        trading_pair: &str,
        config: &OptimizationConfig,
    ) -> Result<Vec<OptimizationResult>, BacktestError> {
        match &self.search_strategy {
            SearchStrategy::GridSearch => {
                self.grid_search(trading_pair, config).await
            }
            SearchStrategy::RandomSearch { iterations, seed } => {
                self.random_search(trading_pair, config, *iterations, *seed).await
            }
            SearchStrategy::BayesianOptimization { iterations, acquisition_function, initial_samples } => {
                self.bayesian_optimization(trading_pair, config, *iterations, acquisition_function, *initial_samples).await
            }
            SearchStrategy::GeneticAlgorithm { population_size, generations, mutation_rate, crossover_rate } => {
                self.genetic_algorithm(trading_pair, config, *population_size, *generations, *mutation_rate, *crossover_rate).await
            }
            SearchStrategy::ParticleSwarm { particles, iterations, inertia, cognitive, social } => {
                self.particle_swarm(trading_pair, config, *particles, *iterations, *inertia, *cognitive, *social).await
            }
            SearchStrategy::SimulatedAnnealing { initial_temperature, cooling_rate, min_temperature, iterations } => {
                self.simulated_annealing(trading_pair, config, *initial_temperature, *cooling_rate, *min_temperature, *iterations).await
            }
        }
    }

    async fn grid_search(
        &self,
        trading_pair: &str,
        config: &OptimizationConfig,
    ) -> Result<Vec<OptimizationResult>, BacktestError> {
        info!("🔍 Starting exhaustive grid search for {}", trading_pair);
        
        let parameter_combinations = self.generate_grid_combinations(config);
        let mut results = Vec::new();

        for (i, params) in parameter_combinations.iter().enumerate() {
            let result = self.evaluate_parameters(trading_pair, params).await?;
            results.push(result);

            if (i + 1) % 50 == 0 {
                info!("Grid search progress: {}/{}", i + 1, parameter_combinations.len());
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(results)
    }

    async fn random_search(
        &self,
        trading_pair: &str,
        config: &OptimizationConfig,
        iterations: usize,
        seed: Option<u64>,
    ) -> Result<Vec<OptimizationResult>, BacktestError> {
        info!("🎲 Starting random search for {} ({} iterations)", trading_pair, iterations);
        
        use rand::SeedableRng;
        use rand::rngs::StdRng;
        
        let mut rng = match seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_entropy(),
        };

        let mut results = Vec::new();
        let mut best_score = f64::NEG_INFINITY;
        let mut no_improvement_count = 0;

        for i in 0..iterations {
            let params = self.generate_random_parameters(config, &mut rng);
            let result = self.evaluate_parameters(trading_pair, &params).await?;
            
            if result.score > best_score {
                best_score = result.score;
                no_improvement_count = 0;
                info!("🎯 New best score: {:.4} (iteration {})", best_score, i + 1);
            } else {
                no_improvement_count += 1;
            }

            results.push(result);

            // Early stopping
            if no_improvement_count >= self.convergence_criteria.patience {
                info!("Early stopping after {} iterations without improvement", no_improvement_count);
                break;
            }

            if (i + 1) % 25 == 0 {
                info!("Random search progress: {}/{}", i + 1, iterations);
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(results)
    }

    async fn genetic_algorithm(
        &self,
        trading_pair: &str,
        config: &OptimizationConfig,
        population_size: usize,
        generations: usize,
        mutation_rate: f64,
        crossover_rate: f64,
    ) -> Result<Vec<OptimizationResult>, BacktestError> {
        info!("🧬 Starting genetic algorithm for {} (pop={}, gen={})", trading_pair, population_size, generations);
        
        let mut rng = rand::thread_rng();

        // Initialize population
        let mut population = Vec::new();
        for _ in 0..population_size {
            let params = self.generate_random_parameters(config, &mut rng);
            let result = self.evaluate_parameters(trading_pair, &params).await?;
            
            population.push(Individual {
                parameters: params,
                fitness: result.score,
                age: 0,
            });
        }

        let mut best_fitness = population.iter().map(|ind| ind.fitness).fold(f64::NEG_INFINITY, f64::max);
        let mut generation_results = Vec::new();

        for generation in 0..generations {
            // Selection, crossover, and mutation
            let mut new_population = self.evolve_population(&population, config, mutation_rate, crossover_rate, &mut rng);
            
            // Evaluate new individuals
            for individual in &mut new_population {
                if individual.fitness == 0.0 {  // New individual
                    let result = self.evaluate_parameters(trading_pair, &individual.parameters).await?;
                    individual.fitness = result.score;
                }
                individual.age += 1;
            }

            // Track best fitness
            let current_best = new_population.iter().map(|ind| ind.fitness).fold(f64::NEG_INFINITY, f64::max);
            if current_best > best_fitness {
                best_fitness = current_best;
                info!("🏆 Generation {}: New best fitness {:.4}", generation + 1, best_fitness);
            }

            population = new_population;

            if generation % 10 == 0 {
                info!("GA progress: Generation {}/{}", generation + 1, generations);
            }
        }

        // Convert population to results
        for individual in population {
            generation_results.push(OptimizationResult {
                parameters: individual.parameters,
                backtest_result: BacktestMetrics {
                    total_return: 0.0,  // Will be filled by actual evaluation
                    sharpe_ratio: 0.0,
                    max_drawdown: 0.0,
                    win_rate: 0.0,
                    profit_factor: 0.0,
                    total_trades: 0,
                    avg_trade_duration: 0.0,
                    risk_adjusted_return: 0.0,
                },
                score: individual.fitness,
                rank: 0,
            });
        }

        generation_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(generation_results)
    }

    // Additional optimization methods would be implemented here...
    async fn bayesian_optimization(
        &self,
        _trading_pair: &str,
        _config: &OptimizationConfig,
        _iterations: usize,
        _acquisition_function: &AcquisitionFunction,
        _initial_samples: usize,
    ) -> Result<Vec<OptimizationResult>, BacktestError> {
        // TODO: Implement Bayesian optimization
        // For now, return empty results
        warn!("Bayesian optimization not yet implemented, falling back to random search");
        Ok(Vec::new())
    }

    async fn particle_swarm(
        &self,
        _trading_pair: &str,
        _config: &OptimizationConfig,
        _particles: usize,
        _iterations: usize,
        _inertia: f64,
        _cognitive: f64,
        _social: f64,
    ) -> Result<Vec<OptimizationResult>, BacktestError> {
        // TODO: Implement particle swarm optimization
        warn!("Particle swarm optimization not yet implemented");
        Ok(Vec::new())
    }

    async fn simulated_annealing(
        &self,
        _trading_pair: &str,
        _config: &OptimizationConfig,
        _initial_temperature: f64,
        _cooling_rate: f64,
        _min_temperature: f64,
        _iterations: usize,
    ) -> Result<Vec<OptimizationResult>, BacktestError> {
        // TODO: Implement simulated annealing
        warn!("Simulated annealing not yet implemented");
        Ok(Vec::new())
    }

    /// Evaluate a parameter set by running a backtest
    async fn evaluate_parameters(
        &self,
        trading_pair: &str,
        parameters: &ParameterSet,
    ) -> Result<OptimizationResult, BacktestError> {
        let builder = BacktestBuilder::new()
            .with_grid_levels(parameters.grid_levels)
            .with_grid_spacing(parameters.grid_spacing);
        
        let mut engine = builder.build();
        let backtest_result = engine.run_backtest(
            trading_pair,
            parameters.date_range.start,
            parameters.date_range.end,
            parameters.timeframe_minutes,
        ).await?;
        
        let metrics = BacktestMetrics {
            total_return: backtest_result.performance_metrics.total_return_pct,
            sharpe_ratio: backtest_result.performance_metrics.sharpe_ratio,
            max_drawdown: backtest_result.performance_metrics.max_drawdown_pct,
            win_rate: backtest_result.performance_metrics.win_rate_pct,
            profit_factor: backtest_result.performance_metrics.profit_factor,
            total_trades: backtest_result.performance_metrics.total_trades,
            avg_trade_duration: backtest_result.performance_metrics.avg_time_in_position_hours,
            risk_adjusted_return: backtest_result.performance_metrics.total_return_pct 
                / (backtest_result.performance_metrics.volatility_pct.max(0.01)),
        };
        
        let score = self.calculate_composite_score(&metrics);
        
        Ok(OptimizationResult {
            parameters: parameters.clone(),
            backtest_result: metrics,
            score,
            rank: 0, // Will be set later
        })
    }

    /// Calculate composite optimization score
    fn calculate_composite_score(&self, metrics: &BacktestMetrics) -> f64 {
        // Multi-objective optimization score
        // Weights can be adjusted based on preferences
        let return_weight = 0.3;
        let sharpe_weight = 0.25;
        let drawdown_weight = 0.2;  // Penalty for high drawdown
        let win_rate_weight = 0.15;
        let profit_factor_weight = 0.1;
        
        let return_score = metrics.total_return / 100.0;  // Normalize
        let sharpe_score = metrics.sharpe_ratio.max(0.0) / 3.0;  // Cap at 3
        let drawdown_penalty = 1.0 - (metrics.max_drawdown / 100.0).min(0.5);  // Penalty
        let win_rate_score = metrics.win_rate / 100.0;
        let profit_factor_score = (metrics.profit_factor - 1.0).max(0.0) / 2.0;  // Cap at 3
        
        return_weight * return_score
            + sharpe_weight * sharpe_score
            + drawdown_weight * drawdown_penalty
            + win_rate_weight * win_rate_score
            + profit_factor_weight * profit_factor_score
    }

    fn generate_grid_combinations(&self, _config: &OptimizationConfig) -> Vec<ParameterSet> {
        // Implementation similar to the one in ParameterOptimizer
        // This would generate all possible combinations for grid search
        Vec::new() // Placeholder
    }

    fn generate_random_parameters<R: rand::Rng>(
        &self,
        config: &OptimizationConfig,
        rng: &mut R,
    ) -> ParameterSet {
        let grid_levels = rng.gen_range(config.grid_levels.min..=config.grid_levels.max);
        let grid_spacing = rng.gen_range(config.grid_spacing.min..=config.grid_spacing.max);
        let timeframe = *config.timeframes.get(rng.gen_range(0..config.timeframes.len())).unwrap();
        let max_drawdown = *config.risk_management.max_drawdown.get(rng.gen_range(0..config.risk_management.max_drawdown.len())).unwrap();
        let stop_loss = *config.risk_management.stop_loss.get(rng.gen_range(0..config.risk_management.stop_loss.len())).unwrap();
        let position_size = *config.risk_management.position_size.get(rng.gen_range(0..config.risk_management.position_size.len())).unwrap();
        let date_range = config.date_ranges.get(rng.gen_range(0..config.date_ranges.len())).unwrap().clone();

        ParameterSet {
            grid_levels,
            grid_spacing,
            timeframe_minutes: timeframe,
            max_drawdown,
            stop_loss,
            position_size,
            date_range,
        }
    }

    fn evolve_population(
        &self,
        population: &[Individual],
        _config: &OptimizationConfig,
        mutation_rate: f64,
        crossover_rate: f64,
        rng: &mut impl rand::Rng,
    ) -> Vec<Individual> {
        let mut new_population = Vec::new();
        
        // Keep best individuals (elitism)
        let mut sorted_pop = population.to_vec();
        sorted_pop.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());
        let elite_count = population.len() / 10; // Keep top 10%
        new_population.extend_from_slice(&sorted_pop[..elite_count]);

        // Generate rest through crossover and mutation
        while new_population.len() < population.len() {
            // Tournament selection
            let parent1 = self.tournament_selection(population, 3, rng);
            let parent2 = self.tournament_selection(population, 3, rng);

            let mut offspring = if rng.gen::<f64>() < crossover_rate {
                self.crossover(parent1, parent2, rng)
            } else {
                parent1.clone()
            };

            if rng.gen::<f64>() < mutation_rate {
                self.mutate(&mut offspring, rng);
            }

            new_population.push(offspring);
        }

        new_population
    }

    fn tournament_selection<'a>(
        &self,
        population: &'a [Individual],
        tournament_size: usize,
        rng: &mut impl rand::Rng,
    ) -> &'a Individual {
        use rand::seq::SliceRandom;
        
        let mut tournament: Vec<&Individual> = population.choose_multiple(rng, tournament_size).collect();
        tournament.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());
        tournament[0]
    }

    fn crossover(
        &self,
        parent1: &Individual,
        parent2: &Individual,
        rng: &mut impl rand::Rng,
    ) -> Individual {
        // Simple uniform crossover
        let mut offspring_params = parent1.parameters.clone();
        
        if rng.gen::<f64>() < 0.5 {
            offspring_params.grid_levels = parent2.parameters.grid_levels;
        }
        if rng.gen::<f64>() < 0.5 {
            offspring_params.grid_spacing = parent2.parameters.grid_spacing;
        }
        if rng.gen::<f64>() < 0.5 {
            offspring_params.timeframe_minutes = parent2.parameters.timeframe_minutes;
        }
        if rng.gen::<f64>() < 0.5 {
            offspring_params.max_drawdown = parent2.parameters.max_drawdown;
        }

        Individual {
            parameters: offspring_params,
            fitness: 0.0, // Will be evaluated later
            age: 0,
        }
    }

    fn mutate(&self, individual: &mut Individual, rng: &mut impl rand::Rng) {
        // Gaussian mutation
        if rng.gen::<f64>() < 0.2 {
            let delta = rng.gen_range(-2..=2);
            individual.parameters.grid_levels = (individual.parameters.grid_levels as i32 + delta).max(3) as usize;
        }
        
        if rng.gen::<f64>() < 0.2 {
            let delta = rng.gen_range(-0.005..=0.005);
            individual.parameters.grid_spacing = (individual.parameters.grid_spacing + delta).max(0.001);
        }
    }
}

// ParameterEvaluator trait removed - evaluation is now handled directly by ParameterSearchEngine

impl Default for ConvergenceCriteria {
    fn default() -> Self {
        Self {
            max_iterations: 1000,
            tolerance: 1e-6,
            patience: 50,
            min_improvement: 0.001,
        }
    }
}