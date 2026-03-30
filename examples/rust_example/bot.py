"""Minimal Rust bot — edit src/main.rs, then: cargo build --release"""

import os
import platform
from rust_runner import RustBotWrapper

_DIR = os.path.dirname(os.path.abspath(__file__))
_BIN = "my_rust_bot.exe" if platform.system() == "Windows" else "my_rust_bot"


def create_bot(time_limit=0.05):
    return RustBotWrapper(
        binary_path=os.path.join(_DIR, "target", "release", _BIN),
        time_limit=time_limit,
        depth=6,
    )
