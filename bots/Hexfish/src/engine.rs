use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt;
use std::sync::OnceLock;
use std::time::Instant;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::board::{Board, Cell};

const TT_SIZE: usize = 1_048_576; // 2^20
const TT_EXACT: u8 = 0;
const TT_LOWER: u8 = 1;
const TT_UPPER: u8 = 2;

const POW10: [i64; 10] = [
    1,
    10,
    100,
    1_000,
    10_000,
    100_000,
    1_000_000,
    10_000_000,
    100_000_000,
    1_000_000_000,
];
const WIN_DIRS: [(i32, i32); 3] = [(1, 0), (0, 1), (1, -1)];

const ROOT_TOP_N: usize = 25;
const ROOT_CRITICAL_N: usize = 10;
const NODE_TOP_N: usize = 20;
const NODE_CRITICAL_N: usize = 10;

const NODE_MAX_WIDTH: usize = 20;
const NODE_MIN_WIDTH: usize = 5;
const NODE_WIDTH_STEP: i32 = 1;
const ROOT_MAX_WIDTH: usize = 25;
const ROOT_MIN_WIDTH: usize = 15;
const ROOT_WIDTH_STEP: i32 = 0;

type C2 = (i32, i32);
type MovePair = (C2, C2);
type TTEntry = (u64, bool, i32, u8, f64, Option<MovePair>);

fn neighbor_offsets() -> &'static [(i32, i32)] {
    static OFFSETS: OnceLock<Vec<(i32, i32)>> = OnceLock::new();
    OFFSETS.get_or_init(|| {
        let mut v = Vec::with_capacity(24);
        for dq in -2i32..=2 {
            for dr in -2i32..=2 {
                if dq != 0 || dr != 0 {
                    v.push((dq, dr));
                }
            }
        }
        v
    })
}

pub struct Engine {
    pub board: Board,
    pub weights: [f64; 5],
    current_hash: u64,
    empty_cells_set: FxHashSet<C2>,
    frontier_counts: FxHashMap<C2, i32>,
    history_table: FxHashMap<MovePair, i64>,
    window_counts: FxHashMap<(i32, i32, i32, i32), [i32; 2]>,
    x_needs: [i32; 6],
    o_needs: [i32; 6],
    cell_scores: FxHashMap<C2, i64>,
    board_state: FxHashMap<C2, u8>,
    zobrist_table: FxHashMap<(u8, i32, i32), u64>,
    tt: Vec<Option<TTEntry>>,
    rng: SmallRng,
    // Time management
    deadline: Option<Instant>,
    nodes_searched: u64,
    timed_out: bool,
}

impl Engine {
    pub fn new(weights: [f64; 5]) -> Self {
        Self::with_name(weights)
    }

    pub fn with_name(weights: [f64; 5]) -> Self {
        let mut rng = SmallRng::from_entropy();
        let mut zobrist_table: FxHashMap<(u8, i32, i32), u64> = FxHashMap::default();
        // Pre-fill center piece (matching Python __init__)
        zobrist_table.insert((1, 0, 0), rng.gen::<u64>());

        let mut engine = Engine {
            board: Board::new(),
            weights,
            current_hash: 0,
            empty_cells_set: FxHashSet::default(),
            frontier_counts: FxHashMap::default(),
            history_table: FxHashMap::default(),
            window_counts: FxHashMap::default(),
            x_needs: [0; 6],
            o_needs: [0; 6],
            cell_scores: FxHashMap::default(),
            board_state: FxHashMap::default(),
            zobrist_table,
            tt: vec![None; TT_SIZE],
            rng,
            deadline: None,
            nodes_searched: 0,
            timed_out: false,
        };
        engine.init_incremental_state();
        engine
    }

    pub fn reset(&mut self) {
        self.board.reset();
        self.init_incremental_state();
    }

    // ------------------------------------------------------------------
    // State initialisation
    // ------------------------------------------------------------------

    fn init_incremental_state(&mut self) {
        self.current_hash = 0;
        self.empty_cells_set.clear();
        self.frontier_counts.clear();
        self.history_table.clear();
        self.window_counts.clear();
        self.x_needs = [0; 6];
        self.o_needs = [0; 6];
        self.cell_scores.clear();
        self.board_state.clear();

        let existing: Vec<(u8, i32, i32)> = self.board.moves.iter().cloned().collect();
        for move_ in existing {
            self.add_move(move_);
        }
    }

