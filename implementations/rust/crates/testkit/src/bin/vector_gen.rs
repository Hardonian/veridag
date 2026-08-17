//! Generate golden and malformed test vectors (spec conformance, Phase 2).

#![forbid(unsafe_code)]

use std::fs;
use std::path::PathBuf;

use veridag_capabilities::{Capability, CapabilityKind, Constraints};
use veridag_codec::{Encode, Encoder};
use veridag_crypto::{hash, Keypair};
use veridag_object_state::Object;
use veridag_protocol_types::{
    object_type, Address, ApplicationId, CapabilityId, ObjectRef, Ownership, ResourceBudget,
    CURRENT_PROTOCOL_VERSION,
};
use veridag_testkit::{GoldenVector, MalformedVector, SignatureVector};
use veridag_transaction::{Operation, SignedTransaction, Transaction};

fn hex(b: &[u8]) -> String {
    format!("0x{}", hex::encode(b))
}

fn out_dir() -> PathBuf {
    // <workspace>/../../../.. -> repo root, then protocol/test-vectors
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("../../../../protocol/test-vectors")
}

fn malformed_dir() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("../../../../conformance/malformed")
}

fn write_json<T: serde::Serialize>(dir: &PathBuf, name: &str, v: &T) {
    fs::create_dir_all(dir).unwrap();
    let p = dir.join(name);
    fs::write(&p, serde_json::to_string_pretty(v).unwrap()).unwrap();
    println!("wrote {}", p.display());
}

