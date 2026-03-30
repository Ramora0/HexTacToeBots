use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Cell {
    X = 1,
    O = 2,
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cell::X => write!(f, "X"),
            Cell::O => write!(f, "O"),
        }
    }
}

pub struct Board {
    pub moves: FxHashSet<(u8, i32, i32)>,
    pub turn_placed: FxHashMap<(i32, i32), u32>,
    pub game_history: String,
    pub window: (i32, i32, i32, i32), // (max_q, min_q, max_r, min_r)
    pub turn: u32,
    pub winner: Option<Cell>,
}

impl Board {
    pub fn new() -> Self {
        let mut b = Board {
            moves: FxHashSet::default(),
            turn_placed: FxHashMap::default(),
            game_history: String::new(),
            window: (6, -6, 6, -6),
            turn: 2,
            winner: None,
        };
        b.moves.insert((Cell::X as u8, 0, 0));
        b.turn_placed.insert((0, 0), 1);
        b
    }

    pub fn reset(&mut self) {
        self.moves.clear();
        self.moves.insert((Cell::X as u8, 0, 0));
        self.turn_placed.clear();
        self.turn_placed.insert((0, 0), 1);
        self.game_history.clear();
        self.window = (6, -6, 6, -6);
        self.turn = 2;
        self.winner = None;
    }

    pub fn current_player(&self) -> Cell {
        if self.turn % 2 == 0 {
            Cell::O
        } else {
            Cell::X
        }
    }

    pub fn check_win(&mut self) -> bool {
        let player = if self.current_player() == Cell::O {
            Cell::X
        } else {
            Cell::O
        };
        let pval = player as u8;

        let positions: FxHashSet<(i32, i32)> = self
            .moves
            .iter()
            .filter(|(p, _, _)| *p == pval)
            .map(|(_, q, r)| (*q, *r))
            .collect();

        let directions = [(1i32, -1i32), (0, 1), (1, 0)];
        for &(q, r) in &positions {
            for &(dq, dr) in &directions {
                if positions.contains(&(q - dq, r - dr)) {
                    continue;
                }
                let mut count = 1i32;
                while positions.contains(&(q + dq * count, r + dr * count)) {
                    count += 1;
                }
                if count >= 6 {
                    self.winner = Some(player);
                    return true;
                }
            }
        }
        false
    }

    pub fn play(&mut self, m: [(i32, i32); 2]) -> Result<bool, String> {
        if self.winner.is_some() {
            return Err("Game is already over".to_string());
        }
        if m[0] == m[1] {
            return Err("You must provide two different moves".to_string());
        }
        let player = self.current_player();
        let pval = player as u8;
        let other = if pval == 1 { 2u8 } else { 1u8 };

        for &(q, r) in &m {
            if self.moves.contains(&(pval, q, r)) || self.moves.contains(&(other, q, r)) {
                return Err("Cell is already occupied".to_string());
            }
        }

        for &(q, r) in &m {
            self.moves.insert((pval, q, r));
            self.turn_placed.insert((q, r), self.turn);
            self.window = (
                self.window.0.max(q + 6),
                self.window.1.min(q - 6),
                self.window.2.max(r + 6),
                self.window.3.min(r - 6),
            );
        }

        self.game_history.push_str(&format!(
            "{}. [{},{}][{},{}]; ",
            self.turn - 1,
            m[0].0,
            m[0].1,
            m[1].0,
            m[1].1
        ));
        self.turn += 1;
        Ok(self.check_win())
    }

    pub fn export(&self) -> String {
        self.game_history.trim_end().to_string()
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (max_q, min_q, max_r, min_r) = self.window;
        let width = (max_q - min_q) as usize;

        // Initial indent for the first row (matches Python: board = " " * width)
        let mut output = " ".repeat(width);
        let mut indent = 1usize;
        let mut i = max_q;
        while i >= min_q {
            for j in min_r..=max_r {
                if self.moves.contains(&(Cell::X as u8, i, j)) {
                    output.push_str("X ");
                } else if self.moves.contains(&(Cell::O as u8, i, j)) {
                    output.push_str("O ");
                } else {
                    output.push_str(". ");
                }
            }
            output.push('\n');
            output.push_str(&" ".repeat(width.saturating_sub(indent)));
            indent += 1;
            i -= 1;
        }
        write!(f, "{}", output)
    }
}
