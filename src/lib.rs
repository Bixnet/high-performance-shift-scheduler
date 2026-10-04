//! High-performance shift scheduling with tabu search.

use rand::{rngs::StdRng, Rng, SeedableRng};
use rayon::prelude::*;
use std::collections::VecDeque;

pub const UNASSIGNED: usize = usize::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    pub slot: usize,
    pub from: usize,
    pub to: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct ShiftSlot {
    pub day: usize,
    pub shift: u8,
}

#[derive(Clone, Debug)]
pub struct Staff {
    pub target_assignments: i32,
    pub max_nights: i32,
    pub unavailable_days: Vec<bool>,
}

#[derive(Clone, Debug)]
pub struct Problem {
    pub staff: Vec<Staff>,
    pub slots: Vec<ShiftSlot>,
}

impl Problem {
    pub fn synthetic(staff_count: usize, days: usize, seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let shift_count = days * 3;
        let target_total = shift_count.max(staff_count);
        let base_target = (target_total / staff_count.max(1)) as i32;
        let remainder = (target_total % staff_count.max(1)) as i32;

        let staff = (0..staff_count).map(|id| {
            let target = base_target + i32::from(id < remainder as usize);
            let unavailable_days = (0..days).map(|_| rng.gen_bool(0.08)).collect();
            Staff {
                target_assignments: target,
                max_nights: (target / 3).max(1),
                unavailable_days,
            }
        }).collect();

        let slots = (0..days)
            .flat_map(|day| (0..3).map(move |shift| ShiftSlot { day, shift }))
            .collect();

        Self { staff, slots }
    }
}

#[derive(Clone, Debug)]
pub struct Schedule {
    pub assignment: Vec<usize>,
}

impl Schedule {
    pub fn new(problem: &Problem) -> Self {
        Self { assignment: vec![UNASSIGNED; problem.slots.len()] }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScoreState {
    staff_scores: Vec<i64>,
    pub total: i64,
    assignment_counts: Vec<i32>,
    night_counts: Vec<i32>,
    unavailable_counts: Vec<i32>,
}

impl ScoreState {
    pub fn from_schedule(problem: &Problem, schedule: &Schedule) -> Self {
        let n = problem.staff.len();
        let mut state = Self {
            staff_scores: vec![0; n],
            total: 0,
            assignment_counts: vec![0; n],
            night_counts: vec![0; n],
            unavailable_counts: vec![0; n],
        };

        for (slot_idx, &staff_id) in schedule.assignment.iter().enumerate() {
            if staff_id != UNASSIGNED {
                state.apply_assignment(problem, slot_idx, staff_id, 1);
            }
        }
        state.total = state.staff_scores.iter().sum();
        state
    }

    fn staff_score(problem: &Problem, staff_id: usize, assignments: i32, nights: i32, unavailable: i32) -> i64 {
        let staff = &problem.staff[staff_id];
        let diff = assignments - staff.target_assignments;
        let load_penalty = i64::from(diff * diff) * 2;
        let night_over = (nights - staff.max_nights).max(0);
        let night_penalty = i64::from(night_over * night_over) * 5;
        let availability_penalty = i64::from(unavailable) * 100;
        load_penalty + night_penalty + availability_penalty
    }

    fn apply_assignment(&mut self, problem: &Problem, slot_idx: usize, staff_id: usize, direction: i32) {
        let old_score = self.staff_scores[staff_id];
        self.assignment_counts[staff_id] += direction;
        if problem.slots[slot_idx].shift == 2 {
            self.night_counts[staff_id] += direction;
        }
        if problem.staff[staff_id].unavailable_days[problem.slots[slot_idx].day] {
            self.unavailable_counts[staff_id] += direction;
        }
        let new_score = Self::staff_score(
            problem,
            staff_id,
            self.assignment_counts[staff_id],
            self.night_counts[staff_id],
            self.unavailable_counts[staff_id],
        );
        self.staff_scores[staff_id] = new_score;
        self.total += new_score - old_score;
    }

    pub fn apply_move(&mut self, problem: &Problem, schedule: &mut Schedule, mv: Move) {
        if mv.from != UNASSIGNED {
            self.apply_assignment(problem, mv.slot, mv.from, -1);
        }
        self.apply_assignment(problem, mv.slot, mv.to, 1);
        schedule.assignment[mv.slot] = mv.to;
    }

    pub fn recompute_total(&self) -> i64 {
        self.staff_scores.iter().sum()
    }
}

#[derive(Clone, Debug)]
pub struct TabuList {
    entries: VecDeque<Move>,
    capacity: usize,
}

impl TabuList {
    pub fn new(capacity: usize) -> Self {
        Self { entries: VecDeque::with_capacity(capacity), capacity: capacity.max(1) }
    }

    pub fn push(&mut self, mv: Move) {
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(mv);
    }
}

#[derive(Clone, Debug)]
pub struct SolverConfig {
    pub iterations: usize,
    pub candidates_per_slot: usize,
    pub tabu_capacity: usize,
    pub seed: u64,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self { iterations: 500, candidates_per_slot: 8, tabu_capacity: 256, seed: 42 }
    }
}

#[derive(Clone, Debug)]
pub struct SolveResult {
    pub schedule: Schedule,
    pub score: i64,
    pub iterations: usize,
}

pub struct Solver {
    config: SolverConfig,
}

impl Solver {
    pub fn new(config: SolverConfig) -> Self {
        Self { config }
    }

