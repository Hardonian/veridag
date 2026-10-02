# Veridag Python SDK

Python client, canonical transaction builder, and Ed25519 signing utilities for
Veridag. The SDK is pre-GA and intended for development networks until the
runtime capability matrix promotes the relevant server paths.

```python
from veridag import Keypair, TxBuilder, VeridagClient

key = Keypair.from_seed(bytes([1]) * 32)
client = VeridagClient("http://127.0.0.1:8080", token="development-token")
health = client.health()
```

See the repository `docs/capability-matrix.md` before production evaluation.