fn main() {
    let vectors = out_dir();
    let malformed = malformed_dir();

    // --- encoding golden vectors -------------------------------------------
    let mut enc: Vec<GoldenVector> = Vec::new();

    let mut e = Encoder::new();
    e.u64(0x0102_0304_0506_0708);
    enc.push(GoldenVector {
        name: "u64_be".into(),
        type_name: "u64".into(),
        description: "u64 0x0102030405060708 big-endian".into(),
        bytes: hex(e.as_bytes()),
        hash: None,
    });

    let mut e = Encoder::new();
    e.bool(true);
    enc.push(GoldenVector {
        name: "bool_true".into(),
        type_name: "bool".into(),
        description: "true encodes as 0x01".into(),
        bytes: hex(e.as_bytes()),
        hash: None,
    });

    let mut e = Encoder::new();
    e.bytes(b"veridag");
    enc.push(GoldenVector {
        name: "bytes_len_prefixed".into(),
        type_name: "bytes".into(),
        description: "\"veridag\" as u32be length + bytes".into(),
        bytes: hex(e.as_bytes()),
        hash: None,
    });

    let mut e = Encoder::new();
    e.option(&Some(0x2Au8), |e, v| e.u8(*v));
    enc.push(GoldenVector {
        name: "option_some".into(),
        type_name: "Option<u8>".into(),
        description: "Some(42)".into(),
        bytes: hex(e.as_bytes()),
        hash: None,
    });

    let mut e = Encoder::new();
    e.option::<u8>(&None, |e, v| e.u8(*v));
    enc.push(GoldenVector {
        name: "option_none".into(),
        type_name: "Option<u8>".into(),
        description: "None".into(),
        bytes: hex(e.as_bytes()),
        hash: None,
    });

    let mut e = Encoder::new();
    e.seq(&[1u32, 2, 3], |e, v| e.u32(*v));
    enc.push(GoldenVector {
        name: "seq_u32".into(),
        type_name: "Vec<u32>".into(),
        description: "[1,2,3] as count + elements".into(),
        bytes: hex(e.as_bytes()),
        hash: None,
    });

    // --- object vector -------------------------------------------------------
    let creator: Address = [7u8; 32];
    let obj = Object::new(
        Object::derive_id(&creator, 3),
        object_type::BALANCE,
        Ownership::Address(creator),
        100u64.to_be_bytes().to_vec(),
        vec![],
    );
    let obj_bytes = obj.to_bytes();
    enc.push(GoldenVector {
        name: "balance_object".into(),
        type_name: "Object".into(),
        description: "Balance object, creator 0x07.., nonce 3, amount 100".into(),
        bytes: hex(&obj_bytes),
        hash: Some(hex(&hash("VERIDAG_BMH_LEAF_V1", &obj_bytes))),
    });

    // --- capability vector ----------------------------------------------------
    let mut cap = Capability {
        id: CapabilityId::ZERO,
        issuer: [1u8; 32],
        holder: [2u8; 32],
        kind: CapabilityKind::Spend {
            max_per_epoch: 20,
            current_epoch_spent: 0,
        },
        constraints: Constraints {
            expiry_epoch: 1000,
            rate_limit: None,
            resource_limit: None,
        },
        delegable: false,
        revoked: false,
        parent: None,
    };
    cap.id = Capability::derive_id(&cap.fields_bytes());
    enc.push(GoldenVector {
        name: "spend_capability".into(),
        type_name: "Capability".into(),
        description: "Spend capability max 20/epoch, expiry epoch 1000".into(),
        bytes: hex(&cap.to_bytes()),
        hash: Some(hex(&cap.id.0)),
    });

    // --- transaction + signature vectors --------------------------------------
    let kp = Keypair::from_seed(&[0x11u8; 32]);
    let tx = Transaction {
        protocol_version: CURRENT_PROTOCOL_VERSION,
        chain_id: 1,
        sender: kp.address(),
        nonce: 0,
        expiry_epoch: 100,
        declared_reads: vec![],
        declared_writes: vec![],
        capabilities: vec![],
        operation: Operation::TransferValue {
            from: ObjectRef {
                id: Object::derive_id(&kp.address(), 0),
                expected: 0,
            },
            to: [9u8; 32],
            amount: 25,
        },
        resource_budget: ResourceBudget::default(),
        metadata: vec![],
    };
    let tx_bytes = tx.to_bytes();
    let sig = kp.sign("VERIDAG_TX_V1", &tx_bytes);
    let stx = SignedTransaction { tx, signature: sig };
    enc.push(GoldenVector {
        name: "transfer_transaction".into(),
        type_name: "SignedTransaction".into(),
        description: "Transfer 25 from seed 0x11.. key to 0x09.. address".into(),
        bytes: hex(&stx.to_bytes()),
        hash: Some(hex(&stx.id().0)),
    });

    let sigvec = SignatureVector {
        name: "ed25519_transfer".into(),
        secret_seed: hex(&[0x11u8; 32]),
        public_key: hex(&kp.public()),
        domain: "VERIDAG_TX_V1".into(),
        payload: hex(&tx_bytes),
        signature: hex(&sig),
    };

    // --- application vector (ApplicationId derivation) -------------------------
    let app_id = ApplicationId(hash("VERIDAG_APP_V1", b"service-market"));
    enc.push(GoldenVector {
        name: "application_id".into(),
        type_name: "ApplicationId".into(),
        description: "H(VERIDAG_APP_V1 || 'service-market')".into(),
        bytes: hex(&app_id.0),
        hash: None,
    });

    write_json(&vectors.join("encoding"), "golden.json", &enc);
    write_json(
        &vectors.join("signatures"),
        "signatures.json",
        &vec![sigvec],
    );

    // --- malformed vectors ------------------------------------------------------
    let bad: Vec<MalformedVector> = vec![
        MalformedVector {
            name: "bool_noncanonical".into(),
            type_name: "bool".into(),
            reason: "bool tag 0x02 is not 0x00/0x01".into(),
            bytes: "0x02".into(),
        },
        MalformedVector {
            name: "option_bad_tag".into(),
            type_name: "Option<u8>".into(),
            reason: "option tag 0x07 invalid".into(),
            bytes: "0x07".into(),
        },
        MalformedVector {
            name: "trailing_bytes".into(),
            type_name: "u8".into(),
            reason: "trailing byte after top-level value".into(),
            bytes: "0x0102".into(),
        },
        MalformedVector {
            name: "bytes_limit_exceeded".into(),
            type_name: "bytes".into(),
            reason: "declared length 0xFFFFFFFF exceeds MAX_BYTES".into(),
            bytes: "0xffffffff".into(),
        },
        MalformedVector {
            name: "length_overflow".into(),
            type_name: "bytes".into(),
            reason: "declared length 5 but only 1 byte present".into(),
            bytes: "0x00000005aa".into(),
        },
        MalformedVector {
            name: "seq_count_exceeded".into(),
            type_name: "Vec<u8>".into(),
            reason: "count 65537 exceeds MAX_SEQ".into(),
            bytes: "0x00010001".into(),
        },
        MalformedVector {
            name: "invalid_utf8".into(),
            type_name: "string".into(),
            reason: "0xff is not valid UTF-8".into(),
            bytes: "0x00000001ff".into(),
        },
        MalformedVector {
            name: "unknown_variant".into(),
            type_name: "Operation".into(),
            reason: "operation variant 99 undefined".into(),
            bytes: "0x0000000000000001".into(), // decoded within full tx context
        },
        MalformedVector {
            name: "truncated_u64".into(),
            type_name: "u64".into(),
            reason: "only 3 of 8 bytes present".into(),
            bytes: "0x010203".into(),
        },
    ];
    write_json(&malformed, "malformed.json", &bad);

    println!("vector generation complete");
}
