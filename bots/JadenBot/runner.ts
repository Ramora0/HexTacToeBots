/**
 * TypeScript bot runner harness for HexTacToeBots.
 *
 * Handles communication with the Python framework over stdin/stdout.
 * Protocol: JSON-lines on stdin/stdout. Use console.error() for debug logging.
 */

import { createInterface } from "readline";
import { chooseBotTurnDetailed } from "./search";
import type { LiveLikeState, Player, BotSearchOptions } from "./types";
import { DEFAULT_BOT_TUNING, DEFAULT_BOT_SEARCH_OPTIONS } from "./types";

const rl = createInterface({ input: process.stdin, terminal: false });

function respond(obj: unknown) {
  process.stdout.write(JSON.stringify(obj) + "\n");
}

function playerFromInt(p: number): Player {
  return p === 1 ? "X" : "O";
}

rl.on("line", (line) => {
  let msg: any;
  try {
    msg = JSON.parse(line);
  } catch (err: any) {
    respond({ type: "error", message: `Invalid JSON: ${err.message}` });
    return;
  }

  if (msg.type === "shutdown") {
    process.exit(0);
  }

  if (msg.type === "get_move") {
    try {
      const gs = msg.game_state;

      // Build moves Map<"q,r", Player> from the board triples
      const moves = new Map<string, Player>();
      for (const [q, r, p] of gs.board) {
        moves.set(`${q},${r}`, playerFromInt(p));
      }

      const state: LiveLikeState = {
        moves,
        moveHistory: gs.board.map(([q, r, p]: [number, number, number]) => ({
          q,
          r,
          mark: playerFromInt(p),
        })),
        turn: playerFromInt(gs.current_player),
        placementsLeft: gs.moves_left_in_turn,
      };

      const timeLimitMs = (msg.time_limit || 0.05) * 1000;
      const searchOptions: Partial<BotSearchOptions> = {
        budget: {
          maxTimeMs: Math.max(30, timeLimitMs * 0.8),
          maxNodes: DEFAULT_BOT_SEARCH_OPTIONS.budget.maxNodes,
        },
      };

      const decision = chooseBotTurnDetailed(state, DEFAULT_BOT_TUNING, searchOptions);

      const moves_out = decision.moves.map((m) => [m.q, m.r]);
      respond({
        type: "moves",
        moves: moves_out,
        depth: decision.stats.maxDepthTurns,
      });
    } catch (err: any) {
      respond({ type: "error", message: err.message });
    }
  }
});
