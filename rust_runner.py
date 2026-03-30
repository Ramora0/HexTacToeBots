"""Bridge for running Rust bots as subprocesses.

Usage in a bot's bot.py:

    from rust_runner import RustBotWrapper

    def create_bot(time_limit=0.05):
        return RustBotWrapper(
            binary_path=os.path.join(_DIR, "target", "release", "httt"),
            time_limit=time_limit,
        )

The Rust bot must support --bot mode which uses a line-based protocol:
  - Startup prints "READY"
  - After each command prints "READY"
  - "go <ms>" prints "MOVE <depth> q1 r1 q2 r2" before READY
  - "move q1 r1 q2 r2" sends opponent moves
  - "reset" clears the board
Use stderr for debug logging (stdout is reserved for the protocol).
"""

import os
import subprocess
import sys


class RustBotWrapper:
    """Runs a Rust bot binary in a long-lived subprocess.

    The subprocess is started lazily on the first get_move() call.
    Handles coordinate translation so the engine always sees the first
    stone at (0, 0), regardless of where Player A actually placed it.
    """

    def __init__(self, binary_path, time_limit=0.05):
        self.binary_path = os.path.abspath(binary_path)
        self.time_limit = time_limit
        self.last_depth = 0
        self._proc = None
        self._offset = (0, 0)
        self._known_stones = set()  # (q, r, player_value) in framework coords
        self._initialized = False

    # ── Coordinate translation ──

    def _to_engine(self, q, r):
        return (q + self._offset[0], r + self._offset[1])

    def _from_engine(self, q, r):
        return (q - self._offset[0], r - self._offset[1])

    # ── Subprocess management ──

    def _ensure_started(self):
        if self._proc is not None and self._proc.poll() is None:
            return
        self._proc = subprocess.Popen(
            [self.binary_path, "--bot"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        self._read_until_ready()
        self._initialized = False
        self._known_stones = set()
        self._offset = (0, 0)

    def _send(self, cmd):
        try:
            self._proc.stdin.write(cmd + "\n")
            self._proc.stdin.flush()
        except (BrokenPipeError, OSError) as exc:
            print(f"[rust_runner] send error: {exc}", file=sys.stderr)
            self._kill()

    def _read_until_ready(self):
        """Read lines until we see READY. Returns all lines before it."""
        lines = []
        try:
            while True:
                line = self._proc.stdout.readline()
                if not line:
                    break
                line = line.rstrip("\n")
                if line == "READY":
                    break
                lines.append(line)
        except (BrokenPipeError, OSError) as exc:
            print(f"[rust_runner] read error: {exc}", file=sys.stderr)
            self._kill()
        return lines

    # ── Bot interface ──

    def get_move(self, game):
        self._ensure_started()
        if self._proc is None:
            return []

        current_board = {pos: p.value for pos, p in game.board.items()}

        if not self._initialized:
            if not current_board:
                # We are Player A, first move. Engine already has X at (0,0).
                self._offset = (0, 0)
                self._known_stones = {(0, 0, game.current_player.value)}
                self._initialized = True
                return [(0, 0)]
            else:
                # We are Player B. Opponent placed 1 stone.
                opos = next(iter(current_board))
                oq, or_ = opos
                self._offset = (-oq, -or_)
                self._known_stones = {(oq, or_, current_board[opos])}
                self._initialized = True
                return self._go(game)

        # Find new opponent stones since last call
        new_stones = []
        for pos, pval in current_board.items():
            if (pos[0], pos[1], pval) not in self._known_stones:
                new_stones.append(pos)

        if len(new_stones) == 2:
            eq1, er1 = self._to_engine(*new_stones[0])
            eq2, er2 = self._to_engine(*new_stones[1])
            self._send(f"move {eq1} {er1} {eq2} {er2}")
            self._read_until_ready()
            for pos in new_stones:
                self._known_stones.add((pos[0], pos[1], current_board[pos]))

        return self._go(game)

    def _go(self, game):
        time_ms = int(self.time_limit * 1000)
        self._send(f"go {time_ms}")
        lines = self._read_until_ready()
        for line in lines:
            if line.startswith("MOVE "):
                # Format: MOVE <depth> <q1> <r1> <q2> <r2>
                parts = line.split()
                self.last_depth = int(parts[1])
                eq1, er1 = int(parts[2]), int(parts[3])
                eq2, er2 = int(parts[4]), int(parts[5])
                fq1, fr1 = self._from_engine(eq1, er1)
                fq2, fr2 = self._from_engine(eq2, er2)
                self._known_stones.add((fq1, fr1, game.current_player.value))
                self._known_stones.add((fq2, fr2, game.current_player.value))
                return [(fq1, fr1), (fq2, fr2)]
        return []

    # ── Lifecycle ──

    def shutdown(self):
        if self._proc is None or self._proc.poll() is not None:
            self._proc = None
            return
        try:
            self._proc.stdin.write("quit\n")
            self._proc.stdin.flush()
            self._proc.wait(timeout=2)
        except Exception:
            self._kill()
        self._proc = None

    def _kill(self):
        if self._proc is not None:
            try:
                self._proc.kill()
                self._proc.wait(timeout=1)
            except Exception:
                pass
            self._proc = None

    def __del__(self):
        self.shutdown()

    def __str__(self):
        return "RustBot"
