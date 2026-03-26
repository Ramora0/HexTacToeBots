"""Example Python bot for HexTacToeBots.

Quick start:
  1. Copy this folder into bots/:  cp -r examples/python_example bots/my_bot
  2. Run: python evaluate.py my_bot random_bot

All you need is a get_move(game) function that returns a list of (q, r) tuples.
Return 1 move when game.moves_left_in_turn == 1, otherwise 2.

For iterative deepening, make get_move a generator that yields progressively
better move lists. The framework takes the last result before time runs out:

    def get_move(game):
        yield quick_moves(game)    # depth 1
        yield better_moves(game)   # depth 2
        yield best_moves(game)     # depth 3

The game object has:
  game.board              -> dict: (q, r) -> Player.A or Player.B
  game.current_player     -> Player.A or Player.B (you)
  game.moves_left_in_turn -> 1 or 2
  game.move_count         -> total stones on board
  game.game_over          -> bool
  game.is_valid_move(q,r) -> bool
"""

import random


def get_move(game):
    # TODO: replace with your own logic
    return [(0, 0)] * game.moves_left_in_turn
