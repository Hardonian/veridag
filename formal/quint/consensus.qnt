// Veridag BaselineDagBft — formal model (Level 2). See spec 09-consensus.
//
// Models the safety core: a structured DAG with a deterministic leader schedule
// and a quorum-based wave commit rule, under Byzantine equivocation, withholding,
// and asynchronous delivery. Transactions are abstracted (safety of ordering does
// not depend on their contents); Integrity requires committed anchors be real.
//
// Checked invariants: Agreement, Finality, Integrity.
module consensus {
  import basicSpells.* from "./basicSpells"

  // ---- Parameters ----------------------------------------------------------
  const VALIDATORS: Set[int]   // all validators
  const BYZ: Set[int]          // Byzantine subset, |BYZ| <= (N-1)/3
  const MAX_WAVE: int          // waves explored by the checker

  // ---- Derived --------------------------------------------------------------
  pure val N: int = VALIDATORS.size()
  pure val F: int = (N - 1) / 3
  pure val QUORUM: int = 2 * F + 1
  pure val WAVE: int = 4
  pure val HONEST: Set[int] = VALIDATORS.exclude(BYZ)

  // Deterministic leader schedule: canonically sorted validator list; leader of
  // wave w is LEADER_ORDER[(w-1) % N]. We build a list from the set with fold
  // (arbitrary order) and then sortList to a canonical, deterministic order.
  pure val LEADER_ORDER: List[int] =
    sortList(VALIDATORS.fold(List(), (acc, v) => acc.append(v)), (a, b) => a < b)

  // ---- Types -----------------------------------------------------------------
  // A vertex id: author, round, and a tag distinguishing equivocations.
  type VertexId = { author: int, round: int, tag: int }
  // A vertex: its id plus parent references (previous round).
  type Vertex = { id: VertexId, parents: Set[VertexId] }

  // ---- State ------------------------------------------------------------------
  var vertices: Set[Vertex]                 // all proposed vertices
  var delivered: int -> Set[VertexId]       // per-honest-validator delivered ids
  var committedLog: int -> List[VertexId]   // per-honest-validator committed anchors
  var nextWave: int -> int                  // per-honest-validator next wave to commit

  // ---- Helpers -----------------------------------------------------------------
  pure def leader(w: int): int = LEADER_ORDER[(w - 1) % N]

  pure def anchors(verticesIn: Set[Vertex], w: int): Set[VertexId] =
    verticesIn
      .filter(v => v.id.author == leader(w) and v.id.round == w * WAVE)
      .map(v => v.id)

  // In validator p's delivered view, does anchor a (wave w) have QUORUM votes in
  // round w*WAVE+1?
  pure def anchorHasQuorum(view: Set[VertexId], verticesIn: Set[Vertex], a: VertexId, w: int): bool = {
    val voters =
      verticesIn.filter(v =>
        view.contains(v.id) and v.id.round == w * WAVE + 1 and v.parents.contains(a))
    voters.map(v => v.id.author).size() >= QUORUM
  }

  // Commit candidates for wave w in p's view: anchors present and quorum-voted.
  pure def commitCandidate(view: Set[VertexId], verticesIn: Set[Vertex], w: int): Set[VertexId] =
    anchors(verticesIn, w).filter(a => view.contains(a) and anchorHasQuorum(view, verticesIn, a, w))

  // Deterministic tie-break among candidate anchors: minimal tag, then author.
  // (Safety needs only that all honest validators apply the SAME rule; the
  // reference implementation uses the spec's seed-based permutation.)
  pure def chooseAnchor(cands: Set[VertexId], w: int): VertexId =
    cands.fold(
      { author: leader(w), round: w * WAVE, tag: 1000000 },
      (best, a) => if (a.tag < best.tag) a else best)

  // ---- Actions -----------------------------------------------------------------
  action init: bool = all {
    vertices' = Set(),
    delivered' = HONEST.mapBy(_p => Set()),
    committedLog' = HONEST.mapBy(_p => List()),
    nextWave' = HONEST.mapBy(_p => 1),
  }

  // Honest proposal: exactly one vertex per (author, round) once a quorum of the
  // previous round is in the proposer's view.
  // An honest validator's current frontier round: the highest round it has a
  // quorum for (0 if none). It proposes at frontier+1 referencing a quorum of
  // the previous round. This is constructive, matching the real protocol's
  // round-advance rule (spec 08), so the DAG actually grows and commits occur.
  pure def frontierRound(view: Set[VertexId]): int =
    // highest round r such that view has a quorum of round r; 0 if none.
    // We bound the search by MAX_WAVE*WAVE.
    (1.to(MAX_WAVE * WAVE)).fold(0, (best, r) =>
      if (view.filter(id => id.round == r).map(id => id.author).size() >= QUORUM and r > best)
        r
      else
        best)

