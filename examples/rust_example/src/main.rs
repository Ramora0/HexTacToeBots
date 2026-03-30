//! Minimal Rust bot template for HexTacToeBots.
//!
//! Protocol (--bot mode):
//!   - Print "READY" on startup and after every command.
//!   - Read commands from stdin, one per line:
//!       depth <N>              — set search depth
//!       move <q1> <r1> <q2> <r2> — opponent played these two stones
//!       go                     — compute and print your move
//!       reset                  — new game (X at 0,0, O to move)
//!       quit                   — exit
//!   - On "go", print "MOVE <q1> <r1> <q2> <r2>" before "READY".
//!
//! The board starts with X at (0,0). Turn order: O, X, O, X, …
//! Each turn places exactly 2 stones.
//!
//! Edit the `get_move` function below with your own logic.
//! Build: cargo build --release
//! The framework will run: target/release/my_rust_bot --bot

use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, Write};

fn get_move(board: &HashMap<(i32, i32), u8>, current_player: u8) -> [(i32, i32); 2] {
    // Gather candidate cells: empty cells within distance 2 of any stone
    let mut candidates: HashSet<(i32, i32)> = HashSet::new();
    for &(q, r) in board.keys() {
        for dq in -2..=2i32 {
            for dr in -2..=2i32 {
                if dq == 0 && dr == 0 {
                    continue;
                }
                let c = (q + dq, r + dr);
                if !board.contains_key(&c) {
                    candidates.insert(c);
                }
            }
        }
    }
    let mut rng = rand::thread_rng();
    let list: Vec<(i32, i32)> = candidates.into_iter().collect();
    let m1 = *list.choose(&mut rng).unwrap();
    let remaining: Vec<&(i32, i32)> = list.iter().filter(|&&c| c != m1).collect();
    let m2 = **remaining.choose(&mut rng).unwrap();
    [m1, m2]
}

fn main() {
    let bot_mode = std::env::args().any(|a| a == "--bot");

    // Board state: (q, r) -> player (1=X, 2=O)
    let mut board: HashMap<(i32, i32), u8> = HashMap::new();
    board.insert((0, 0), 1); // X at center
    let mut turn: u32 = 2; // O moves next

    if bot_mode {
        println!("READY");
        io::stdout().flush().unwrap();
    }

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            if bot_mode { continue; }
        }
        let cmd = parts[0].to_lowercase();

        match cmd.as_str() {
            "quit" | "exit" => break,

            "depth" => { /* ignore for this simple bot */ }

            "move" => {
                if parts.len() == 5 {
                    let q1: i32 = parts[1].parse().unwrap_or(0);
                    let r1: i32 = parts[2].parse().unwrap_or(0);
                    let q2: i32 = parts[3].parse().unwrap_or(0);
                    let r2: i32 = parts[4].parse().unwrap_or(0);
                    let p = if turn % 2 == 0 { 2u8 } else { 1u8 };
                    board.insert((q1, r1), p);
                    board.insert((q2, r2), p);
                    turn += 1;
                }
            }

            "go" => {
                let p = if turn % 2 == 0 { 2u8 } else { 1u8 };
                let [m1, m2] = get_move(&board, p);
                board.insert(m1, p);
                board.insert(m2, p);
                turn += 1;
                if bot_mode {
                    println!("MOVE {} {} {} {}", m1.0, m1.1, m2.0, m2.1);
                }
            }

            "reset" => {
                board.clear();
                board.insert((0, 0), 1);
                turn = 2;
            }

            _ => {}
        }

        if bot_mode {
            println!("READY");
            io::stdout().flush().unwrap();
        }
    }
}
