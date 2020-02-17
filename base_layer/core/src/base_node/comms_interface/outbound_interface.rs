// Copyright 2019. The Tari Project
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
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use crate::{
    base_node::comms_interface::{
        error::CommsInterfaceError,
        NodeCommsRequest,
        NodeCommsRequestType,
        NodeCommsResponse,
    },
    blocks::{blockheader::BlockHeader, Block},
    chain_storage::{ChainMetadata, HistoricalBlock},
    transactions::{
        transaction::{TransactionKernel, TransactionOutput},
        types::HashOutput,
    },
};
use futures::channel::mpsc::UnboundedSender;
use log::*;
use tari_comms::types::CommsPublicKey;
use tari_service_framework::reply_channel::SenderService;
use tower_service::Service;

pub const LOG_TARGET: &str = "c::bn::comms_interface::outbound_interface";

/// The OutboundNodeCommsInterface provides an interface to request information from remove nodes.
#[derive(Clone)]
pub struct OutboundNodeCommsInterface {
    request_sender:
        SenderService<(NodeCommsRequest, NodeCommsRequestType), Result<Vec<NodeCommsResponse>, CommsInterfaceError>>,
    block_sender: UnboundedSender<(Block, Vec<CommsPublicKey>)>,
}

impl OutboundNodeCommsInterface {
    /// Construct a new OutboundNodeCommsInterface with the specified SenderService.
    pub fn new(
        request_sender: SenderService<
            (NodeCommsRequest, NodeCommsRequestType),
            Result<Vec<NodeCommsResponse>, CommsInterfaceError>,
        >,
        block_sender: UnboundedSender<(Block, Vec<CommsPublicKey>)>,
    ) -> Self
    {
        Self {
            request_sender,
            block_sender,
        }
    }

    /// Request metadata from remote base nodes.
    pub async fn get_metadata(&mut self) -> Result<Vec<ChainMetadata>, CommsInterfaceError> {
        let mut responses = Vec::<ChainMetadata>::new();
        self.request_sender
            .call((NodeCommsRequest::GetChainMetadata, NodeCommsRequestType::Many))
            .await??
            .into_iter()
            .for_each(|response| {
                if let NodeCommsResponse::ChainMetadata(metadata) = response {
                    responses.push(metadata);
                }
            });
        trace!(target: LOG_TARGET, "Remote metadata requested: {:?}", responses,);
        Ok(responses)
    }

    /// Fetch the transaction kernels with the provided hashes from remote base nodes.
    pub async fn fetch_kernels(
        &mut self,
        hashes: Vec<HashOutput>,
    ) -> Result<Vec<TransactionKernel>, CommsInterfaceError>
    {
        if let Some(NodeCommsResponse::TransactionKernels(kernels)) = self
            .request_sender
            .call((NodeCommsRequest::FetchKernels(hashes), NodeCommsRequestType::Single))
            .await??
            .first()
        {
            Ok(kernels.clone())
        } else {
            Err(CommsInterfaceError::UnexpectedApiResponse)
        }
    }

    /// Fetch the block headers corresponding to the provided block numbers from remote base nodes.
    pub async fn fetch_headers(&mut self, block_nums: Vec<u64>) -> Result<Vec<BlockHeader>, CommsInterfaceError> {
        if let Some(NodeCommsResponse::BlockHeaders(headers)) = self
            .request_sender
            .call((NodeCommsRequest::FetchHeaders(block_nums), NodeCommsRequestType::Single))
            .await??
            .first()
        {
            Ok(headers.clone())
        } else {
            Err(CommsInterfaceError::UnexpectedApiResponse)
        }
    }

    /// Fetch the UTXOs with the provided hashes from remote base nodes.
    pub async fn fetch_utxos(
        &mut self,
        hashes: Vec<HashOutput>,
    ) -> Result<Vec<TransactionOutput>, CommsInterfaceError>
    {
        if let Some(NodeCommsResponse::TransactionOutputs(utxos)) = self
            .request_sender
            .call((NodeCommsRequest::FetchUtxos(hashes), NodeCommsRequestType::Single))
            .await??
            .first()
        {
            Ok(utxos.clone())
        } else {
            Err(CommsInterfaceError::UnexpectedApiResponse)
        }
    }

    /// Fetch the Historical Blocks corresponding to the provided block numbers from remote base nodes.
    pub async fn fetch_blocks(&mut self, block_nums: Vec<u64>) -> Result<Vec<HistoricalBlock>, CommsInterfaceError> {
        if let Some(NodeCommsResponse::HistoricalBlocks(blocks)) = self
            .request_sender
            .call((NodeCommsRequest::FetchBlocks(block_nums), NodeCommsRequestType::Single))
            .await??
            .first()
        {
            Ok(blocks.clone())
        } else {
            Err(CommsInterfaceError::UnexpectedApiResponse)
        }
    }

    /// Transmit a block to remote base nodes, excluding the provided peers.
    pub async fn propagate_block(
        &mut self,
        block: Block,
        exclude_peers: Vec<CommsPublicKey>,
    ) -> Result<(), CommsInterfaceError>
    {
        self.block_sender
            .unbounded_send((block, exclude_peers))
            .map_err(|_| CommsInterfaceError::BroadcastFailed)
    }
}
