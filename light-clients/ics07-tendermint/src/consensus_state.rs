use ibc::prelude::*;

use core::{convert::Infallible, fmt::Debug};

use serde::Serialize;
use tendermint::{hash::Algorithm, time::Time, Hash};
use tendermint_proto::{google::protobuf as tpb, Protobuf};

use crate::{error::Error, header::Header};
use ibc::{core::ics23_commitment::commitment::CommitmentRoot, timestamp::Timestamp};
use ibc_proto::ibc::lightclients::tendermint::v1::ConsensusState as RawConsensusState;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConsensusState {
	pub timestamp: Time,
	pub root: CommitmentRoot,
	pub next_validators_hash: Hash,
}

impl ConsensusState {
	pub fn new(root: CommitmentRoot, timestamp: Time, next_validators_hash: Hash) -> Self {
		Self { timestamp, root, next_validators_hash }
	}
}

impl ibc::core::ics02_client::client_consensus::ConsensusState for ConsensusState {
	type Error = Infallible;

	fn root(&self) -> &CommitmentRoot {
		&self.root
	}

	fn timestamp(&self) -> Timestamp {
		self.timestamp.into()
	}

	fn encode_to_vec(&self) -> Vec<u8> {
		self.encode_vec()
	}
}

impl Protobuf<RawConsensusState> for ConsensusState {}

impl TryFrom<RawConsensusState> for ConsensusState {
	type Error = Error;

	fn try_from(raw: RawConsensusState) -> Result<Self, Self::Error> {
		let ibc_proto::google::protobuf::Timestamp { seconds, nanos } = raw
			.timestamp
			.ok_or_else(|| Error::invalid_raw_consensus_state("missing timestamp".into()))?;
		// FIXME: shunts like this are necessary due to
		// https://github.com/informalsystems/tendermint-rs/issues/1053
		let proto_timestamp = tpb::Timestamp { seconds, nanos };
		let timestamp = proto_timestamp
			.try_into()
			.map_err(|e| Error::invalid_raw_consensus_state(format!("invalid timestamp: {}", e)))?;

		Ok(Self {
			root: raw
				.root
				.ok_or_else(|| {
					Error::invalid_raw_consensus_state("missing commitment root".into())
				})?
				.hash
				.into(),
			timestamp,
			next_validators_hash: Hash::from_bytes(Algorithm::Sha256, &raw.next_validators_hash)
				.map_err(|e| Error::invalid_raw_consensus_state(e.to_string()))?,
		})
	}
}

impl From<ConsensusState> for RawConsensusState {
	fn from(value: ConsensusState) -> Self {
		// FIXME: shunts like this are necessary due to
		// https://github.com/informalsystems/tendermint-rs/issues/1053
		let tpb::Timestamp { seconds, nanos } = value.timestamp.into();
		let timestamp = ibc_proto::google::protobuf::Timestamp { seconds, nanos };

		RawConsensusState {
			timestamp: Some(timestamp),
			root: Some(ibc_proto::ibc::core::commitment::v1::MerkleRoot {
				hash: value.root.into_vec(),
			}),
			next_validators_hash: value.next_validators_hash.as_bytes().to_vec(),
		}
	}
}

impl From<tendermint::block::Header> for ConsensusState {
	fn from(header: tendermint::block::Header) -> Self {
		Self {
			root: CommitmentRoot::from_bytes(header.app_hash.as_ref()),
			timestamp: header.time,
			next_validators_hash: header.next_validators_hash,
		}
	}
}

impl From<Header> for ConsensusState {
	fn from(header: Header) -> Self {
		Self::from(header.signed_header.header)
	}
}

#[cfg(test)]
mod tests {
	use tendermint_rpc::endpoint::abci_query::AbciQuery;
	use test_log::test;

	use ibc::test::test_serialization_roundtrip;

	#[test]
	fn serialization_roundtrip_no_proof() {
		let json_data = include_str!("mock/query/serialization/consensus_state.json");
		test_serialization_roundtrip::<AbciQuery>(json_data);
	}

	#[test]
	fn serialization_roundtrip_with_proof() {
		let json_data = include_str!("mock/query/serialization/consensus_state_proof.json");
		test_serialization_roundtrip::<AbciQuery>(json_data);
	}
}