    fn zobrist_get(&mut self, key: (u8, i32, i32)) -> u64 {
        if let Some(&v) = self.zobrist_table.get(&key) {
            return v;
        }
        let v: u64 = self.rng.gen();
        self.zobrist_table.insert(key, v);
        v
    }

    // ------------------------------------------------------------------
    // Public play (wraps Board::play + incremental update)
    // ------------------------------------------------------------------

    pub fn play(&mut self, m: [(i32, i32); 2]) -> Result<bool, String> {
        let pval = self.board.current_player() as u8;
        let is_win = self.board.play(m)?;
        self.add_move((pval, m[0].0, m[0].1));
        self.add_move((pval, m[1].0, m[1].1));
        if self.x_needs[5] > 0 {
            self.board.winner = Some(Cell::X);
            return Ok(true);
        }
        if self.o_needs[5] > 0 {
            self.board.winner = Some(Cell::O);
            return Ok(true);
        }
        Ok(is_win)
    }

    // ------------------------------------------------------------------
    // Move application / removal
    // ------------------------------------------------------------------

    fn add_move(&mut self, move_: (u8, i32, i32)) {
        let (player, q, r) = move_;

        self.board_state.insert((q, r), player);
        let zh = self.zobrist_get(move_);
        self.current_hash ^= zh;
        self.empty_cells_set.remove(&(q, r));

        for &(dq, dr) in neighbor_offsets() {
            let (nq, nr) = (q + dq, r + dr);
            *self.frontier_counts.entry((nq, nr)).or_insert(0) += 1;
            if !self.board_state.contains_key(&(nq, nr)) {
                self.empty_cells_set.insert((nq, nr));
            }
        }

        let is_x = player == 1;
        for &(dq, dr) in &WIN_DIRS {
            for k in 0..6i32 {
                let key = (dq, dr, q - k * dq, r - k * dr);
                self.update_window_add(key, is_x);
            }
        }
    }

    fn update_window_add(&mut self, key: (i32, i32, i32, i32), is_x: bool) {
        // Ensure entry exists, read initial counts as owned copy
        let [x_c, o_c] = {
            let e = self.window_counts.entry(key).or_insert([0, 0]);
            [e[0], e[1]]
        };

        let old_val: i64 = if o_c == 0 && x_c > 0 {
            self.x_needs[(x_c - 1) as usize] -= 1;
            POW10[x_c as usize]
        } else if x_c == 0 && o_c > 0 {
            self.o_needs[(o_c - 1) as usize] -= 1;
            POW10[o_c as usize]
        } else {
            0
        };

        let (new_x_c, new_o_c) = {
            let counts = self.window_counts.get_mut(&key).unwrap();
            if is_x {
                counts[0] += 1;
            } else {
                counts[1] += 1;
            }
            (counts[0], counts[1])
        };

        let new_val: i64 = if new_o_c == 0 && new_x_c > 0 {
            self.x_needs[(new_x_c - 1) as usize] += 1;
            POW10[new_x_c as usize]
        } else if new_x_c == 0 && new_o_c > 0 {
            self.o_needs[(new_o_c - 1) as usize] += 1;
            POW10[new_o_c as usize]
        } else {
            0
        };

        let net = new_val - old_val;
        if net != 0 {
            let (dq, dr, sq, sr) = key;
            for i in 0..6i32 {
                let cell = (sq + i * dq, sr + i * dr);
                *self.cell_scores.entry(cell).or_insert(0) += net;
            }
        }
    }

    fn remove_move(&mut self, move_: (u8, i32, i32)) {
        let (player, q, r) = move_;

        self.board_state.remove(&(q, r));
        let zh = self.zobrist_get(move_);
        self.current_hash ^= zh;

        if self.frontier_counts.get(&(q, r)).copied().unwrap_or(0) > 0 {
            self.empty_cells_set.insert((q, r));
        }

        for &(dq, dr) in neighbor_offsets() {
            let (nq, nr) = (q + dq, r + dr);
            let is_zero = if let Some(count) = self.frontier_counts.get_mut(&(nq, nr)) {
                *count -= 1;
                *count == 0
            } else {
                false
            };
            if is_zero {
                self.frontier_counts.remove(&(nq, nr));
                self.empty_cells_set.remove(&(nq, nr));
            }
        }

        let is_x = player == 1;
        for &(dq, dr) in &WIN_DIRS {
            for k in 0..6i32 {
                let key = (dq, dr, q - k * dq, r - k * dr);
                self.update_window_remove(key, is_x);
            }
        }
    }

