"""Evaluate bots by playing them against each other.

Usage:
    python evaluate.py bot_a bot_b [-n NUM_GAMES] [--time-limit SECONDS]

Bots are loaded from the bots/ directory. Each bot needs a bot.py with either:
  - A get_move(game) function (simplest), or
  - A create_bot(time_limit) factory (for class-based / C++ bots)

get_move can be a plain function or a generator. Generators yield progressively
better moves; the framework takes the last result before time runs out.
"""

import argparse
import importlib
import importlib.util
import math
import os
import random
import sys
import time
from collections import defaultdict
from multiprocessing import Pool
from tqdm import tqdm

from game import HexGame, Player


# ── Constants ──
GRACE_FACTOR = 3.0
MAX_VIOLATIONS_PER_GAME = 10
MAX_MOVES_PER_GAME = 200


# ── Bot wrapper ──

class BotRunner:
    """Uniform wrapper for function-based and class-based bots.

    Handles generator-based iterative deepening: if get_move is a generator,
    it is iterated until the time budget is exhausted and the last yielded
    result is used.  Plain return values are passed through unchanged.
    """

    def __init__(self, name, get_move_fn, time_limit, bot_obj=None):
        self.name = name
        self._get_move = get_move_fn
        self._bot = bot_obj          # underlying class instance, if any
        self.time_limit = time_limit
        self._last_depth = 0

    @property
    def last_depth(self):
        if self._bot is not None and hasattr(self._bot, 'last_depth'):
            return self._bot.last_depth
        return self._last_depth

    def get_move(self, game):
        # Sync time limit to class-based bots that manage their own time
        if self._bot is not None and hasattr(self._bot, 'time_limit'):
            self._bot.time_limit = self.time_limit

        deadline = time.time() + self.time_limit * game.moves_left_in_turn
        result = self._get_move(game)

        # Generator: iterate until deadline, keep last result
        if hasattr(result, '__next__'):
            best = None
            depth = 0
            for moves in result:
                best = moves
                depth += 1
                if time.time() >= deadline:
                    break
            result.close()
            self._last_depth = depth
            return best if best is not None else []

        self._last_depth = 0
        return result

    def __str__(self):
        return self.name


# ── Statistics ──

def _win_rate_stats(wins, losses, draws):
    n = wins + losses + draws
    if n == 0:
        return 0.5, 0.0, 1.0, 1.0, 0, 0, 0
    score = wins + 0.5 * draws
    p_hat = score / n
    z = 1.96
    z2 = z * z
    denom = 1 + z2 / n
    centre = (p_hat + z2 / (2 * n)) / denom
    spread = z * math.sqrt((p_hat * (1 - p_hat) + z2 / (4 * n)) / n) / denom
    ci_lo = max(0.0, centre - spread)
    ci_hi = min(1.0, centre + spread)
    if n > 0:
        z_obs = (score - 0.5 * n) / math.sqrt(0.25 * n)
        p_value = 2 * _norm_sf(abs(z_obs))
    else:
        p_value = 1.0
    elo_diff = _score_to_elo(p_hat)
    elo_lo = _score_to_elo(ci_lo)
    elo_hi = _score_to_elo(ci_hi)
    return p_hat, ci_lo, ci_hi, p_value, elo_diff, elo_lo, elo_hi


def _score_to_elo(score):
    if score <= 0.0:
        return float('-inf')
    if score >= 1.0:
        return float('inf')
    return -400 * math.log10(1.0 / score - 1.0)


def _norm_sf(x):
    t = 1.0 / (1.0 + 0.2316419 * x)
    poly = t * (0.319381530 + t * (-0.356563782 + t * (1.781477937
            + t * (-1.821255978 + t * 1.330274429))))
    return poly * math.exp(-0.5 * x * x) / math.sqrt(2 * math.pi)


# ── Exceptions ──

class TimeLimitExceeded(Exception):
    def __init__(self, bot, violations):
        self.bot = bot
        self.violations = violations
        super().__init__(f"{bot} exceeded time limit {violations} times")


# ── Game helpers ──

def _generate_random_positions(num_positions, num_random_moves=10, win_length=6, seed=42):
    """Generate starting positions by playing random moves from empty board."""
    from bots.random_bot.bot import get_move as random_get_move
    random.seed(seed)
    positions = []
    for _ in range(num_positions):
        game = HexGame(win_length=win_length)
        for _ in range(num_random_moves):
            if game.game_over:
                break
            moves = random_get_move(game)
            for q, r in moves:
                if game.game_over:
                    break
                game.make_move(q, r)
        if not game.game_over:
            positions.append((dict(game.board), game.current_player))
    return positions


