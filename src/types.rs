use std::sync::Arc;

use beacon_node::beacon_chain::types::{
    BlobSidecar, BlobSidecarList, EthSpec, Hash256, SignedBeaconBlockHeader,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlobSidecarsDataSignedBlockHeaderMessage {
    pub slot: String,
    pub proposer_index: String,
    pub parent_root: String,
    pub state_root: String,
    pub body_root: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlobSidecarsDataSignedBlockHeader {
    pub message: BlobSidecarsDataSignedBlockHeaderMessage,
    pub signature: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlobSidecarsData {
    pub index: String,
    pub blob: String,
    pub kzg_commitment: String,
    pub kzg_proof: String,
    pub signed_block_header: BlobSidecarsDataSignedBlockHeader,
    pub kzg_commitment_inclusion_proof: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct BlobSidecarsResponse {
    pub data: Vec<BlobSidecarsData>,
}

#[derive(Debug, Deserialize)]
pub struct BlockHeaderData {
    pub root: String,
    pub canonical: bool,
    pub header: BlobSidecarsDataSignedBlockHeader, // same contents
}

#[derive(Debug, Deserialize)]
pub struct BlockHeadersData {
    pub execution_optimistic: bool,
    pub finalized: bool,
    pub data: Vec<BlockHeaderData>,
}

#[derive(Debug, Deserialize)]
pub struct SingleBlockHeaderData {
    pub execution_optimistic: bool,
    pub finalized: bool,
    pub data: BlockHeaderData,
}

#[derive(Serialize, Debug, Clone)]
pub struct BlobsDataToWrite {
    pub slot: usize,
    pub data: Vec<BlobSidecarsData>,
    pub root: Hash256,
}

impl<E: EthSpec> Into<BlobSidecar<E>> for BlobSidecarsData {
    fn into(self) -> BlobSidecar<E> {
        BlobSidecar {
            index: self.index.parse::<u64>().expect("parsing index failed"),
            blob: hex::decode(self.blob.replace("0x", ""))
                .expect("parsing blob to bytes failed")
                .into(), // self.blob.parse::<_>().expect("parsing blob failed"),
            kzg_commitment: self
                .kzg_commitment
                .parse::<_>()
                .expect("parsing kzg_commitment failed"),
            kzg_proof: self
                .kzg_proof
                .parse::<_>()
                .expect("parsing kzg_proof failed"),
            kzg_commitment_inclusion_proof: self
                .kzg_commitment_inclusion_proof
                .iter()
                .map(|x| {
                    x.parse::<_>()
                        .expect("parsing kzg_commitment_inclusion_proof failed")
                })
                .collect::<Vec<_>>()
                .into(),
            signed_block_header: SignedBeaconBlockHeader {
                message: beacon_node::beacon_chain::types::BeaconBlockHeader {
                    slot: self
                        .signed_block_header
                        .message
                        .slot
                        .parse()
                        .expect("parsing signed_block_header.message.slot failed"),
                    proposer_index: self
                        .signed_block_header
                        .message
                        .proposer_index
                        .parse()
                        .expect("parsing signed_block_header.message.proposer_index failed"),
                    parent_root: self
                        .signed_block_header
                        .message
                        .parent_root
                        .parse()
                        .expect("parsing signed_block_header.message.parent_root failed"),
                    state_root: self
                        .signed_block_header
                        .message
                        .state_root
                        .parse()
                        .expect("parsing signed_block_header.message.state_root failed"),
                    body_root: self
                        .signed_block_header
                        .message
                        .body_root
                        .parse()
                        .expect("parsing signed_block_header.message.body_root failed"),
                },
                signature: self
                    .signed_block_header
                    .signature
                    .parse()
                    .expect("parsing signed_block_header.signature failed"),
            },
        }
    }
}

impl<E: EthSpec> Into<BlobSidecarList<E>> for &BlobsDataToWrite {
    fn into(self) -> BlobSidecarList<E> {
        BlobSidecarList::new(
            self.data
                .iter()
                .map(|x| Arc::new(Into::<BlobSidecar<E>>::into(x.clone())))
                .collect::<Vec<_>>(),
        )
        .expect("BlobsDataToWrite to BlobSidecarList failed")
    }
}
