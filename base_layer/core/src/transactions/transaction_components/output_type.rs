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
    fmt::{Display, Formatter},
    io,
    io::Read,
};

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::consensus::{ConsensusDecoding, ConsensusEncoding, ConsensusEncodingSized};

#[derive(Debug, Clone, Copy, Hash, Deserialize_repr, Serialize_repr, PartialEq, Eq, FromPrimitive)]
#[repr(u8)]
pub enum OutputType {
    /// An standard non-coinbase output.
    Standard = 0,
    /// Output is a coinbase output, must not be spent until maturity.
    Coinbase = 1,
    /// Output is a burned output and can not be spent ever.
    Burn = 2,
}

impl OutputType {
    /// Returns a single byte that represents this OutputType
    pub fn as_byte(self) -> u8 {
        self as u8
    }

    /// Returns the OutputType that corresponds to this OutputType. If the byte does not correspond to any OutputType,
    /// None is returned.
    pub fn from_byte(value: u8) -> Option<Self> {
        FromPrimitive::from_u8(value)
    }

    pub const fn all() -> &'static [Self] {
        &[OutputType::Standard, OutputType::Coinbase, OutputType::Burn]
    }
}

impl Default for OutputType {
    fn default() -> Self {
        Self::Standard
    }
}

impl ConsensusEncoding for OutputType {
    fn consensus_encode<W: io::Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        writer.write_all(&[self.as_byte()])?;
        Ok(())
    }
}

impl ConsensusEncodingSized for OutputType {
    fn consensus_encode_exact_size(&self) -> usize {
        1
    }
}

impl ConsensusDecoding for OutputType {
    fn consensus_decode<R: Read>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        let output_type = OutputType::from_byte(buf[0]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Byte {:x?} is not a valid OutputType", buf[0]),
            )
        })?;
        Ok(output_type)
    }
}

impl Display for OutputType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Debug "shortcut" works because variants do not have fields
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::check_consensus_encoding_correctness;

    #[test]
    fn it_converts_from_byte_to_output_type() {
        assert_eq!(OutputType::from_byte(0), Some(OutputType::Standard));
        assert_eq!(OutputType::from_byte(1), Some(OutputType::Coinbase));
        assert_eq!(OutputType::from_byte(2), Some(OutputType::Burn));
        assert_eq!(OutputType::from_byte(255), None);
    }

    #[test]
    fn consensus_encoding() {
        let t = OutputType::Standard;
        check_consensus_encoding_correctness(t).unwrap();
    }
}
