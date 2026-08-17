// Veridag — deterministic run tests driving the commit rule. These prove the
// commit rule is reachable and correctly encoded (non-vacuous invariants), by
// constructing a coherent DAG and forcing an anchor commit.
module consensus_test {
  import consensus(VALIDATORS = Set(1, 2, 3, 4), BYZ = Set(4), MAX_WAVE = 3).* from "./consensus"

  // Helper: have all honest validators deliver all current vertices.
  action syncAll: bool = all {
    delivered' = HONEST.mapBy(_p => vertices.map(v => v.id)),
    vertices' = vertices,
    committedLog' = committedLog,
    nextWave' = nextWave,
  }

  // One honest proposal per step (a single var update per step). Three honest
  // proposals + sync = one round. Repeat for rounds 1..5, then commit wave 1.
  run driveCommit: bool =
    init
      // round 1
      .then(honestPropose(1)).then(honestPropose(2)).then(honestPropose(3))
      .then(syncAll)
      // round 2
      .then(honestPropose(1)).then(honestPropose(2)).then(honestPropose(3))
      .then(syncAll)
      // round 3
      .then(honestPropose(1)).then(honestPropose(2)).then(honestPropose(3))
      .then(syncAll)
      // round 4 (leader 1's anchor round for wave 1)
      .then(honestPropose(1)).then(honestPropose(2)).then(honestPropose(3))
      .then(syncAll)
      // round 5 (votes on the anchor)
      .then(honestPropose(1)).then(honestPropose(2)).then(honestPropose(3))
      .then(syncAll)
      .expect(HONEST.forall(p => frontierRound(delivered.get(p)) >= 5))
      // commit wave 1 for all honest validators
      .then(commit(1)).then(commit(2)).then(commit(3))
      .expect(HONEST.forall(p => committedLog.get(p).length() == 1))
      // invariants hold on this concrete path
      .expect(Agreement)
      .expect(Finality)
      .expect(Integrity)
}
