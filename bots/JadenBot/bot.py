"""JadenBot — MCTS + greedy search ported from hex-tic-tac-toe (TypeScript)."""

import os
from js_runner import JsBotWrapper

_DIR = os.path.dirname(os.path.abspath(__file__))


def create_bot(time_limit=0.05):
    return JsBotWrapper(
        script_path=os.path.join(_DIR, "runner.ts"),
        time_limit=time_limit,
        cmd=["npx", "tsx"],
    )
