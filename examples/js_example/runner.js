/**
 * JS bot runner harness for HexTacToeBots.
 *
 * This file handles communication with the Python framework over stdin/stdout.
 * You should NOT edit this file — edit my_bot.js instead.
 *
 * Protocol: JSON-lines on stdin/stdout. Use console.error() for debug logging.
 */

const readline = require("readline");
const path = require("path");

// Load the user's bot module (default: my_bot.js in the same directory)
const botPath = process.argv[2] || "./my_bot.js";
const bot = require(path.resolve(botPath));

if (typeof bot.getMove !== "function") {
  process.stderr.write(
    `Error: ${botPath} must export a getMove(gameState) function\n`
  );
  process.exit(1);
}

const rl = readline.createInterface({ input: process.stdin, terminal: false });

rl.on("line", (line) => {
  let msg;
  try {
    msg = JSON.parse(line);
  } catch (err) {
    respond({ type: "error", message: `Invalid JSON: ${err.message}` });
    return;
  }

  if (msg.type === "shutdown") {
    process.exit(0);
  }

  if (msg.type === "get_move") {
    try {
      const gs = msg.game_state;

      // Build a Map<"q,r", player> for O(1) board lookups
      const boardMap = new Map();
      for (const [q, r, p] of gs.board) {
        boardMap.set(`${q},${r}`, p);
      }

      const gameState = {
        board: gs.board,
        boardMap,
        currentPlayer: gs.current_player,
        movesLeftInTurn: gs.moves_left_in_turn,
        moveCount: gs.move_count,
        winLength: gs.win_length,
        timeLimit: msg.time_limit,
      };

      const result = bot.getMove(gameState);
      const resp = { type: "moves", moves: result };

      // Optional: forward search depth if the bot reports it
      if (typeof bot.lastDepth === "number") {
        resp.depth = bot.lastDepth;
      }

      respond(resp);
    } catch (err) {
      respond({ type: "error", message: err.message });
    }
  }
});

function respond(obj) {
  process.stdout.write(JSON.stringify(obj) + "\n");
}
