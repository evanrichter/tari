// Copyright 2018 The Tari Project
//
// Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
// following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
// disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
// following disclaimer in the documentation and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
// products derived from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
// INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE
//
// Portions of this file were originally copyrighted (c) 2018 The Grin Developers, issued under the Apache License,
// Version 2.0, available at http://www.apache.org/licenses/LICENSE-2.0.

use std::{
    cmp::Ordering,
    fmt::{Display, Formatter},
    io,
    io::{Read, Write},
};

use serde::{Deserialize, Serialize};
use tari_common_types::types::{Commitment, FixedHash, PublicKey, Signature};
use tari_utilities::{hex::Hex, message_format::MessageFormat};

use super::TransactionKernelVersion;
use crate::{
    consensus::{ConsensusDecoding, ConsensusEncoding, ConsensusEncodingSized, DomainSeparatedConsensusHasher},
    transactions::{
        tari_amount::MicroTari,
        transaction_components::{KernelFeatures, TransactionError},
        transaction_protocol::TransactionMetadata,
        TransactionHashDomain,
    },
};

/// The transaction kernel tracks the excess for a given transaction. For an explanation of what the excess is, and
/// why it is necessary, refer to the
/// [Mimblewimble TLU post](https://tlu.tarilabs.com/protocols/mimblewimble-1/sources/PITCHME.link.html?highlight=mimblewimble#mimblewimble).
/// The kernel also tracks other transaction metadata, such as the lock height for the transaction (i.e. the earliest
/// this transaction can be mined) and the transaction fee, in cleartext.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionKernel {
    pub version: TransactionKernelVersion,
    /// Options for a kernel's structure or use
    pub features: KernelFeatures,
    /// Fee originally included in the transaction this proof is for.
    pub fee: MicroTari,
    /// This kernel is not valid earlier than lock_height blocks
    /// The max lock_height of all *inputs* to this transaction
    pub lock_height: u64,
    /// Remainder of the sum of all transaction commitments (minus an offset). If the transaction is well-formed,
    /// amounts plus fee will sum to zero, and the excess is hence a valid public key.
    pub excess: Commitment,
    /// An aggregated signature of the metadata in this kernel, signed by the individual excess values and the offset
    /// excess of the sender.
    pub excess_sig: Signature,
    /// This is an optional field that must be set if the transaction contains a burned output.
    pub burn_commitment: Option<Commitment>,
}

impl TransactionKernel {
    pub fn new(
        version: TransactionKernelVersion,
        features: KernelFeatures,
        fee: MicroTari,
        lock_height: u64,
        excess: Commitment,
        excess_sig: Signature,
        burn_commitment: Option<Commitment>,
    ) -> TransactionKernel {
        TransactionKernel {
            version,
            features,
            fee,
            lock_height,
            excess,
            excess_sig,
            burn_commitment,
        }
    }

    /// Produce a canonical hash for a transaction kernel.
    pub fn hash(&self) -> FixedHash {
        DomainSeparatedConsensusHasher::<TransactionHashDomain>::new("transaction_kernel")
            .chain(self)
            .finalize()
            .into()
    }

    pub fn new_current_version(
        features: KernelFeatures,
        fee: MicroTari,
        lock_height: u64,
        excess: Commitment,
        excess_sig: Signature,
        burn_commitment: Option<Commitment>,
    ) -> TransactionKernel {
        TransactionKernel::new(
            TransactionKernelVersion::get_current_version(),
            features,
            fee,
            lock_height,
            excess,
            excess_sig,
            burn_commitment,
        )
    }

    pub fn is_coinbase(&self) -> bool {
        self.features.contains(KernelFeatures::COINBASE_KERNEL)
    }

    /// Is this a burned output kernel?
    pub fn is_burned(&self) -> bool {
        self.features.contains(KernelFeatures::BURN_KERNEL)
    }

    pub fn verify_signature(&self) -> Result<(), TransactionError> {
        let excess = self.excess.as_public_key();
        let r = self.excess_sig.get_public_nonce();
        let c = TransactionKernel::build_kernel_challenge(
            r,
            excess,
            self.fee,
            self.lock_height,
            &self.features,
            &self.burn_commitment,
        );
        if self.excess_sig.verify_challenge(excess, &c) {
            Ok(())
        } else {
            Err(TransactionError::InvalidSignatureError(
                "Verifying kernel signature".to_string(),
            ))
        }
    }