def _setup_game_from_position(board_dict, current_player, win_length=6):
    game = HexGame(win_length=win_length)
    game.board = dict(board_dict)
    game.current_player = current_player
    game.move_count = len(board_dict)
    game.moves_left_in_turn = 2
    return game


# ── Core game loop ──

def play_game(bot_a, bot_b, win_length=6, violations=None, max_moves=None,
              start_position=None):
    if max_moves is None:
        max_moves = MAX_MOVES_PER_GAME
    if start_position is not None:
        board_dict, current_player = start_position
        game = _setup_game_from_position(board_dict, current_player, win_length)
    else:
        game = HexGame(win_length=win_length)

    bots = {Player.A: bot_a, Player.B: bot_b}
    depths = {Player.A: defaultdict(int), Player.B: defaultdict(int)}
    times = {Player.A: [0.0, 0], Player.B: [0.0, 0]}
    total_moves = 0

    while not game.game_over:
        player = game.current_player
        bot = bots[player]

        t0 = time.time()
        moves = bot.get_move(game)
        elapsed = time.time() - t0

        if not moves:
            # Bot produced no moves — forfeit
            return (Player.B if player == Player.A else Player.A,
                    depths[Player.A], depths[Player.B],
                    tuple(times[Player.A]), tuple(times[Player.B]))

        num_moves = len(moves)
        times[player][0] += elapsed
        times[player][1] += num_moves

        allowed_time = bot.time_limit * num_moves
        if elapsed > allowed_time * GRACE_FACTOR:
            if violations is not None:
                violations[bot] = violations.get(bot, 0) + 1
                if violations[bot] >= MAX_VIOLATIONS_PER_GAME:
                    raise TimeLimitExceeded(bot, violations[bot])

        depths[player][bot.last_depth] += num_moves
        total_moves += num_moves

        if total_moves >= max_moves:
            return (Player.NONE, depths[Player.A], depths[Player.B],
                    tuple(times[Player.A]), tuple(times[Player.B]))

        invalid = False
        for q, r in moves:
            if game.game_over or not game.make_move(q, r):
                invalid = True
                break
        if invalid:
            return (Player.B if player == Player.A else Player.A,
                    depths[Player.A], depths[Player.B],
                    tuple(times[Player.A]), tuple(times[Player.B]))

    return (game.winner, depths[Player.A], depths[Player.B],
            tuple(times[Player.A]), tuple(times[Player.B]))


def _play_one(args):
    name_a, name_b, time_limit, game_idx, win_length, max_moves, start_position = args
    swapped = game_idx % 2 == 1

    # Create fresh bot instances in-process (avoids pickling C++ objects)
    bot_a = _create_bot(name_a, time_limit)
    bot_b = _create_bot(name_b, time_limit)

    if swapped:
        seat_a, seat_b = bot_b, bot_a
    else:
        seat_a, seat_b = bot_a, bot_b

    violations = {}
    exceeded = False
    try:
        winner, d_a, d_b, t_a, t_b = play_game(
            seat_a, seat_b, win_length, violations, max_moves,
            start_position=start_position)
    except TimeLimitExceeded:
        exceeded = True
        winner = Player.NONE
        d_a, d_b = defaultdict(int), defaultdict(int)
        t_a, t_b = (0.0, 0), (0.0, 0)

    move_count = t_a[1] + t_b[1]
    return (winner, swapped, dict(d_a), dict(d_b),
            violations.get(seat_a, 0), violations.get(seat_b, 0),
            exceeded, t_a, t_b, move_count)


def _create_bot(name, time_limit):
    """Create a BotRunner by name (used by worker processes)."""
    return _load_bot_from_module(name, time_limit)


# ── Evaluate ──

