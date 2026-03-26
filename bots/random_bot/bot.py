"""Random bot — places stones randomly near existing stones."""

import random


def hex_distance(dq, dr):
    return max(abs(dq), abs(dr), abs(dq + dr))


_D2_OFFSETS = tuple(
    (dq, dr)
    for dq in range(-2, 3)
    for dr in range(-2, 3)
    if hex_distance(dq, dr) <= 2 and (dq, dr) != (0, 0)
)


def get_move(game):
    if not game.board:
        return [(0, 0)]

    candidates = set()
    for q, r in game.board:
        for dq, dr in _D2_OFFSETS:
            nb = (q + dq, r + dr)
            if nb not in game.board:
                candidates.add(nb)

    moves = []
    for _ in range(game.moves_left_in_turn):
        if not candidates:
            break
        move = random.choice(list(candidates))
        moves.append(move)
        candidates.discard(move)

    return moves
