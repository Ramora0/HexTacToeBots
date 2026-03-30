"""Hexfish — alpha-beta minimax engine in Rust."""

import os
import platform
from rust_runner import RustBotWrapper

_DIR = os.path.dirname(os.path.abspath(__file__))
_BIN = "httt.exe" if platform.system() == "Windows" else "httt"


def create_bot(time_limit=0.05):
    return RustBotWrapper(
        binary_path=os.path.join(_DIR, "target", "release", _BIN),
        time_limit=time_limit,
    )