def evaluate(name_a, name_b, num_games=100, win_length=6, time_limit=0.1,
             use_tqdm=True, max_moves=None, positions=None):
    if max_moves is None:
        max_moves = MAX_MOVES_PER_GAME

    # Create bots in main process for validation / display names
    bot_a = load_bot(name_a, time_limit)
    bot_b = load_bot(name_b, time_limit)

    bot_a_wins = 0
    bot_b_wins = 0
    draws = 0
    games_played = 0
    bot_a_depths = defaultdict(int)
    bot_b_depths = defaultdict(int)
    bot_a_violations = 0
    bot_b_violations = 0
    aborted_games = 0
    bot_a_time = [0.0, 0]
    bot_b_time = [0.0, 0]
    game_lengths = []

    workers = os.cpu_count() or 1
    if positions is not None:
        num_games = len(positions) * 2
        args = []
        for i, pos in enumerate(positions):
            args.append((name_a, name_b, time_limit, i * 2,     win_length, max_moves, pos))
            args.append((name_a, name_b, time_limit, i * 2 + 1, win_length, max_moves, pos))
    else:
        args = [(name_a, name_b, time_limit, i, win_length, max_moves, None) for i in range(num_games)]

    t0 = time.time()
    with Pool(workers) as pool:
        results_iter = pool.imap_unordered(_play_one, args)
        if use_tqdm:
            results_iter = tqdm(results_iter, total=num_games, desc="Games", unit="game")
        for result in results_iter:
            winner, swapped, d_a, d_b, v_a, v_b, exceeded, t_a, t_b, move_count = result

            if exceeded:
                aborted_games += 1
            else:
                game_lengths.append(move_count)

            if swapped:
                for d, c in d_a.items(): bot_b_depths[d] += c
                for d, c in d_b.items(): bot_a_depths[d] += c
                bot_b_violations += v_a
                bot_a_violations += v_b
                bot_b_time[0] += t_a[0]; bot_b_time[1] += t_a[1]
                bot_a_time[0] += t_b[0]; bot_a_time[1] += t_b[1]
                if winner == Player.A:     bot_b_wins += 1
                elif winner == Player.B:   bot_a_wins += 1
                else:                      draws += 1
            else:
                for d, c in d_a.items(): bot_a_depths[d] += c
                for d, c in d_b.items(): bot_b_depths[d] += c
                bot_a_violations += v_a
                bot_b_violations += v_b
                bot_a_time[0] += t_a[0]; bot_a_time[1] += t_a[1]
                bot_b_time[0] += t_b[0]; bot_b_time[1] += t_b[1]
                if winner == Player.A:     bot_a_wins += 1
                elif winner == Player.B:   bot_b_wins += 1
                else:                      draws += 1

            games_played += 1
            if use_tqdm:
                results_iter.set_postfix(A=bot_a_wins, B=bot_b_wins, D=draws)

    elapsed = time.time() - t0
    total = max(games_played, 1)

    # ── Report ──
    print(f"\n\n{'='*50}")
    print(f"  {bot_a} vs {bot_b}  \u2014  {games_played} games in {elapsed:.1f}s")
    print(f"{'='*50}")
    na, nb = str(bot_a), str(bot_b)
    print(f"  {na:>15s}: {bot_a_wins:3d} wins ({100*bot_a_wins/total:.0f}%)")
    print(f"  {nb:>15s}: {bot_b_wins:3d} wins ({100*bot_b_wins/total:.0f}%)")
    print(f"  {'Draws':>15s}: {draws:3d}      ({100*draws/total:.0f}%)")

    win_rate, ci_lo, ci_hi, p_value, elo_diff, elo_lo, elo_hi = _win_rate_stats(bot_a_wins, bot_b_wins, draws)
    print(f"\n  {na} win rate: {100*win_rate:.1f}% "
          f"(95% CI: {100*ci_lo:.1f}%\u2013{100*ci_hi:.1f}%)")
    def _fmt_elo(e):
        return ("+\u221e" if e > 0 else "-\u221e") if math.isinf(e) else f"{e:+.0f}"
    print(f"  Elo difference: {_fmt_elo(elo_diff)} "
          f"(95% CI: {_fmt_elo(elo_lo)} to {_fmt_elo(elo_hi)})")
    if p_value < 0.001:
        p_str = f"{p_value:.1e}"
    else:
        p_str = f"{p_value:.3f}"
    sig = "*" if p_value < 0.05 else ""
    print(f"  p-value (H\u2080: equal strength): {p_str} {sig}")
    print()

    for name, depths in [(na, bot_a_depths), (nb, bot_b_depths)]:
        if not depths:
            continue
        total_moves = sum(depths.values())
        avg = sum(d * c for d, c in depths.items()) / total_moves
        lo, hi = min(depths), max(depths)
        print(f"  {name} search depth: avg {avg:.1f}, range [{lo}-{hi}]")
        buckets = sorted(depths.items())
        dist = "  ".join(f"d{d}:{c}" for d, c in buckets)
        print(f"    {dist}")

    for name, bt in [(na, bot_a_time), (nb, bot_b_time)]:
        if bt[1] > 0:
            avg_ms = 1000 * bt[0] / bt[1]
            print(f"  {name} avg move time: {avg_ms:.0f}ms ({bt[1]} moves)")

    if game_lengths:
        avg_len = sum(game_lengths) / len(game_lengths)
        lo_len, hi_len = min(game_lengths), max(game_lengths)
        print(f"\n  Game length: avg {avg_len:.1f} moves, range [{lo_len}-{hi_len}]")

    if bot_a_violations or bot_b_violations or aborted_games:
        print()
        print(f"  TIME VIOLATIONS: {na}={bot_a_violations}, {nb}={bot_b_violations}"
              f"  ({aborted_games} games forfeited)")

    print(f"{'='*50}")
    return bot_a_wins, bot_b_wins, draws


