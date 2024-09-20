//! Utility methods for the Tendermint RPC crate.

use getrandom::getrandom;

use crate::prelude::*;

/// Produce a string containing a UUID.
///
/// Panics if random number generation fails.
pub fn uuid_str() -> String {
    let mut bytes = [0; 16];
    getrandom(&mut bytes).expect("RNG failure!");

    let mut builder = uuid::Builder::from_bytes(bytes);
    builder
        .set_variant(uuid::Variant::RFC4122)
        .set_version(uuid::Version::Random);

    builder.into_uuid().to_string()
}