    pub fn solve(&self, problem: &Problem) -> SolveResult {
        let mut rng = StdRng::seed_from_u64(self.config.seed);
        let mut schedule = self.initial_schedule(problem, &mut rng);
        let mut state = ScoreState::from_schedule(problem, &schedule);
        let mut tabu = TabuList::new(self.config.tabu_capacity);
        let mut best_schedule = schedule.clone();
        let mut best_score = state.total;

        for iteration in 0..self.config.iterations {
            let candidate_moves = self.generate_candidates(problem, &schedule, &mut rng);
            if candidate_moves.is_empty() {
                break;
            }

            let tabu_snapshot = tabu.entries.iter().copied().collect::<Vec<_>>();
            let evaluated = candidate_moves.par_iter()
                .map(|&mv| {
                    let candidate_score = delta_score(problem, &state, mv);
                    let is_tabu = tabu_snapshot.iter().any(|x| x == &mv);
                    let aspiration = candidate_score < best_score;
                    (mv, candidate_score, is_tabu, aspiration)
                })
                .filter(|(_, _, is_tabu, aspiration)| !*is_tabu || *aspiration)
                .min_by_key(|(_, score, _, _)| *score);

            let Some((chosen, _, _, _)) = evaluated else {
                continue;
            };

            tabu.push(Move { slot: chosen.slot, from: chosen.to, to: chosen.from });
            state.apply_move(problem, &mut schedule, chosen);

            if state.total < best_score {
                best_score = state.total;
                best_schedule = schedule.clone();
            }

            if best_score == 0 {
                return SolveResult { schedule: best_schedule, score: best_score, iterations: iteration + 1 };
            }
        }

        SolveResult { schedule: best_schedule, score: best_score, iterations: self.config.iterations }
    }

    fn initial_schedule(&self, problem: &Problem, rng: &mut StdRng) -> Schedule {
        let mut schedule = Schedule::new(problem);
        let mut remaining = problem.staff.iter().map(|s| s.target_assignments).collect::<Vec<_>>();
        let mut slot_indices = (0..problem.slots.len()).collect::<Vec<_>>();

        for i in (1..slot_indices.len()).rev() {
            let j = rng.gen_range(0..=i);
            slot_indices.swap(i, j);
        }

        for slot in slot_indices {
            let shift = problem.slots[slot];
            let mut candidates = (0..problem.staff.len())
                .filter(|&staff_id| remaining[staff_id] > 0)
                .filter(|&staff_id| !problem.staff[staff_id].unavailable_days[shift.day])
                .collect::<Vec<_>>();

            if candidates.is_empty() {
                candidates = (0..problem.staff.len()).filter(|&id| remaining[id] > 0).collect();
            }
            if candidates.is_empty() {
                candidates = (0..problem.staff.len()).collect();
            }

            if let Some(&staff_id) = candidates.iter().min_by_key(|&&id| remaining[id]) {
                schedule.assignment[slot] = staff_id;
                remaining[staff_id] -= 1;
            }
        }
        schedule
    }

    fn generate_candidates(&self, problem: &Problem, schedule: &Schedule, rng: &mut StdRng) -> Vec<Move> {
        let mut moves = Vec::with_capacity(schedule.assignment.len() * self.config.candidates_per_slot);
        let staff_count = problem.staff.len();
        if staff_count == 0 {
            return moves;
        }

        for slot in 0..schedule.assignment.len() {
            let from = schedule.assignment[slot];
            for _ in 0..self.config.candidates_per_slot {
                let to = rng.gen_range(0..staff_count);
                if to != from {
                    moves.push(Move { slot, from, to });
                }
            }
        }
        moves
    }
}

fn delta_score(problem: &Problem, state: &ScoreState, mv: Move) -> i64 {
    let mut total = state.total;
    let touched = [mv.from, mv.to];

    for (idx, &staff_id) in touched.iter().enumerate() {
        if staff_id == UNASSIGNED || (idx == 1 && touched[0] == touched[1]) {
            continue;
        }

        let old = state.staff_scores[staff_id];
        let mut assignments = state.assignment_counts[staff_id];
        let mut nights = state.night_counts[staff_id];
        let mut unavailable = state.unavailable_counts[staff_id];
        let direction = if idx == 0 { -1 } else { 1 };

        assignments += direction;
        if problem.slots[mv.slot].shift == 2 {
            nights += direction;
        }
        if problem.staff[staff_id].unavailable_days[problem.slots[mv.slot].day] {
            unavailable += direction;
        }

        let new = ScoreState::staff_score(problem, staff_id, assignments, nights, unavailable);
        total += new - old;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incremental_score_matches_full_recompute() {
        let problem = Problem::synthetic(32, 14, 7);
        let solver = Solver::new(SolverConfig::default());
        let mut schedule = solver.initial_schedule(&problem, &mut StdRng::seed_from_u64(2));
        let mut state = ScoreState::from_schedule(&problem, &schedule);
        let mv = Move { slot: 0, from: schedule.assignment[0], to: 3 };
        state.apply_move(&problem, &mut schedule, mv);
        assert_eq!(state.total, state.recompute_total());
    }

    #[test]
    fn solver_returns_consistent_score() {
        let problem = Problem::synthetic(50, 14, 1);
        let solver = Solver::new(SolverConfig { iterations: 25, candidates_per_slot: 4, ..Default::default() });
        let result = solver.solve(&problem);
        let state = ScoreState::from_schedule(&problem, &result.schedule);
        assert_eq!(result.score, state.total);
    }

    #[test]
    fn tabu_is_bounded() {
        let mut tabu = TabuList::new(4);
        for i in 0..20 {
            tabu.push(Move { slot: i, from: 0, to: 1 });
        }
        assert_eq!(tabu.entries.len(), 4);
    }
}