# ── Bot loading ──

def _import_bot_module(name):
    """Import bots/<name>/bot.py, with or without __init__.py."""
    try:
        return importlib.import_module(f"bots.{name}.bot")
    except ModuleNotFoundError:
        pass
    # Fallback: load by file path (no __init__.py needed)
    bots_dir = os.path.join(os.path.dirname(__file__), "bots")
    bot_file = os.path.join(bots_dir, name, "bot.py")
    if not os.path.isfile(bot_file):
        return None
    spec = importlib.util.spec_from_file_location(f"bots.{name}.bot", bot_file)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def _load_bot_from_module(name, time_limit):
    """Load a bot module and return a BotRunner."""
    mod = _import_bot_module(name)
    if mod is None:
        return None

    # Pattern 1: create_bot() factory (class-based / C++ bots)
    if hasattr(mod, 'create_bot'):
        bot_obj = mod.create_bot(time_limit)
        if not hasattr(bot_obj, 'get_move'):
            return None
        return BotRunner(name, bot_obj.get_move, time_limit, bot_obj=bot_obj)

    # Pattern 2: bare get_move() function
    if hasattr(mod, 'get_move'):
        return BotRunner(name, mod.get_move, time_limit)

    return None


def load_bot(name, time_limit=0.05):
    """Load a bot by name from the bots/ directory.

    Supports two patterns:
      - A bare get_move(game) function (simplest)
      - A create_bot(time_limit) factory (for class-based / C++ bots)
    """
    bot = _load_bot_from_module(name, time_limit)
    if bot is None:
        print(f"Error: bot '{name}' not found or invalid.")
        print(f"  Expected bots/{name}/bot.py with a get_move(game) function")
        print(f"  or a create_bot(time_limit) factory.")
        print(f"Available bots: {', '.join(list_bots())}")
        sys.exit(1)
    return bot


def list_bots():
    """List available bot names from the bots/ directory."""
    bots_dir = os.path.join(os.path.dirname(__file__), "bots")
    names = []
    for d in sorted(os.listdir(bots_dir)):
        bot_file = os.path.join(bots_dir, d, "bot.py")
        if os.path.isdir(os.path.join(bots_dir, d)) and os.path.isfile(bot_file):
            names.append(d)
    return names


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Evaluate two bots against each other.",
        epilog=f"Available bots: {', '.join(list_bots())}")
    parser.add_argument("bot_a", nargs="?", help="Bot name (from bots/ directory)")
    parser.add_argument("bot_b", nargs="?", help="Bot name (from bots/ directory)")
    parser.add_argument("-n", "--num-games", type=int, default=20,
                        help="Number of games (default: 20)")
    parser.add_argument("-t", "--time-limit", type=float, default=0.1,
                        help="Time limit per move in seconds (default: 0.1)")
    parser.add_argument("--no-tqdm", action="store_true",
                        help="Disable progress bar")
    parser.add_argument("--random-positions", action="store_true",
                        help="Start from random positions instead of empty board")
    parser.add_argument("--random-moves", type=int, default=10,
                        help="Moves per random starting position (default: 10)")
    parser.add_argument("--list", action="store_true",
                        help="List available bots and exit")
    parsed = parser.parse_args()

    if parsed.list:
        print("Available bots:")
        for name in list_bots():
            print(f"  {name}")
        sys.exit(0)

    if not parsed.bot_a or not parsed.bot_b:
        parser.error("bot_a and bot_b are required (use --list to see available bots)")

    # Validate bots exist before starting
    load_bot(parsed.bot_a, time_limit=parsed.time_limit)
    load_bot(parsed.bot_b, time_limit=parsed.time_limit)

    positions = None
    if parsed.random_positions:
        positions = _generate_random_positions(
            parsed.num_games, num_random_moves=parsed.random_moves)

    evaluate(parsed.bot_a, parsed.bot_b, num_games=parsed.num_games, positions=positions,
             time_limit=parsed.time_limit, use_tqdm=not parsed.no_tqdm)
