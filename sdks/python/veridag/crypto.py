"""
Cryptographic routines for the Veridag Python SDK.
BLAKE3 domain-separated hashing and Ed25519 signing.
"""

import struct
from cryptography.hazmat.primitives.asymmetric import ed25519
from .types import Address, ObjectId

IV = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
    0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
]

MSG_PERM = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8]


def rotr(w: int, c: int) -> int:
    return ((w >> c) | (w << (32 - c))) & 0xFFFFFFFF


def g(s, a, b, c, d, mx, my):
    s[a] = (s[a] + s[b] + mx) & 0xFFFFFFFF
    s[d] = rotr(s[d] ^ s[a], 16)
    s[c] = (s[c] + s[d]) & 0xFFFFFFFF
    s[b] = rotr(s[b] ^ s[c], 12)
    s[a] = (s[a] + s[b] + my) & 0xFFFFFFFF
    s[d] = rotr(s[d] ^ s[a], 8)
    s[c] = (s[c] + s[d]) & 0xFFFFFFFF
    s[b] = rotr(s[b] ^ s[c], 7)


def round_fn(s, m):
    g(s, 0, 4, 8, 12, m[0], m[1])
    g(s, 1, 5, 9, 13, m[2], m[3])
    g(s, 2, 6, 10, 14, m[4], m[5])
    g(s, 3, 7, 11, 15, m[6], m[7])
    g(s, 0, 5, 10, 15, m[8], m[9])
    g(s, 1, 6, 11, 12, m[10], m[11])
    g(s, 2, 7, 8, 13, m[12], m[13])
    g(s, 3, 4, 9, 14, m[14], m[15])


def compress(cv, block: bytes, block_len: int, counter: int, flags: int):
    s = list(cv) + list(IV)
    s[12] = counter & 0xFFFFFFFF
    s[13] = 0
    s[14] = block_len & 0xFFFFFFFF
    s[15] = flags & 0xFFFFFFFF
    m = list(struct.unpack("<16I", block.ljust(64, b"\x00")))
    for _ in range(7):
        round_fn(s, m)
        m = [m[MSG_PERM[i]] for i in range(16)]
    return [(s[i] ^ s[i + 8]) & 0xFFFFFFFF for i in range(8)]


def blake3(data: bytes) -> bytes:
    """Pure Python BLAKE3 implementation."""
    cv = list(IV)
    CHUNK_START, CHUNK_END, ROOT = 1, 2, 8
    n_blocks = (len(data) + 63) // 64 or 1
    for b in range(n_blocks):
        block = data[b * 64 : (b + 1) * 64]
        flags = (CHUNK_START if b == 0 else 0) | (CHUNK_END | ROOT if b == n_blocks - 1 else 0)
        cv = compress(cv, block, len(block), 0, flags)
    return struct.pack("<8I", *cv)


def hash_domain(domain: str, payload: bytes) -> bytes:
    """Protocol domain-separated hash: BLAKE3(domain || 0x00 || payload)."""
    return blake3(domain.encode("utf-8") + b"\x00" + payload)


def address_of(public_key: bytes) -> Address:
    """Derive address from Ed25519 public key."""
    return hash_domain("VERIDAG_ADDRESS_V1", public_key)


def derive_object_id(creator: Address, nonce: int) -> ObjectId:
    """Derive canonical ObjectId for creator and nonce."""
    buf = creator + struct.pack(">Q", nonce)
    return hash_domain("VERIDAG_OBJECT_ID_V1", buf)


class Keypair:
    """Ed25519 Keypair wrapper for transaction signing."""

    def __init__(self, private_key: ed25519.Ed25519PrivateKey):
        self._priv = private_key
        self._pub = private_key.public_key().public_bytes_raw()
        self._addr = address_of(self._pub)

    @classmethod
    def from_seed(cls, seed: bytes) -> "Keypair":
        if len(seed) != 32:
            raise ValueError("seed must be 32 bytes")
        priv = ed25519.Ed25519PrivateKey.from_private_bytes(seed)
        return cls(priv)

    def public(self) -> bytes:
        return self._pub

    def address(self) -> Address:
        return self._addr

    def sign(self, domain: str, payload: bytes) -> bytes:
        """Sign domain-separated payload."""
        msg = domain.encode("utf-8") + b"\x00" + payload
        return self._priv.sign(msg)
