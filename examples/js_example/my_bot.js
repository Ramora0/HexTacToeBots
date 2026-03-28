/**
 * Example JavaScript bot — greedy 1-ply search with threat evaluation.
 *
 * Evaluates every candidate cell by scanning all 6-cell windows that pass
 * through it, scoring each window by how close it is to winning. Picks the
 * highest-scoring placement. Also checks for immediate wins and forced blocks.
 *
 * gameState has:
 *   .board            - Array of [q, r, player] triples (player: 1=A, 2=B)
 *   .boardMap         - Map<"q,r", player> for O(1) lookups
 *   .currentPlayer    - 1 or 2
 *   .movesLeftInTurn  - 1 or 2
 *   .moveCount        - total stones on the board
 *   .winLength        - 6 (stones in a row to win)
 *   .timeLimit        - seconds per move
 *
 * Return: Array of [q, r] pairs, e.g. [[3, -1]] or [[3, -1], [4, -1]]
 * Use console.error() for debug output (stdout is reserved for the protocol).
 */

const WIN_LENGTH = 6;
const WIN_DIRS = [
  [1, 0],
  [0, 1],
  [1, -1],
];

// Hex offsets within distance 2
const D2_OFFSETS = [];
for (let dq = -2; dq <= 2; dq++) {
  for (let dr = -2; dr <= 2; dr++) {
    const dist = Math.max(Math.abs(dq), Math.abs(dr), Math.abs(dq + dr));
    if (dist <= 2 && (dq !== 0 || dr !== 0)) D2_OFFSETS.push([dq, dr]);
  }
}

function key(q, r) {
  return `${q},${r}`;
}

/** Count consecutive friendly stones in one direction. */
function countDir(boardMap, q, r, dq, dr, player) {
  let n = 0;
  let cq = q + dq,
    cr = r + dr;
  while (boardMap.get(key(cq, cr)) === player) {
    n++;
    cq += dq;
    cr += dr;
  }
  return n;
}

/** Does placing here make WIN_LENGTH in a row? */
function isWin(boardMap, q, r, player) {
  for (const [dq, dr] of WIN_DIRS) {
    if (
      1 +
        countDir(boardMap, q, r, dq, dr, player) +
        countDir(boardMap, q, r, -dq, -dr, player) >=
      WIN_LENGTH
    )
      return true;
  }
  return false;
}

/**
 * Score how valuable placing player's stone at (q,r) would be.
 * Temporarily places the stone, then scans all windows of 6 through (q,r).
 * Each unblocked window (no opponent stones) contributes 5^(own_count).
 */
function windowScore(boardMap, q, r, player, opponent) {
  const k = key(q, r);
  boardMap.set(k, player);
  let total = 0;
  for (const [dq, dr] of WIN_DIRS) {
    for (let offset = 0; offset < WIN_LENGTH; offset++) {
      const sq = q - dq * offset;
      const sr = r - dr * offset;
      let own = 0;
      let blocked = false;
      for (let i = 0; i < WIN_LENGTH; i++) {
        const mark = boardMap.get(key(sq + dq * i, sr + dr * i));
        if (mark === player) own++;
        else if (mark === opponent) {
          blocked = true;
          break;
        }
      }
      if (!blocked) total += 5 ** own;
    }
  }
  boardMap.delete(k);
  return total;
}

/** Get empty cells within distance 2 of any stone. */
function getCandidates(boardMap, board) {
  if (board.length === 0) return [[0, 0]];
  const cands = [];
  const seen = new Set();
  for (const [q, r] of board) {
    for (const [dq, dr] of D2_OFFSETS) {
      const nq = q + dq,
        nr = r + dr;
      const k = key(nq, nr);
      if (!seen.has(k) && !boardMap.has(k)) {
        seen.add(k);
        cands.push([nq, nr]);
      }
    }
  }
  return cands;
}

function pickBest(boardMap, candidates, me, opp) {
  // 1. Check for immediate winning move
  for (let i = 0; i < candidates.length; i++) {
    const [q, r] = candidates[i];
    boardMap.set(key(q, r), me);
    const win = isWin(boardMap, q, r, me);
    boardMap.delete(key(q, r));
    if (win) return i;
  }

  // 2. Check for forced blocks (opponent wins next move)
  const blocks = [];
  for (let i = 0; i < candidates.length; i++) {
    const [q, r] = candidates[i];
    boardMap.set(key(q, r), opp);
    const threat = isWin(boardMap, q, r, opp);
    boardMap.delete(key(q, r));
    if (threat) blocks.push(i);
  }
  if (blocks.length > 0) {
    // Pick the block that also maximizes our own score
    let bestIdx = blocks[0];
    let bestScore = -Infinity;
    for (const i of blocks) {
      const [q, r] = candidates[i];
      const score = windowScore(boardMap, q, r, me, opp);
      if (score > bestScore) {
        bestScore = score;
        bestIdx = i;
      }
    }
    return bestIdx;
  }

  // 3. Score all candidates: attack + defense
  let bestIdx = 0;
  let bestScore = -Infinity;
  for (let i = 0; i < candidates.length; i++) {
    const [q, r] = candidates[i];
    const attack = windowScore(boardMap, q, r, me, opp);
    const defend = windowScore(boardMap, q, r, opp, me);
    const score = attack + defend * 0.8;
    if (score > bestScore) {
      bestScore = score;
      bestIdx = i;
    }
  }
  return bestIdx;
}

function getMove(gameState) {
  const me = gameState.currentPlayer;
  const opp = me === 1 ? 2 : 1;
  const board = gameState.boardMap;
  const candidates = getCandidates(board, gameState.board);
  const moves = [];

  for (let i = 0; i < gameState.movesLeftInTurn; i++) {
    if (candidates.length === 0) break;
    const idx = pickBest(board, candidates, me, opp);
    const [q, r] = candidates[idx];
    moves.push([q, r]);
    board.set(key(q, r), me); // commit for second placement
    candidates.splice(idx, 1);
  }

  return moves;
}

module.exports = { getMove };
