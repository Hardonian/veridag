# 03 — Canonical Encoding (VCE-1)

Status: NORMATIVE

Veridag Canonical Encoding version 1 (**VCE-1**) is the only encoding permitted on
consensus-visible paths. Network/RPC layers MAY use other formats, but every value
entering consensus MUST be re-encoded to VCE-1 before hashing, signing, or
comparing.

Design goal: **equivalent semantic objects have exactly one canonical byte
representation**, and any byte string that is not the canonical encoding of a
semantic object MUST be rejected.

VCE-1 is deliberately not bincode, not serde_json, not CBOR, not MessagePack, not
protobuf. It is specified here from first principles so that an independent team
can implement it in any language without reading Rust code.

## Primitive types

| Type | Encoding | Notes |
|------|----------|-------|
| `u8` | 1 byte | |
| `u16`, `u32`, `u64` | big-endian, fixed width | no variable-length ints |
| `bool` | `0x00` or `0x01` | any other byte is INVALID |
| `Hash` | 32 bytes | output of protocol hash |
| `Address` | 32 bytes | |
| `Ed25519PublicKey` | 32 bytes | |
| `Ed25519Signature` | 64 bytes | |
| `byte string` | `u32be(len) || bytes` | len is exact; see limits |
| `string` | `u32be(len) || UTF-8 bytes` | MUST be valid UTF-8 |

There are no variable-length integers in VCE-1. This removes a class of
non-canonical-integer attacks.

## Composite types

* **Sequence `Vec<T>`** — `u32be(count) || T_0 || ... || T_{count-1}`.
* **Fixed array `[T; N]`** — `T_0 || ... || T_{N-1}` (no length prefix).
* **Option `Option<T>`** — tag byte: `0x00` (None) or `0x01` (Some) followed by
  `T`. Any other tag is INVALID.
* **Struct** — fields encoded in the order declared by this specification, with
  no padding and no field names.
* **Tagged union (enum)** — `u8` variant index followed by the variant's fields.
  Variant indices are assigned by this specification and are part of the protocol.
  Unknown variant indices MUST be rejected.
* **Map** — VCE-1 has no map type. Maps MUST be represented as a sequence of
  `(key, value)` pairs sorted by canonical key bytes, with duplicate keys
  rejected. This makes canonical map ordering explicit.

## Limits (attacker-facing paths)

Unless a specific structure states otherwise:

* byte strings and strings: `<= 2^20` bytes;
* sequences: `<= 2^16` elements;
* nested structure depth: `<= 16`.

A decoder MUST enforce limits before allocation where feasible (length-delimited
formats allow checking the declared length against the remaining input and the
limit before allocating).

## Decoding rules (MUST)

A VCE-1 decoder MUST reject input that:

1. has trailing bytes after the top-level value;
2. contains a non-minimal or non-canonical form (e.g., `bool` not in `{0,1}`,
   `Option` tag not in `{0,1}`, unknown enum variant, unsorted map keys,
   duplicate map keys);
3. violates a length or count limit;
4. contains invalid UTF-8 in a string field;
5. is truncated.

Decoding MUST be total: every byte string either decodes to exactly one value or
is rejected. There are no "default" fields.

## Golden vectors

`protocol/test-vectors/encoding/` contains machine-readable vectors of the form:

```json
{ "name": "...", "type": "...", "value": <semantic JSON>, "bytes": "0x..." }
```

Every conforming implementation MUST reproduce `bytes` from `value`, and MUST
accept `bytes` and decode it to `value`.

## Malformed vectors

`conformance/malformed/` contains byte strings that MUST be rejected, covering:
noncanonical bool/option tags, trailing bytes, wrong variant tags, oversized
declared lengths, invalid UTF-8, unsorted/duplicate map keys, truncation.

Every conforming implementation MUST reject every malformed vector.

## Versioning

VCE-1 is versioned as part of the protocol version. Any change to these rules is a
breaking change requiring a new protocol version (17-upgrades). Historical bytes
never change meaning.
