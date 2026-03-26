"""SealBot — C++ iterative deepening alpha-beta minimax.

Build first:  cd bots/SealBot && python setup.py build_ext --inplace
"""

from bots.SealBot.minimax_cpp import MinimaxBot


def create_bot(time_limit=0.05):
    return MinimaxBot(time_limit)
