// Veridag — n=4, f=1 instance (one Byzantine). Simulation/model-check entry.
//
// Run from this directory:
//   quint run instance4.qnt --invariant=Agreement  --max-steps=25 --max-samples=200
//   quint run instance4.qnt --invariant=Finality   --max-steps=25 --max-samples=200
//   quint run instance4.qnt --invariant=Integrity  --max-steps=25 --max-samples=200
module instance4 {
  import consensus(VALIDATORS = Set(1, 2, 3, 4), BYZ = Set(4), MAX_WAVE = 3).* from "./consensus"
}
