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

//! Asynchronous fetching from external data availability providers.
//!
//! Occasionally, data will be missing from the local persistent storage of this query service. This
//! may be because the query service never received the data from the attached HotShot instance,
//! which happens for each block payload committed while the attached HotShot instance is not a
//! member of the DA committee. It may also be because the query service was started some time after
//! the start of consensus, and needs to catch up on existing data. Or it may simply be a result of
//! failures in the local storage medium.
//!
//! In any case, the query service is able to fetch missing data asynchronously and out of
//! chronological order, and pending [`Fetch`](crate::availability::Fetch) requests for the missing
//! data will resolve as soon as the data is available. Data can be fetched from any external data
//! availability service, including the HotShot CDN, the data availability committee which is
//! responsible for providing a particular payload, or another instance of this same query service.
//!
//! This module defines an abstract interface [`Fetcher`], which allows data to be fetched from any
//! data availability provider, as well as various implementations for different data sources,
//! including:
//! * [`QueryServiceFetcher`]
//!
//! We also provide combinators for modularly adding functionality to existing fetchers:
//! * [`AnyFetcher`]
//! * [`TestFetcher`](testing::TestFetcher)
//!

use crate::{
    availability::{
        BlockHash, BlockQueryData, LeafQueryData, QueryableBlock, UpdateAvailabilityData,
    },
    Block, Deltas, QueryResult, Resolvable,
};
use async_trait::async_trait;
use derivative::Derivative;
use hotshot_types::traits::node_implementation::{NodeImplementation, NodeType};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, hash::Hash};

mod any;
mod query_service;
pub mod testing;

pub use any::AnyFetcher;
pub use query_service::QueryServiceFetcher;

/// A request to fetch a resource from an external data availability provider.
///
/// The request contains all the context necessary to retrieve the desired resource. It also
/// contains a unique identifier for the desired resource, which can help to avoid making duplicate
/// requests for the same object at the same time.
pub trait FetchRequest: Debug + Send + Sync + 'static {
    type Key: Copy + Hash + Eq + Send + 'static;

    fn key(&self) -> Self::Key;
}

/// A request to fetch a missing block.
///
/// This type includes sufficient context to retrieve the desired block and build a
/// [`BlockQueryData`] from it.
///
/// Missing blocks are always requested by their hash, so that the hash of the retrieved block may
/// be compared with the requested hash, to ensure the correct data was retrieved. The request for a
/// missing block also includes additional information from the block header, such as `height` and
/// `timestamp`, so that the fetcher can retrieve a simple [`Block`] and use the extra context to
/// reconstruct a [`BlockQueryData`].
#[derive(Derivative, Deserialize, Serialize)]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
#[serde(bound = "")]
pub struct BlockRequest<Types>
where
    Types: NodeType,
{
    pub hash: BlockHash<Types>,
    pub height: u64,
    pub timestamp: i128,
}

impl<Types> FetchRequest for BlockRequest<Types>
where
    Types: NodeType,
{
    type Key = BlockHash<Types>;

    fn key(&self) -> Self::Key {
        self.hash
    }
}

/// A request to fetch a missing leaf.
///
/// This type includes sufficient context to retrieve the desired leaf and build a
/// [`LeafQueryData`] from it.
///
/// Unlike blocks, which are requested by hash, missing leaves are requested by height. This is
/// because the expected hash of a missing block can be gotten from the corresponding leaf, but if
/// the leaf itself is missing, there is no way for us to know what the hash of the missing leaf
/// would be. Therefore, instead of authenticating retrieved leaves by hash, a fetcher retrieving a
/// leaf from an untrusted source will need to receive the appropriate signed QCs justifying the
/// finality of the retrieved leaf at the given height.
#[derive(Derivative, Deserialize, Serialize)]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
#[serde(bound = "")]
pub struct LeafRequest {
    pub height: usize,
}

impl FetchRequest for LeafRequest {
    type Key = usize;

    fn key(&self) -> Self::Key {
        self.height
    }
}

/// A resource which can be fetched from an external data availability provider.
///
/// This trait helps to abstract over leaves and blocks, so much of the logic for retrieving both
/// kinds of objects is shared.
#[async_trait]
pub trait Fetchable<Types, I>: Clone + Debug + Send
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
{
    /// The information needed to retrieve the requested object.
    type Request: FetchRequest;

    /// Insert a fetched object into local storage.
    ///
    /// This method can be used after fetching an object from a remote data availability service in
    /// order to persist the fetched object in local storage, thus avoiding future fetches of the
    /// same object.
    async fn store<S>(self, storage: &mut S) -> Result<(), S::Error>
    where
        S: UpdateAvailabilityData<Types, I> + Send;
}

#[async_trait]
impl<Types, I> Fetchable<Types, I> for LeafQueryData<Types, I>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    Deltas<Types, I>: Resolvable<Block<Types>>,
{
    type Request = LeafRequest;

    async fn store<S>(self, storage: &mut S) -> Result<(), S::Error>
    where
        S: UpdateAvailabilityData<Types, I> + Send,
    {
        storage.insert_leaf(self).await
    }
}

#[async_trait]
impl<Types, I> Fetchable<Types, I> for BlockQueryData<Types>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
{
    type Request = BlockRequest<Types>;

    async fn store<S>(self, storage: &mut S) -> Result<(), S::Error>
    where
        S: UpdateAvailabilityData<Types, I> + Send,
    {
        storage.insert_block(self).await
    }
}

/// An interface for retrieving resources from external data availability providers.
#[async_trait]
pub trait Fetcher<Types, I, T>: Send + Sync + 'static
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    T: Fetchable<Types, I>,
{
    async fn fetch(&self, req: &T::Request) -> QueryResult<T>;
}

/// Convenience trait for fetchers which can fetch both blocks and leaves.
pub trait AvailabilityFetcher<Types, I>:
    Fetcher<Types, I, LeafQueryData<Types, I>> + Fetcher<Types, I, BlockQueryData<Types>>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    Deltas<Types, I>: Resolvable<Block<Types>>,
{
}

impl<Types, I, F> AvailabilityFetcher<Types, I> for F
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    F: Fetcher<Types, I, LeafQueryData<Types, I>> + Fetcher<Types, I, BlockQueryData<Types>>,
    Deltas<Types, I>: Resolvable<Block<Types>>,
{
}
