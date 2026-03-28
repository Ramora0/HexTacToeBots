"""JavaScript bot wrapper — greedy threat-based example.

Quick start:
  1. Copy this folder into bots/:  cp -r examples/js_example bots/my_js_bot
  2. Run: python evaluate.py my_js_bot random_bot

Edit my_bot.js with your logic. runner.js handles communication with Python.
For TypeScript, rename to .ts and pass cmd=["npx", "tsx"] to JsBotWrapper.
"""

import os
from js_runner import JsBotWrapper

_DIR = os.path.dirname(os.path.abspath(__file__))


def create_bot(time_limit=0.05):
    return JsBotWrapper(
        script_path=os.path.join(_DIR, "runner.js"),
        time_limit=time_limit,
    )
