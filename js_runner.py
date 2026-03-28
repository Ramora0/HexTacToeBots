"""Bridge for running JavaScript/TypeScript bots as Node.js subprocesses.

Usage in a bot's bot.py:

    from js_runner import JsBotWrapper

    def create_bot(time_limit=0.05):
        return JsBotWrapper(
            script_path=os.path.join(_DIR, "runner.js"),
            time_limit=time_limit,
        )

The JS bot communicates via JSON-lines on stdin/stdout.
Use console.error() in JS for debug logging (stdout is reserved for the protocol).
"""

import json
import os
import subprocess
import sys


class JsBotWrapper:
    """Runs a JS bot in a long-lived Node.js subprocess.

    The subprocess is started lazily on the first get_move() call and
    terminated when the wrapper is garbage-collected or shutdown() is called.
    Each instance owns its own subprocess, so multiprocessing is safe.
    """

    def __init__(self, script_path, time_limit=0.05, cmd=None):
        self.script_path = os.path.abspath(script_path)
        self.time_limit = time_limit
        self.last_depth = 0
        self._cmd = cmd or ["node"]
        self._proc = None

    def _ensure_started(self):
        if self._proc is not None and self._proc.poll() is None:
            return
        self._proc = subprocess.Popen(
            self._cmd + [self.script_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
            cwd=os.path.dirname(self.script_path),
        )

    def get_move(self, game):
        self._ensure_started()

        # Serialize game state
        board = [[q, r, p.value] for (q, r), p in game.board.items()]
        msg = {
            "type": "get_move",
            "game_state": {
                "board": board,
                "current_player": game.current_player.value,
                "moves_left_in_turn": game.moves_left_in_turn,
                "move_count": game.move_count,
                "win_length": game.win_length,
            },
            "time_limit": self.time_limit,
        }

        try:
            self._proc.stdin.write(json.dumps(msg) + "\n")
            self._proc.stdin.flush()

            line = self._proc.stdout.readline()
            if not line:
                return []

            resp = json.loads(line)
            if resp.get("type") == "moves":
                self.last_depth = resp.get("depth", 0)
                return [tuple(m) for m in resp["moves"]]
            if resp.get("type") == "error":
                print(f"[js_runner] JS error: {resp.get('message')}", file=sys.stderr)
                return []

        except (BrokenPipeError, OSError, json.JSONDecodeError, ValueError) as exc:
            print(f"[js_runner] communication error: {exc}", file=sys.stderr)
            self._kill()

        return []

    def shutdown(self):
        if self._proc is None or self._proc.poll() is not None:
            self._proc = None
            return
        try:
            self._proc.stdin.write(json.dumps({"type": "shutdown"}) + "\n")
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
        return os.path.basename(os.path.dirname(self.script_path))
