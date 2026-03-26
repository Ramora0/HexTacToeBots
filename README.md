# HexTacToeBots

A framework for building and battling bots that play Hex Tic-Tac-Toe.

## The Game

Two players take turns placing stones on an **infinite hexagonal grid**. First to get **6 in a row** along any hex axis wins.

**Turn structure:**
- Player A places **1 stone** on the very first turn (to offset first-move advantage)
- After that, players alternate placing **2 stones** per turn

That's it. No captures, no special moves, just line up 6.

## Quick Start

```bash
make setup                                    # create venv + install deps
make build                                    # build C++ bots (SealBot, etc.)
make run A=SealBot B=random_bot               # run a match
make run A=SealBot B=random_bot N=100 T=0.5   # 100 games, 500ms per move
make list                                     # see all available bots
```

Or run the evaluator directly for full control:

```bash
python evaluate.py SealBot random_bot -n 100 -t 0.5          # 500ms per move
python evaluate.py SealBot random_bot --random-positions      # start mid-game
python evaluate.py --help                                     # all options
```

## Adding Your Bot

**1. Scaffold it:**

```bash
make new BOT=MyBot
```

This creates `bots/MyBot/bot.py` from the example template. Or do it manually:

**1. Create a folder:** `bots/MyBot/`

**2. Add `bot.py` with this structure:**

```python
class MyBot:
    def __init__(self, time_limit=0.05):
        self.time_limit = time_limit
        self.last_depth = 0   # search depth you reached (0 if N/A)
        self._nodes = 0       # nodes searched (0 if N/A)

    def get_move(self, game):
        # Return a list of 1 or 2 moves: [(q, r)] or [(q1, r1), (q2, r2)]
        return [(0, 0)]

    def __str__(self):
        return "MyBot"

def create_bot(time_limit=0.05):
    return MyBot(time_limit)
```

**3. Run it:**

```bash
make run A=MyBot B=random_bot
```

### What `get_move` receives

Your bot's `get_move(game)` receives a `HexGame` object with:

| Attribute               | Type                              | Description                           |
|--------------------------|-----------------------------------|---------------------------------------|
| `game.board`            | `dict[(q,r) -> Player.A\|B]`     | All occupied cells                    |
| `game.current_player`   | `Player.A` or `Player.B`         | Whose turn it is (your bot)           |
| `game.moves_left_in_turn` | `1` or `2`                     | Stones you must place this call       |
| `game.move_count`       | `int`                             | Total stones on the board             |
| `game.game_over`        | `bool`                            | Whether the game has ended            |
| `game.is_valid_move(q, r)` | `bool`                         | Check if a cell is empty and playable |

### What `get_move` must return

A **list** of `(q, r)` tuples -- one per stone you're placing:

```python
# You must return moves_left_in_turn moves:
return [(3, -1)]              # if moves_left_in_turn == 1
return [(3, -1), (4, -1)]    # if moves_left_in_turn == 2
```

If you return an invalid move (occupied cell), your bot **forfeits the game**.

### Time limit

The evaluator sets `bot.time_limit` before the match. Your `get_move` call must complete within `time_limit * num_moves` seconds. Occasional spikes are forgiven (3x grace factor), but 10 violations in one game = forfeit.

## The Coordinate System

The board uses **axial hex coordinates** `(q, r)`. Think of it like a skewed grid:

```
        (-1,-1) (0,-1) (1,-1)
       /       /      /
   (-1,0) --(0,0)-- (1,0)
     /       /      /
  (-1,1) (0,1)  (1,1)
```

Each hex cell has **6 neighbors:**

```
              (q, r-1)    (q+1, r-1)
                  \          /
        (q-1, r) -- (q, r) -- (q+1, r)
                  /          \
           (q-1, r+1)    (q, r+1)
```

**The three win axes** (directions you can line up 6):

| Axis      | Direction `(dq, dr)` | Looks like           |
|-----------|----------------------|----------------------|
| Horizontal | `(1, 0)`            | `---` east/west      |
| Diagonal  | `(0, 1)`             | `\` northwest/southeast |
| Anti-diag | `(1, -1)`            | `/` northeast/southwest |

**Key insight:** The board is infinite and sparse. `game.board` only contains occupied cells. There are no edges or boundaries -- you can place a stone at any unoccupied `(q, r)`.

## C++ Bots

For performance-critical bots, you can write the logic in C++ and use pybind11 to expose it to Python.

**1. Create your bot folder** with these files:

```
bots/MyCppBot/
    bot.py              # Python entry point
    my_engine.cpp       # Your C++ code with pybind11 bindings
    setup.py            # Build configuration
    __init__.py         # Empty file
    *.h                 # Any headers you need
```

**2. `bot.py`** -- thin wrapper that imports the compiled module:

```python
from bots.MyCppBot.my_engine_cpp import MyCppBot

def create_bot(time_limit=0.05):
    return MyCppBot(time_limit)
```

**3. `setup.py`:**

```python
from pybind11.setup_helpers import Pybind11Extension, build_ext
from setuptools import setup

setup(
    name="my_engine_cpp",
    ext_modules=[
        Pybind11Extension("my_engine_cpp", ["my_engine.cpp"],
                          cxx_std=17,
                          extra_compile_args=["-O3", "-march=native"],
                          include_dirs=["."]),
    ],
    cmdclass={"build_ext": build_ext},
)
```

**4. Build:**

```bash
make build    # builds all C++ bots
```

Or build one bot manually: `cd bots/MyCppBot && python setup.py build_ext --inplace`

Your compiled `.so` stays in the bot folder. See `examples/example.cpp` for the full pybind11 boilerplate, or look at `bots/SealBot/` for a real C++ bot.

## Project Structure

```
HexTacToeBots/
    Makefile             # make setup / build / run / new
    requirements.txt     # Python dependencies
    game.py              # Game rules -- don't modify
    evaluate.py          # Evaluation harness
    bots/
        random_bot/      # Baseline bot (plays randomly)
            bot.py
        SealBot/         # Example C++ bot with minimax search
            bot.py
            setup.py
            minimax_bot.cpp
            engine.h
            ...
    examples/
        example.py       # Copy to bots/YourBot/bot.py to get started
        example.cpp      # C++ bot boilerplate
```

## Evaluator Output

```
==================================================
  SealBot vs random_bot  --  20 games in 2.1s
==================================================
          SealBot:  19 wins (95%)
       random_bot:   1 wins (5%)
            Draws:   0      (0%)

  SealBot win rate: 95.0% (95% CI: 76.4%--99.7%)
  Elo difference: +458 (95% CI: +195 to +847)
  p-value (H0: equal strength): 4.5e-05 *

  SealBot search depth: avg 3.2, range [0-6]
  SealBot avg move time: 45ms (182 moves)
  random_bot avg move time: 0ms (178 moves)
==================================================
```

Games alternate sides (odd games = swapped colors) so results aren't biased by first-move advantage.
