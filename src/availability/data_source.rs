// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

use super::query_data::{
    BlockHash, BlockQueryData, LeafQueryData, QueryableBlock, TransactionHash, TransactionIndex,
};
use crate::{Block, Deltas, Leaf, QueryError, QueryResult, Resolvable};
use async_trait::async_trait;
use commit::{Commitment, Committable};
use derivative::Derivative;
use derive_more::{Display, From};
use futures::stream::Stream;
use hotshot_types::traits::{
    node_implementation::{NodeImplementation, NodeType},
    signature_key::EncodedPublicKey,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::error::Error;
use std::fmt::Debug;
use std::ops::RangeBounds;

#[derive(Derivative, From, Display)]
#[derivative(Ord = "feature_allow_slow_enum")]
#[derivative(
    Copy(bound = ""),
    Debug(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = ""),
    Ord(bound = ""),
    Hash(bound = "")
)]
pub enum ResourceId<T: Committable> {
    #[display(fmt = "{_0}")]
    Number(usize),
    #[display(fmt = "{_0}")]
    Hash(Commitment<T>),
}

impl<T: Committable> Clone for ResourceId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Committable> PartialOrd for ResourceId<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub type BlockId<Types> = ResourceId<Block<Types>>;
pub type LeafId<Types, I> = ResourceId<Leaf<Types, I>>;

/// A notification of an error.
///
/// This event is broadcast, via [`AvailabilityDataSource::ErrorStream`], when an error occurs in an
/// asynchronous task that the data source cannot handle on its own. It includes the original error
/// that triggered the event as well as any available context about what the data source was trying
/// to do when it encountered the error, which may help the event handler resolve the problem.
#[derive(Derivative, Deserialize, Serialize)]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
#[serde(bound = "")]
pub struct ErrorEvent<Types>
where
    Types: NodeType,
{
    /// The error which triggered this event.
    pub error: QueryError,

    /// Additional information about what was happening when the event was triggered.
    ///
    /// Some errors are recoverable, but only with additional information that is beyond the scope
    /// of the data source itself. In such cases, `context` includes any data required to retrieve
    /// this additional information, and these errors can be resolved by providing the required
    /// information (such as a block payload which was missing from storage).
    pub context: Option<ErrorContext<Types>>,
}

/// Information about what was happening when an [`ErrorEvent`] event was triggered.
#[derive(Derivative, Deserialize, Serialize, From)]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
#[serde(bound = "")]
pub enum ErrorContext<Types>
where
    Types: NodeType,
{
    /// The data source was looking up a block which was not available in its storage.
    ///
    /// Fetching the block with the given hash and adding it to storage via
    /// [`UpdateAvailabilityData::insert_block`] may resolve the problem.
    ///
    /// Missing blocks are always requested by their hash, so that the hash of the retrieved block
    /// may be compared with the requested hash, to ensure the correct data was retrieved. The
    /// context for a missing block also includes additional information from the block header, such
    /// as `height` and `timestamp`, so that the fetcher can retrieve a simple [`Block`] and use the
    /// extra context to reconstruct a [`BlockQueryData`].
    MissingBlock(MissingBlockContext<Types>),

    /// The data source was looking up a leaf which was not available in its storage.
    ///
    /// Fetching the leaf at the given height and adding it to storage via
    /// [`UpdateAvailabilityData::insert_leaf`] may resolve the problem.
    ///
    /// Unlike blocks, which are requested by hash, missing leaves are requested by height. This is
    /// because the expected hash of a missing block can be gotten from the corresponding leaf, but
    /// if the leaf itself is missing, there is no way for us to know what the hash of the missing
    /// leaf would be. Therefore, instead of authenticating retrieved leaves by hash, a fetcher
    /// retrieving a leaf from an untrusted source will need to receive the appropriate signed QCs
    /// justifying the finality of the retrieved leaf at the given height. This authentication is
    /// internal to the specific fetcher implementation; the data source itself is not resonsible
    /// for authenticating fetched leaves.
    MissingLeaf(MissingLeafContext),
}