    fn update_window_remove(&mut self, key: (i32, i32, i32, i32), is_x: bool) {
        // Read initial counts — skip entirely if window not tracked
        let [x_c, o_c] = match self.window_counts.get(&key) {
            Some(c) => [c[0], c[1]],
            None => return,
        };

        let old_val: i64 = if o_c == 0 && x_c > 0 {
            self.x_needs[(x_c - 1) as usize] -= 1;
            POW10[x_c as usize]
        } else if x_c == 0 && o_c > 0 {
            self.o_needs[(o_c - 1) as usize] -= 1;
            POW10[o_c as usize]
        } else {
            0
        };

        let (new_x_c, new_o_c) = {
            let counts = self.window_counts.get_mut(&key).unwrap();
            if is_x {
                counts[0] -= 1;
            } else {
                counts[1] -= 1;
            }
            (counts[0], counts[1])
        };

        let new_val: i64 = if new_o_c == 0 && new_x_c > 0 {
            self.x_needs[(new_x_c - 1) as usize] += 1;
            POW10[new_x_c as usize]
        } else if new_x_c == 0 && new_o_c > 0 {
            self.o_needs[(new_o_c - 1) as usize] += 1;
            POW10[new_o_c as usize]
        } else {
            0
        };

        let net = new_val - old_val;
        if net != 0 {
            let (dq, dr, sq, sr) = key;
            for i in 0..6i32 {
                let cell = (sq + i * dq, sr + i * dr);
                *self.cell_scores.entry(cell).or_insert(0) += net;
            }
        }

        if new_x_c == 0 && new_o_c == 0 {
            self.window_counts.remove(&key);
        }
    }

    // ------------------------------------------------------------------
    // Evaluation
    // ------------------------------------------------------------------

    fn evaluate(&self) -> f64 {
        if self.x_needs[5] > 0 {
            return 1_000_000_000_000.0;
        }
        if self.o_needs[5] > 0 {
            return -1_000_000_000_000.0;
        }
        let [w1, w2, w3, w4, w5] = self.weights;
        let x_score = w1 * self.x_needs[0] as f64
            + w2 * self.x_needs[1] as f64
            + w3 * self.x_needs[2] as f64
            + w4 * self.x_needs[3] as f64
            + w5 * self.x_needs[4] as f64;
        let o_score = w1 * self.o_needs[0] as f64
            + w2 * self.o_needs[1] as f64
            + w3 * self.o_needs[2] as f64
            + w4 * self.o_needs[3] as f64
            + w5 * self.o_needs[4] as f64;
        x_score - o_score
    }

    // ------------------------------------------------------------------
    // Candidate move generation
    // ------------------------------------------------------------------

    fn get_top_cells(&self, n: usize) -> Vec<C2> {
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;

        let cell_scores = &self.cell_scores;
        // Min-heap capped at n: O(N log n) instead of O(N log N)
        let mut heap: BinaryHeap<Reverse<(i64, C2)>> = BinaryHeap::with_capacity(n + 1);

        for &cell in &self.empty_cells_set {
            let score = cell_scores.get(&cell).copied().unwrap_or(0);
            if heap.len() < n {
                heap.push(Reverse((score, cell)));
            } else if let Some(&Reverse((min_score, _))) = heap.peek() {
                if score > min_score {
                    heap.pop();
                    heap.push(Reverse((score, cell)));
                }
            }
        }

        let mut result: Vec<C2> = heap.into_iter().map(|Reverse((_, c))| c).collect();
        result.sort_unstable_by(|a, b| {
            let sa = cell_scores.get(a).copied().unwrap_or(0);
            let sb = cell_scores.get(b).copied().unwrap_or(0);
            sb.cmp(&sa)
        });
        result
    }

