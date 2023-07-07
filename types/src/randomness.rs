// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct DKGTranscript {
    // dkg todo: fill in the fields
    pub dummy_bytes: Vec<u8>,
}

impl DKGTranscript {
    pub fn new() -> Self {
        Self {
            dummy_bytes: vec![0],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct Randomness {
    // randomness todo: fill in the fields
    pub dummy_bytes: Vec<u8>,
}

impl Randomness {
    pub fn new() -> Self {
        Self {
            dummy_bytes: vec![0],
        }
    }
}
