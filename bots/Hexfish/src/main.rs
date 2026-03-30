mod board;
mod engine;

use std::env;
use std::io::{self, BufRead, Write};
use std::time::Instant;

use engine::Engine;

fn main() {
    let bot_mode = env::args().any(|a| a == "--bot");

    let weights = [1.0, 5.0, 100.0, 5000.0, 1000.0];
    let mut engine = Engine::new(weights);
    let mut current_depth: i32 = 100;

    if bot_mode {
        println!("READY");
        io::stdout().flush().unwrap();
    } else {
        println!("Initializing Hexagonal Tic Tac Toe Engine...");
        println!("Commands:");
        println!("  show                     - Display the current board");
        println!("  depth <value>            - Set the AI search depth (e.g., depth 3)");
        println!("  move <q1> <r1> <q2> <r2> - Play a manual turn (provide 2 coordinate pairs)");
        println!("  go (or press Enter)      - Let the engine calculate and play the best move");
        println!("  export                   - Export the game notation");
        println!("  import <notation>        - Import a game from notation");
        println!("  reset                    - Reset the board to a new game");
        println!("  quit                     - Exit the CLI");
        println!("\nBoard coordinates follow the axial system [q, r].");
        print!("{}", engine);
    }

    let stdin = io::stdin();
    loop {
        if !bot_mode {
            print!("[Turn {}] > ", engine.board.turn);
        }
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            if !bot_mode {
                println!("Exiting...");
            }
            break;
        }
        let user_input: Vec<&str> = line.trim().split_whitespace().collect();

        let cmd = if user_input.is_empty() {
            if bot_mode { continue; } else { "go".to_string() }
        } else {
            user_input[0].to_lowercase()
        };

        match cmd.as_str() {
            "quit" | "exit" => {
                if !bot_mode {
                    println!("Exiting...");
                }
                break;
            }

            "show" => {
                print!("{}", engine);
            }

            "depth" => {
                if user_input.len() < 2 {
                    println!("Current depth is {}", current_depth);
                } else {
                    match user_input[1].parse::<i32>() {
                        Ok(d) => {
                            current_depth = d;
                            if !bot_mode {
                                println!("Engine search depth set to {}", current_depth);
                            }
                        }
                        Err(_) => println!("Invalid depth value"),
                    }
                }
            }

            "move" => {
                if user_input.len() != 5 {
                    println!("Usage: move <q1> <r1> <q2> <r2>");
                    if bot_mode { println!("READY"); io::stdout().flush().unwrap(); }
                    continue;
                }
                let coords: Vec<i32> = user_input[1..5]
                    .iter()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if coords.len() != 4 {
                    println!("Value Error: invalid coordinates");
                    if bot_mode { println!("READY"); io::stdout().flush().unwrap(); }
                    continue;
                }
                let m = [(coords[0], coords[1]), (coords[2], coords[3])];
                match engine.play(m) {
                    Ok(is_win) => {
                        if !bot_mode {
                            print!("{}", engine);
                        }
                        if is_win {
                            println!(
                                "Game Over! The winner is {}",
                                engine.board.winner.unwrap()
                            );
                        }
                    }
                    Err(e) => println!("Value Error: {}", e),
                }
            }

            "go" | "engine" => {
                if engine.board.winner.is_some() {
                    println!("Game is already over!");
                    if bot_mode { println!("READY"); io::stdout().flush().unwrap(); }
                    continue;
                }

                // Parse optional time budget: "go 200" = 200ms
                let time_budget_ms = if user_input.len() >= 2 {
                    user_input[1].parse::<u64>().ok()
                } else {
                    None
                };

                let t = Instant::now();
                if !bot_mode {
                    println!("Engine is thinking... (depth={})", current_depth);
                }
                let (best_move, depth_reached) = engine.get_best_move(current_depth, time_budget_ms);
                let elapsed = t.elapsed().as_secs_f64();

                match best_move {
                    None => println!("Engine could not find a valid move."),
                    Some(mv) => {
                        let (q1, r1) = mv.0;
                        let (q2, r2) = mv.1;
                        if bot_mode {
                            println!("MOVE {} {} {} {} {}", depth_reached, q1, r1, q2, r2);
                        } else {
                            println!(
                                "Engine plays: [{}, {}] and [{}, {}] in {:.2} seconds (depth {})",
                                q1, r1, q2, r2, elapsed, depth_reached
                            );
                        }
                        match engine.play([(q1, r1), (q2, r2)]) {
                            Ok(is_win) => {
                                if !bot_mode {
                                    print!("{}", engine);
                                }
                                if is_win {
                                    println!(
                                        "Game Over! The winner is {}",
                                        engine.board.winner.unwrap()
                                    );
                                }
                            }
                            Err(e) => println!("Value Error: {}", e),
                        }
                    }
                }
            }

            "export" => {
                println!("\nHexagonal Tic Tac Toe Notation:");
                println!("---------------------------------");
                println!("{}", engine.export());
                println!("---------------------------------");
            }

            "import" => {
                if user_input.len() < 2 {
                    println!("Usage: import <notation>");
                    if bot_mode { println!("READY"); io::stdout().flush().unwrap(); }
                    continue;
                }
                let notation = user_input[1..].join(" ");
                engine.reset();
                let mut ok = true;
                'outer: for segment in notation.split(';') {
                    let segment = segment.trim();
                    if segment.is_empty() {
                        continue;
                    }
                    let parts: Vec<&str> = segment.splitn(2, '.').collect();
                    if parts.len() < 2 {
                        continue;
                    }
                    let moves_part = parts[1].trim();
                    let halves: Vec<&str> = moves_part.split("][").collect();
                    if halves.len() < 2 {
                        println!("Failed to import: malformed notation");
                        ok = false;
                        break 'outer;
                    }
                    let m1_str = halves[0].trim_matches(|c| c == '[' || c == ']' || c == ' ');
                    let m2_str = halves[1].trim_matches(|c| c == '[' || c == ']' || c == ' ');
                    let parse_pair = |s: &str| -> Option<(i32, i32)> {
                        let nums: Vec<i32> = s.split(',')
                            .filter_map(|x| x.trim().parse().ok())
                            .collect();
                        if nums.len() == 2 { Some((nums[0], nums[1])) } else { None }
                    };
                    match (parse_pair(m1_str), parse_pair(m2_str)) {
                        (Some((q1, r1)), Some((q2, r2))) => {
                            if let Err(e) = engine.play([(q1, r1), (q2, r2)]) {
                                println!("Failed to import: {}", e);
                                ok = false;
                                break 'outer;
                            }
                        }
                        _ => {
                            println!("Failed to import: could not parse coordinates");
                            ok = false;
                            break 'outer;
                        }
                    }
                }
                if ok {
                    if !bot_mode {
                        println!("Game imported successfully.");
                        print!("{}", engine);
                    }
                }
            }

            "reset" => {
                engine.reset();
                if !bot_mode {
                    println!("Game reset. Board is clear.");
                    print!("{}", engine);
                }
            }

            _ => {
                if !bot_mode {
                    println!(
                        "Unknown command. Type 'show', 'depth', 'move', 'go', 'export', 'import', 'reset', or 'quit'."
                    );
                }
            }
        }

        if bot_mode {
            println!("READY");
            io::stdout().flush().unwrap();
        }
    }
}