  action honestPropose(p: int): bool = {
    // Honest validators reference ALL delivered vertices of their frontier round
    // (this is what real DAG-BFT does; nondeterminism lives in delivery, not in
    // parent selection). This makes anchors gather votes and lets commits occur.
    pure def frontier: int = frontierRound(delivered.get(p))
    pure def frontierParents: Set[VertexId] =
      delivered.get(p).filter(id => id.round == frontier)
    all {
      p.in(HONEST),
      frontier + 1 <= MAX_WAVE * WAVE,
      // need a quorum at the frontier to advance (or genesis at frontier 0)
      if (frontier == 0)
        true
      else
        frontierParents.map(id => id.author).size() >= QUORUM,
      val parents = if (frontier == 0) Set() else frontierParents
      vertices' = vertices.union(Set({ id: { author: p, round: frontier + 1, tag: 0 }, parents: parents })),
      delivered' = delivered.put(p, delivered.get(p).union(Set({ author: p, round: frontier + 1, tag: 0 }))),
      committedLog' = committedLog,
      nextWave' = nextWave,
    }
  }

  // Byzantine proposal: arbitrary equivocating vertices with valid-shaped parents.
  action byzPropose(q: int): bool = {
    nondet r = 1.to(MAX_WAVE * WAVE).oneOf()
    nondet tag = 1.to(3).oneOf()
    nondet parents = vertices.map(v => v.id).powerset().oneOf()
    all {
      q.in(BYZ),
      if (r == 1) parents == Set()
      else parents.map(id => id.author).size() >= QUORUM,
      vertices' = vertices.union(Set({ id: { author: q, round: r, tag: tag }, parents: parents })),
      delivered' = delivered,
      committedLog' = committedLog,
      nextWave' = nextWave,
    }
  }

  // Asynchronous delivery (delay, reorder, duplicate, withhold).
  action deliver(p: int): bool = {
    nondet id = vertices.map(v => v.id).oneOf()
    all {
      p.in(HONEST),
      delivered' = delivered.put(p, delivered.get(p).union(Set(id))),
      vertices' = vertices,
      committedLog' = committedLog,
      nextWave' = nextWave,
    }
  }

  // Commit: if p's view yields a candidate for its next wave, append it.
  action commit(p: int): bool = {
    all {
      p.in(HONEST),
      nextWave.get(p) <= MAX_WAVE,
      commitCandidate(delivered.get(p), vertices, nextWave.get(p)).size() >= 1,
      committedLog' = committedLog.put(
        p,
        committedLog.get(p).append(
          chooseAnchor(commitCandidate(delivered.get(p), vertices, nextWave.get(p)), nextWave.get(p)))),
      nextWave' = nextWave.put(p, nextWave.get(p) + 1),
      vertices' = vertices,
      delivered' = delivered,
    }
  }

  action step: bool = any {
    nondet h = HONEST.oneOf()
    any {
      honestPropose(h),
      deliver(h),
      commit(h),
    },
    nondet b = BYZ.oneOf()
    byzPropose(b),
  }

  // ---- Invariants ----------------------------------------------------------------
  // Agreement: no two honest validators finalize conflicting histories (one log is
  // a prefix of the other).
  val Agreement: bool =
    HONEST.forall(p => HONEST.forall(q => and {
      val lp = committedLog.get(p)
      val lq = committedLog.get(q)
      val shorter = if (lp.length() <= lq.length()) lp else lq
      val longer = if (lp.length() <= lq.length()) lq else lp
      shorter.indices().forall(i => shorter.nth(i) == longer.nth(i))
    }))

  // Finality: at any shared wave index, both honest validators committed the same
  // anchor (commits are append-only and consistent).
  val Finality: bool =
    HONEST.forall(p => HONEST.forall(q =>
      committedLog.get(p).indices().intersect(committedLog.get(q).indices()).forall(i =>
        committedLog.get(p).nth(i) == committedLog.get(q).nth(i))))

  // Integrity: every committed anchor was proposed and is a real anchor of its wave.
  val Integrity: bool =
    HONEST.forall(p =>
      committedLog.get(p).indices().forall(i =>
        val a = committedLog.get(p).nth(i)
        and {
          vertices.map(v => v.id).contains(a),
          a.round == (i + 1) * WAVE,
          a.author == leader(i + 1),
        }))
}