/// Context for a missing block.
///
/// This type includes sufficient context to retrieve the desired block and build a
/// [`BlockQueryData`] from it.
#[derive(Derivative, Deserialize, Serialize)]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
#[serde(bound = "")]
pub struct MissingBlockContext<Types>
where
    Types: NodeType,
{
    pub hash: BlockHash<Types>,
    pub height: u64,
    pub timestamp: i128,
}

/// Context for a missing leaf.
///
/// This type includes sufficient context to retrieve the desired leaf and build a
/// [`LeafQueryData`] from it.
#[derive(Derivative, Deserialize, Serialize)]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
#[serde(bound = "")]
pub struct MissingLeafContext {
    pub height: usize,
}

#[async_trait]
pub trait AvailabilityDataSource<Types: NodeType, I: NodeImplementation<Types>>
where
    Block<Types>: QueryableBlock,
{
    type LeafStream: Stream<Item = LeafQueryData<Types, I>> + Unpin + Send;
    type BlockStream: Stream<Item = BlockQueryData<Types>> + Unpin + Send;
    type ErrorStream: Stream<Item = ErrorEvent<Types>> + Unpin + Send;

    type LeafRange<'a, R>: 'a + Stream<Item = QueryResult<LeafQueryData<Types, I>>> + Unpin
    where
        Self: 'a,
        R: RangeBounds<usize> + Send;
    type BlockRange<'a, R>: 'a + Stream<Item = QueryResult<BlockQueryData<Types>>> + Unpin
    where
        Self: 'a,
        R: RangeBounds<usize> + Send;

    async fn get_leaf<ID>(&self, id: ID) -> QueryResult<LeafQueryData<Types, I>>
    where
        ID: Into<LeafId<Types, I>> + Send + Sync;
    async fn get_block<ID>(&self, id: ID) -> QueryResult<BlockQueryData<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync;

    async fn get_leaf_range<R>(&self, range: R) -> QueryResult<Self::LeafRange<'_, R>>
    where
        R: RangeBounds<usize> + Send;
    async fn get_block_range<R>(&self, range: R) -> QueryResult<Self::BlockRange<'_, R>>
    where
        R: RangeBounds<usize> + Send;

    /// Returns the block containing a transaction with the given `hash` and the transaction's
    /// position in the block.
    async fn get_block_with_transaction(
        &self,
        hash: TransactionHash<Types>,
    ) -> QueryResult<(BlockQueryData<Types>, TransactionIndex<Types>)>;

    async fn get_proposals(
        &self,
        proposer: &EncodedPublicKey,
        limit: Option<usize>,
    ) -> QueryResult<Vec<LeafQueryData<Types, I>>>;
    async fn count_proposals(&self, proposer: &EncodedPublicKey) -> QueryResult<usize>;

    async fn subscribe_leaves(&self, height: usize) -> QueryResult<Self::LeafStream>;
    async fn subscribe_blocks(&self, height: usize) -> QueryResult<Self::BlockStream>;

    /// Get notifications when errors occur in the data source.
    ///
    /// A data source frequently spawns long-running background tasks, such as connections to remote
    /// services and long-running streams to clients. Since these tasks run in the background, it is
    /// generally undesirable for the entire task to fail, and even if it were, there is no caller
    /// for it to return an error to. Instead, callers can be notified indirectly of errors that
    /// occur in such tasks by subscribing to this stream.
    ///
    /// This can be useful for, for example, a GUI application which needs to display a popup
    /// notification to alert the user when something unusual occurs. Or, additional components can
    /// be attached via this stream to attempt to recover from errors, such as by fetching resources
    /// whose absence triggered an error from another source.
    async fn subscribe_errors(&self) -> Self::ErrorStream;
}

#[async_trait]
pub trait UpdateAvailabilityData<Types: NodeType, I: NodeImplementation<Types>>
where
    Block<Types>: QueryableBlock,
{
    type Error: Error + Debug + Send + Sync + 'static;
    async fn insert_leaf(&mut self, leaf: LeafQueryData<Types, I>) -> Result<(), Self::Error>
    where
        Deltas<Types, I>: Resolvable<Block<Types>>;
    async fn insert_block(&mut self, block: BlockQueryData<Types>) -> Result<(), Self::Error>;
}
