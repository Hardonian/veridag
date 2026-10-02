#![no_main]

use libfuzzer_sys::fuzz_target;
use veridag_codec::{Decode, Decoder, Encode};
use veridag_dag::Vertex;
use veridag_transaction::SignedTransaction;

fuzz_target!(|data: &[u8]| {
    let mut transaction_decoder = Decoder::new(data);
    if let Ok(transaction) = SignedTransaction::decode(&mut transaction_decoder) {
        if transaction_decoder.finish().is_ok() {
            let canonical = transaction.to_bytes();
            let mut roundtrip = Decoder::new(&canonical);
            let decoded = SignedTransaction::decode(&mut roundtrip).expect("canonical transaction");
            roundtrip.finish().expect("canonical transaction has no trailing bytes");
            assert_eq!(transaction, decoded);
        }
    }

    let mut vertex_decoder = Decoder::new(data);
    if let Ok(vertex) = Vertex::decode(&mut vertex_decoder) {
        if vertex_decoder.finish().is_ok() {
            let canonical = vertex.to_bytes();
            let mut roundtrip = Decoder::new(&canonical);
            let decoded = Vertex::decode(&mut roundtrip).expect("canonical vertex");
            roundtrip.finish().expect("canonical vertex has no trailing bytes");
            assert_eq!(vertex, decoded);
        }
    }
});

