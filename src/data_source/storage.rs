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

//! Persistent storage for data sources.
//!
//! Naturally, an archival query service such as this is heavily dependent on a persistent storage
//! implementation. This module defines the interfaces required of this storage. Any storage layer
//! implementing the appropriate interfaces can be used as the storage layer when constructing a
//! [`DataSource`](super::DataSource), which can in turn be used to instantiate the REST APIs
//! provided by this crate.
//!
//! This module also comes with a few pre-built persistence implementations:
//! * [`SqlStorage`]
//! * [`FileSystemStorage`]
//!

use crate::{
    availability::{
        BlockId, BlockQueryData, LeafId, LeafQueryData, QueryableBlock, TransactionHash,
        TransactionIndex,
    },
    Block,
};
use async_trait::async_trait;
use futures::stream::Stream;
use hotshot_types::traits::node_implementation::{NodeImplementation, NodeType};
use std::ops::RangeBounds;

pub mod fs;
pub mod sql;

pub use fs::FileSystemStorage;
pub use sql::SqlStorage;

/// Persistent storage for a HotShot blockchain.
///
/// This trait defines the interface which must be provided by the storage layer in order to
/// implement an availability data source. It is very similar to
/// [`AvailabilityDataSource`](crate::availability::AvailabilityDataSource) with every occurrence of
/// [`Fetch`](crate::availability::Fetch) replaced by [`Option`]. This is not a coincidence. The
/// purpose of the storage layer is to provide all of the functionality of the data source layer,
/// but independent of an external fetcher for missing data. Thus, when the storage layer encounters
/// missing, corrupt, or inaccessible data, it simply gives up and replaces the missing data with
/// [`None`], rather than creating an asynchronous fetch request to retrieve the missing data.
///
/// Rust gives us ways to abstract and deduplicate these two similar APIs, but they do not lead to a
/// better interface.
#[async_trait]
pub trait AvailabilityStorage<Types, I>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
{
    type LeafRange<R>: Stream<Item = Option<LeafQueryData<Types, I>>> + Unpin + Send + 'static
    where
        R: RangeBounds<usize> + Send;
    type BlockRange<R>: Stream<Item = Option<BlockQueryData<Types>>> + Unpin + Send + 'static
    where
        R: RangeBounds<usize> + Send;

    async fn get_leaf<ID>(&self, id: ID) -> Option<LeafQueryData<Types, I>>
    where
        ID: Into<LeafId<Types, I>> + Send + Sync;
    async fn get_block<ID>(&self, id: ID) -> Option<BlockQueryData<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync;

    async fn get_leaf_range<R>(&self, range: R) -> Self::LeafRange<R>
    where
        R: RangeBounds<usize> + Send + 'static;
    async fn get_block_range<R>(&self, range: R) -> Self::BlockRange<R>
    where
        R: RangeBounds<usize> + Send + 'static;

    async fn get_block_with_transaction(
        &self,
        hash: TransactionHash<Types>,
    ) -> Option<(BlockQueryData<Types>, TransactionIndex<Types>)>;
}
