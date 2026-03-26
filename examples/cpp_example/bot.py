"""C++ bot wrapper.

Quick start:
  1. Copy this folder into bots/:  cp -r examples/cpp_example bots/my_bot
  2. Build: cd bots/my_bot && python setup.py build_ext --inplace
  3. Run: python evaluate.py my_bot random_bot
"""

from bots.cpp_example.my_bot_cpp import MyCppBot


def create_bot(time_limit=0.05):
    return MyCppBot(time_limit)