    fn candidate_moves(
        &mut self,
        top_n: usize,
        critical_n: usize,
    ) -> (Vec<MovePair>, FxHashMap<MovePair, f64>, bool) {
        let top_cells = self.get_top_cells(top_n);
        if top_cells.is_empty() {
            return (vec![], FxHashMap::default(), false);
        }

        let top_score = self.cell_scores.get(&top_cells[0]).copied().unwrap_or(0);
        let has_critical = top_score >= 10_000;

        if !has_critical {
            // Fast path: static combinations, no simulation
            let mut moves = Vec::new();
            let mut pair_scores: FxHashMap<MovePair, f64> = FxHashMap::default();
            for i in 0..top_cells.len() {
                for j in (i + 1)..top_cells.len() {
                    let (c1, c2) = (top_cells[i], top_cells[j]);
                    let s1 = self.cell_scores.get(&c1).copied().unwrap_or(0) as f64;
                    let s2 = self.cell_scores.get(&c2).copied().unwrap_or(0) as f64;
                    let key: MovePair = if c1 < c2 { (c1, c2) } else { (c2, c1) };
                    let score = if s1 >= s2 {
                        s1 + 0.01 * s2
                    } else {
                        s2 + 0.01 * s1
                    };
                    pair_scores.insert(key, score);
                    moves.push((c1, c2));
                }
            }
            return (moves, pair_scores, false);
        }

        // Critical path: simulate first move, re-score for second
        let critical_cells: Vec<C2> = top_cells
            .iter()
            .filter(|&&c| self.cell_scores.get(&c).copied().unwrap_or(0) >= 10_000)
            .take(critical_n)
            .cloned()
            .collect();

        let pval = self.board.current_player() as u8;
        let mut moves_set: FxHashSet<MovePair> = FxHashSet::default();
        let mut pair_scores: FxHashMap<MovePair, f64> = FxHashMap::default();

        for c1 in critical_cells {
            let s1 = self.cell_scores.get(&c1).copied().unwrap_or(0) as f64;
            let m1 = (pval, c1.0, c1.1);
            self.add_move(m1);

            let c2_candidates = self.get_top_cells(top_n);
            for c2 in c2_candidates {
                if c2 == c1 {
                    continue;
                }
                let s2 = self.cell_scores.get(&c2).copied().unwrap_or(0) as f64;
                let combined = if s1 >= s2 {
                    s1 + 0.01 * s2
                } else {
                    s2 + 0.01 * s1
                };
                let pair: MovePair = if c1 < c2 { (c1, c2) } else { (c2, c1) };
                let entry = pair_scores.entry(pair).or_insert(f64::NEG_INFINITY);
                if combined > *entry {
                    *entry = combined;
                }
                moves_set.insert(pair);
            }

            self.remove_move(m1);
        }

        (moves_set.into_iter().collect(), pair_scores, true)
    }

    fn sort_moves(
        moves: &mut Vec<MovePair>,
        pair_scores: &FxHashMap<MovePair, f64>,
        history_table: &FxHashMap<MovePair, i64>,
        preferred: Option<MovePair>,
    ) {
        // moves.sort_by(|a, b| {
        //     let sa = Self::move_score(a, pair_scores, history_table);
        //     let sb = Self::move_score(b, pair_scores, history_table);
        //     sb.partial_cmp(&sa)
        //         .unwrap_or(std::cmp::Ordering::Equal)
        //         .then_with(|| a.cmp(b))
        // });

        // Compute scores once
        let mut scored_moves: Vec<(f64, MovePair)> = moves
            .iter()
            .map(|&m| (Self::move_score(&m, pair_scores, history_table), m))
            .collect();
        // Sort by score descending
        scored_moves.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Extract back to moves
        *moves = scored_moves.into_iter().map(|(_, m)| m).collect();

        if let Some(pref) = preferred {
            let p_rev: MovePair = (pref.1, pref.0);
            if let Some(pos) = moves.iter().position(|&m| m == pref) {
                moves.remove(pos);
                moves.insert(0, pref);
            } else if let Some(pos) = moves.iter().position(|&m| m == p_rev) {
                moves.remove(pos);
                moves.insert(0, p_rev);
            }
        }
    }