    /// This gets the burn commitment if it exists
    pub fn get_burn_commitment(&self) -> Result<&Commitment, TransactionError> {
        match self.burn_commitment {
            Some(ref burn_commitment) => Ok(burn_commitment),
            None => Err(TransactionError::InvalidKernel("Burn commitment not found".to_string())),
        }
    }

    /// This is a helper fuction for build kernel challange that does not take in the individual fields,
    /// but rather takes in the TransactionMetadata object.
    pub fn build_kernel_challenge_from_tx_meta(
        sum_public_nonces: &PublicKey,
        total_excess: &PublicKey,
        tx_meta: &TransactionMetadata,
    ) -> [u8; 32] {
        TransactionKernel::build_kernel_challenge(
            sum_public_nonces,
            total_excess,
            tx_meta.fee,
            tx_meta.lock_height,
            &tx_meta.kernel_features,
            &tx_meta.burn_commitment,
        )
    }

    /// Helper function to creates the kernel excess signature challenge.
    /// The challenge is defined as the hash of the following data:
    ///  Public nonce
    ///  Fee
    ///  Lock height
    ///  Features of the kernel
    ///  Burn commitment if present
    pub fn build_kernel_challenge(
        sum_public_nonces: &PublicKey,
        total_excess: &PublicKey,
        fee: MicroTari,
        lock_height: u64,
        features: &KernelFeatures,
        burn_commitment: &Option<Commitment>,
    ) -> [u8; 32] {
        DomainSeparatedConsensusHasher::<TransactionHashDomain>::new("kernel_signature")
            .chain(sum_public_nonces)
            .chain(total_excess)
            .chain(&fee)
            .chain(&lock_height)
            .chain(features)
            .chain(burn_commitment)
            .finalize()
    }
}

impl Display for TransactionKernel {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            fmt,
            "Fee: {}\nLock height: {}\nFeatures: {:?}\nExcess: {}\nExcess signature: {}\nCommitment: {}\n",
            self.fee,
            self.lock_height,
            self.features,
            self.excess.to_hex(),
            self.excess_sig
                .to_json()
                .unwrap_or_else(|_| "Failed to serialize signature".into()),
            match self.burn_commitment {
                Some(ref burn_commitment) => burn_commitment.to_hex(),
                None => "None".to_string(),
            }
        )
    }
}

impl PartialOrd for TransactionKernel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.excess_sig.partial_cmp(&other.excess_sig)
    }
}

impl Ord for TransactionKernel {
    fn cmp(&self, other: &Self) -> Ordering {
        self.excess_sig.cmp(&other.excess_sig)
    }
}

impl ConsensusEncoding for TransactionKernel {
    fn consensus_encode<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        self.version.consensus_encode(writer)?;
        self.features.consensus_encode(writer)?;
        self.fee.consensus_encode(writer)?;
        self.lock_height.consensus_encode(writer)?;
        self.excess.consensus_encode(writer)?;
        self.excess_sig.consensus_encode(writer)?;
        self.burn_commitment.consensus_encode(writer)?;
        Ok(())
    }
}

impl ConsensusEncodingSized for TransactionKernel {}

impl ConsensusDecoding for TransactionKernel {
    fn consensus_decode<R: Read>(reader: &mut R) -> Result<Self, io::Error> {
        let version = TransactionKernelVersion::consensus_decode(reader)?;
        let features = KernelFeatures::consensus_decode(reader)?;
        let fee = MicroTari::consensus_decode(reader)?;
        let lock_height = u64::consensus_decode(reader)?;
        let excess = Commitment::consensus_decode(reader)?;
        let excess_sig = Signature::consensus_decode(reader)?;
        let commitment = <Option<Commitment> as ConsensusDecoding>::consensus_decode(reader)?;
        let kernel = TransactionKernel::new(version, features, fee, lock_height, excess, excess_sig, commitment);
        Ok(kernel)
    }
}

#[cfg(test)]
mod tests {
    use tari_utilities::ByteArray;

    use super::*;
    use crate::{consensus::check_consensus_encoding_correctness, transactions::test_helpers::TestParams};

    #[test]
    fn consensus_encoding() {
        let test_params = TestParams::new();

        let output = TransactionKernel::new(
            TransactionKernelVersion::get_current_version(),
            KernelFeatures::all(),
            MicroTari::from(100),
            123,
            test_params.commit_value(321.into()),
            Signature::sign(
                test_params.spend_key.clone(),
                test_params.nonce.clone(),
                test_params.nonce.as_bytes(),
            )
            .unwrap(),
            Some(test_params.commit_value(321.into())),
        );
        check_consensus_encoding_correctness(output).unwrap();
    }
}
