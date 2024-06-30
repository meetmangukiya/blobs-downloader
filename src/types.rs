use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct BlobSidecarsDataSignedBlockHeaderMessage {
    pub slot: String,
    pub proposer_index: String,
    pub parent_root: String,
    pub state_root: String,
    pub body_root: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlobSidecarsDataSignedBlockHeader {
    pub message: BlobSidecarsDataSignedBlockHeaderMessage,
    pub signature: String,
}

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Serialize, Debug)]
pub struct BlobsDataToWrite {
    pub slot: usize,
    pub data: Vec<BlobSidecarsData>,
}