    fn move_score(
        c: &MovePair,
        pair_scores: &FxHashMap<MovePair, f64>,
        history: &FxHashMap<MovePair, i64>,
    ) -> f64 {
        let hc: MovePair = if c.0 < c.1 { *c } else { (c.1, c.0) };
        pair_scores.get(&hc).copied().unwrap_or(0.0) + *history.get(&hc).unwrap_or(&0) as f64
    }

    // ------------------------------------------------------------------
    // Search
    // ------------------------------------------------------------------

    fn alphabeta(
        &mut self,
        depth: i32,
        mut alpha: f64,
        mut beta: f64,
        maximizing_player: bool,
    ) -> f64 {
        // Check time every 1024 nodes
        self.nodes_searched += 1;
        if self.nodes_searched & 1023 == 0 {
            if let Some(dl) = self.deadline {
                if Instant::now() >= dl {
                    self.timed_out = true;
                }
            }
        }
        if self.timed_out {
            return self.evaluate();
        }

        let alpha_orig = alpha;
        let beta_orig = beta;

        let tt_index = (self.current_hash as usize) % TT_SIZE;

        // Read TT entry as owned copies, releasing the borrow immediately
        let (tt_best_move, tt_hit, tt_stored_depth, tt_flag, tt_val) = match &self.tt[tt_index] {
            Some(e) if e.0 == self.current_hash && e.1 == maximizing_player => {
                (e.5, true, e.2, e.3, e.4)
            }
            _ => (None, false, 0i32, 0u8, 0.0f64),
        };

        if tt_hit && tt_stored_depth >= depth {
            match tt_flag {
                TT_EXACT => return tt_val,
                TT_LOWER => alpha = alpha.max(tt_val),
                TT_UPPER => beta = beta.min(tt_val),
                _ => {}
            }
            if alpha >= beta {
                return tt_val;
            }
        }

        let opponent_wins = if maximizing_player {
            self.o_needs[5] > 0
        } else {
            self.x_needs[5] > 0
        };

        if depth == 0 || opponent_wins {
            if opponent_wins {
                // Penalise later wins: higher depth remaining = win found sooner = better
                let score = 1_000_000_000_000.0 + depth as f64;
                return if maximizing_player { -score } else { score };
            }
            return self.evaluate();
        }

        let (mut moves, score_dict, has_critical) =
            self.candidate_moves(NODE_TOP_N, NODE_CRITICAL_N);
        // candidate_moves released &mut self; now safe to borrow history_table immutably
        Self::sort_moves(&mut moves, &score_dict, &self.history_table, tt_best_move);
        if !has_critical {
            let width =
                (NODE_MAX_WIDTH - ((depth - 1) * NODE_WIDTH_STEP) as usize).max(NODE_MIN_WIDTH);
            moves.truncate(width);
        }

        if moves.is_empty() {
            return self.evaluate();
        }

        let pval = self.board.current_player() as u8;
        let mut best_eval = if maximizing_player {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
        let mut best_node_move: Option<MovePair> = None;

        for (idx, mv) in moves.iter().enumerate() {
            let (c1, c2) = *mv; // Copy (MovePair is Copy)
            let m1 = (pval, c1.0, c1.1);
            let m2 = (pval, c2.0, c2.1);
            let saved_turn = self.board.turn;
            self.add_move(m1);
            self.add_move(m2);
            self.board.turn += 1;

            let ev = if idx == 0 {
                self.alphabeta(depth - 1, alpha, beta, !maximizing_player)
            } else if maximizing_player {
                let ev = self.alphabeta(depth - 1, alpha, alpha + 1.0, false);
                if alpha < ev && ev < beta {
                    self.alphabeta(depth - 1, alpha, beta, false)
                } else {
                    ev
                }
            } else {
                let ev = self.alphabeta(depth - 1, beta - 1.0, beta, true);
                if alpha < ev && ev < beta {
                    self.alphabeta(depth - 1, alpha, beta, true)
                } else {
                    ev
                }
            };

            self.remove_move(m1);
            self.remove_move(m2);
            self.board.turn = saved_turn;

            if maximizing_player {
                if ev > best_eval {
                    best_eval = ev;
                    best_node_move = Some((c1, c2));
                }
                alpha = alpha.max(ev);
            } else {
                if ev < best_eval {
                    best_eval = ev;
                    best_node_move = Some((c1, c2));
                }
                beta = beta.min(ev);
            }

            if beta <= alpha {
                let hc: MovePair = if c1 < c2 { (c1, c2) } else { (c2, c1) };
                *self.history_table.entry(hc).or_insert(0) += (depth * depth) as i64;
                break;
            }
        }

        let tt_flag_new = if best_eval <= alpha_orig {
            TT_UPPER
        } else if best_eval >= beta_orig {
            TT_LOWER
        } else {
            TT_EXACT
        };

        let should_store = match &self.tt[tt_index] {
            None => true,
            Some(e) => e.0 != self.current_hash || depth >= e.2,
        };
        if should_store {
            self.tt[tt_index] = Some((
                self.current_hash,
                maximizing_player,
                depth,
                tt_flag_new,
                best_eval,
                best_node_move,
            ));
        }

        best_eval
    }

    pub fn get_best_move(&mut self, depth: i32, time_budget_ms: Option<u64>) -> (Option<MovePair>, i32) {
        if self.history_table.len() > 200_000 {
            self.history_table.clear();
        } else {
            for val in self.history_table.values_mut() {
                *val >>= 1;
            }
        }

        // Set up time management
        self.nodes_searched = 0;
        self.timed_out = false;
        self.deadline = time_budget_ms.map(|ms| Instant::now() + std::time::Duration::from_millis(ms));

        let pval = self.board.current_player() as u8;
        let is_maximizing = pval == 1;
        let mut best_overall_move: Option<MovePair> = None;
        let mut depth_reached: i32 = 0;

        for d in 1..=depth {
            if self.timed_out {
                break;
            }

            let mut alpha = f64::NEG_INFINITY;
            let mut beta = f64::INFINITY;
            let mut best_eval = if is_maximizing {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };
            let mut best_move: Option<MovePair> = None;

            let (mut moves, score_dict, has_critical) =
                self.candidate_moves(ROOT_TOP_N, ROOT_CRITICAL_N);
            Self::sort_moves(
                &mut moves,
                &score_dict,
                &self.history_table,
                best_overall_move,
            );
            if !has_critical {
                let width =
                    (ROOT_MAX_WIDTH - ((d - 1) * ROOT_WIDTH_STEP) as usize).max(ROOT_MIN_WIDTH);
                moves.truncate(width);
            }

            if moves.is_empty() {
                return (None, depth_reached);
            }

            for (idx, mv) in moves.iter().enumerate() {
                let (c1, c2) = *mv;
                let m1 = (pval, c1.0, c1.1);
                let m2 = (pval, c2.0, c2.1);
                let saved_turn = self.board.turn;
                self.add_move(m1);
                self.add_move(m2);
                self.board.turn += 1;

                let ev = if idx == 0 {
                    self.alphabeta(d - 1, alpha, beta, !is_maximizing)
                } else if is_maximizing {
                    let ev = self.alphabeta(d - 1, alpha, alpha + 1.0, false);
                    if ev > alpha {
                        self.alphabeta(d - 1, alpha, beta, false)
                    } else {
                        ev
                    }
                } else {
                    let ev = self.alphabeta(d - 1, beta - 1.0, beta, true);
                    if ev < beta {
                        self.alphabeta(d - 1, alpha, beta, true)
                    } else {
                        ev
                    }
                };

                self.remove_move(m1);
                self.remove_move(m2);
                self.board.turn = saved_turn;

                if is_maximizing {
                    if ev > best_eval || best_move.is_none() {
                        best_eval = ev;
                        best_move = Some((c1, c2));
                    }
                    alpha = alpha.max(ev);
                } else {
                    if ev < best_eval || best_move.is_none() {
                        best_eval = ev;
                        best_move = Some((c1, c2));
                    }
                    beta = beta.min(ev);
                }

                if beta <= alpha {
                    break;
                }
            }

            // Only use this depth's result if it completed fully
            if self.timed_out {
                break;
            }
            if let Some(m) = best_move {
                best_overall_move = Some(m);
                depth_reached = d;
            }
        }

        self.deadline = None;
        (best_overall_move, depth_reached)
    }

    pub fn export(&self) -> String {
        self.board.export()
    }
}

impl fmt::Display for Engine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.board)
    }
}
