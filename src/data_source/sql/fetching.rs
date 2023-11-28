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

//! Asynchronous data fetching.
//!
//! This module contains the infrastructure required to make the
//! [`SqlDataSource`](super::SqlDataSource) compatible with asynchronous, out-of-order retrieval of
//! data from external data availability sources. It defines an abstracted notion of a [`Resource`]
//! and a corresponding [`Fetcher`], which allows us to share data retrieval logic between blocks
//! and leaves. On top of these abstractions, we build asynchronous streams which always yield
//! resources in chronological order, even if they are received out of order, and which are able to
//! report missing objects to an external fetcher for retrieval.

use super::{
    parse::{parse_block, parse_leaf, BLOCK_COLUMNS, LEAF_COLUMNS},
    versioned_channel::VersionedReceiver,
};
use crate::{
    availability::{BlockQueryData, ErrorContext, ErrorEvent, LeafQueryData, ResourceId},
    Block, Deltas, Leaf, NotFoundSnafu, QueryError, QueryResult, QueryableBlock, Resolvable,
};
use async_compatibility_layer::async_primitives::broadcast::BroadcastSender;
use async_std::sync::Arc;
use async_trait::async_trait;
use commit::Committable;
use futures::{
    future::{self, TryFutureExt},
    stream::{self, BoxStream, StreamExt},
};
use hotshot_types::traits::node_implementation::{NodeImplementation, NodeType};
use snafu::OptionExt;
use std::ops::{Bound, RangeBounds};
use tokio_postgres::{types::ToSql, Client};

/// Abstraction for some functionality which is common to blocks and leaves.
#[async_trait]
pub(super) trait Resource<Types, I>: Clone + Sized + 'static
where
    Types: NodeType,
    I: NodeImplementation<Types>,
{
    type ConsensusObject: Committable;
    type Fetcher: Fetcher<Types, Self>;

    /// Get a single item of this resource.
    async fn get<ID>(client: &Client, id: ID) -> QueryResult<Self>
    where
        ID: Into<ResourceId<Self::ConsensusObject>> + Send;

    /// Get a contiguous, chronological range of this resource, where available.
    async fn get_range<R>(
        client: &Client,
        range: R,
    ) -> QueryResult<BoxStream<'static, QueryResult<Self>>>
    where
        R: RangeBounds<usize> + Send;

    /// Subscribe to a chronological stream of items of this resource.
    ///
    /// * `client`: a connection to the underlying database
    /// * `errors`: a handle for surfacing errors and requesting the retrieval of missing objects
    /// * `fetcher`: an interface for receiving retrieved objects from an external fetcher
    /// * `height`: the height to start streaming from
    async fn subscribe(
        client: Arc<Client>,
        errors: BroadcastSender<ErrorEvent<Types>>,
        fetcher: Self::Fetcher,
        height: usize,
    ) -> BoxStream<'static, Self> {
        // State maintained by the stream.
        struct State<Types, I, T>
        where
            Types: NodeType,
            I: NodeImplementation<Types>,
            T: Resource<Types, I>,
        {
            client: Arc<Client>,
            available: BoxStream<'static, QueryResult<T>>,
            errors: BroadcastSender<ErrorEvent<Types>>,
            fetcher: T::Fetcher,
            height: usize,
        }

        // Get a range of items, flattening errors.
        async fn get_range<Types, I, T>(
            client: &Client,
            height: usize,
        ) -> BoxStream<'static, QueryResult<T>>
        where
            Types: NodeType,
            I: NodeImplementation<Types>,
            T: Resource<Types, I> + 'static,
        {
            // We must `await` the future returned by `get_range`, since it borrows from `client`.
            // This way, when this future finally returns, the resulting stream will not borrow from
            // `client`, since the `get_range` future is already resolved.
            let res = T::get_range(client, height..).await;
            // `get_range` can fail. We don't care to distinguish failure getting the range from
            // failure getting an individual element, so we flatten the `Result<Stream<Result>>`
            // into a `Stream<Result>`: first wrap the result in a future (that doesn't borrow from
            // `client`) then flatten.
            future::ready(res).try_flatten_stream().boxed()
        }

        let state = State::<Types, I, Self> {
            available: get_range(&client, height).await,
            client,
            errors,
            fetcher,
            height,
        };

        stream::unfold(state, |mut state| async move {
            // Try to fetch the object, retrying on errors, if we expect it to be available in our
            // local storage.
            let mut fetch_result = state.available.next().await;
            let obj = loop {
                match fetch_result {
                    Some(Ok(obj)) => break obj,
                    Some(Err(QueryError::Missing)) => {
                        // An object which should have been available is missing. Retrieve the
                        // object from an external fetcher.
                        tracing::warn!("missing object at height {}", state.height);
                        break state
                            .fetcher
                            .fetch(&state.client, &state.errors, state.height)
                            .await;
                    }
                    Some(Err(err)) => {
                        // We have neither reached the end of the stream of currently available
                        // objects nor encountered a missing object, but something else has gone
                        // wrong while trying to fetch an object which should have been available.
                        // Since the object is not explicitly missing, there is a chance we will be
                        // able to fetch it successfully if we just try again (maybe we temporarily
                        // lost connection to the database, for instance).
                        tracing::error!("error fetching object at height {}: {err}", state.height);
                        fetch_result = Some(Self::get(&state.client, state.height).await);
                        continue;
                    }
                    None => {
                        // We have reached the end of the stream of currently available objects.
                        // Wait for the next object to be provided.
                        let obj = state.fetcher.wait(state.height).await;
                        // Reset `state.available` in case more objects have become available while
                        // we were waiting for the missing one. This stream will be consulted when
                        // we are polled for the next item.
                        state.available = get_range(&state.client, state.height + 1).await;
                        break obj;
                    }
                }
            };

            // Move to the next object.
            state.height += 1;
            // Yield the object we just retrieved.
            Some((obj, state))
        })
        .boxed()
    }
}

#[async_trait]
pub(super) trait Fetcher<Types, T>: Send
where
    Types: NodeType,
{
    /// Fetch a missing resource at the given height.
    ///
    /// This method may communicate with an external resource fetcher by sending messages on the
    /// error stream `errors` and receiving responses via `self`.
    ///
    /// This method will not fail, but may suspend indefinitely until the requested resource is able
    /// to be retrieved. Thus, care must be taken only to fetch a resource that is known to exist
    /// somewhere. Asking for, say, the one millionth block, when the current block height is only
    /// 100, make take a very long time.
    async fn fetch(
        &mut self,
        client: &Client,
        errors: &BroadcastSender<ErrorEvent<Types>>,
        height: usize,
    ) -> T;

    /// Wait for the object with the given height to be received.
    ///
    /// Unlike [`fetch`](Self::fetch), this method does not attempt to trigger the fetcher to
    /// retrieve the object. Instead, [`wait`](Self::wait) is completely passive; the retrieval
    /// must be spontaneous or triggered elsewhere, otherwise this method may suspend indefinitely.
    ///
    /// For example, this method could be used to wait for a resource like a block or a leaf which
    /// has not yet been produced by consensus, since a retrieval is triggered spontaneously for
    /// each newly produced leaf/block pair.
    async fn wait(&mut self, height: usize) -> T;
}

#[async_trait]
impl<Types, I> Fetcher<Types, LeafQueryData<Types, I>>
    for VersionedReceiver<LeafQueryData<Types, I>>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
{
    async fn fetch(
        &mut self,
        _client: &Client,
        errors: &BroadcastSender<ErrorEvent<Types>>,
        height: usize,
    ) -> LeafQueryData<Types, I> {
        // Send a missing leaf error event to trigger the external fetcher to retrieve the leaf.
        let event = ErrorEvent {
            error: QueryError::Missing,
            context: Some(ErrorContext::MissingLeaf(height)),
        };
        if errors.send_async(event.clone()).await.is_err() {
            // If the send fails, it means there is no fetcher task waiting on the other end of this
            // stream. This means the missing leaf will never be fetched, and we will never yield
            // the next leaf in this stream. There is nothing we can do about that but complain.
            tracing::error!("failed to send error event {event:?}, stream may block forever");
        }

        // Wait for the requested leaf.
        self.wait(height).await
    }

    async fn wait(&mut self, height: usize) -> LeafQueryData<Types, I> {
        // Wait for the desired leaf to be provided. Leaves may be inserted out of order, so we
        // need to loop until we receive the correct one.
        loop {
            let leaf = match self.next().await {
                Some(leaf) => leaf,
                None => {
                    // The stream can only end if the send end of the channel has been dropped. In
                    // this case, the send end is owned by the data source itself, so this can only
                    // happen if the entire data source/server is being shut down, but some async
                    // task which receives from the channel has not been properly cleaned up. In
                    // this case it is appropriate to panic and kill the task.
                    panic!("leaf fetching channel closed");
                }
            };
            if leaf.height() == height as u64 {
                break leaf;
            }
        }
    }
}

/// Interface for receiving messages from an external block fetcher.
///
/// In order to fetch a missing block, we need to know its hash, which means we need the
/// corresponding leaf. If we don't have that, we might need to fetch it also, Thus, this fetcher
/// includes channels to receive messages about both blocks and leaves.
pub(super) struct BlockFetcher<Types, I>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
{
    pub(super) blocks: VersionedReceiver<BlockQueryData<Types>>,
    pub(super) leaves: VersionedReceiver<LeafQueryData<Types, I>>,
}

#[async_trait]
impl<Types, I> Fetcher<Types, BlockQueryData<Types>> for BlockFetcher<Types, I>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    Deltas<Types, I>: Resolvable<Block<Types>>,
{
    async fn fetch(
        &mut self,
        client: &Client,
        errors: &BroadcastSender<ErrorEvent<Types>>,
        height: usize,
    ) -> BlockQueryData<Types> {
        // Blocks are always fetched by hash, so we can compare the hash of the fetched block to the
        // requested hash and verify we got the right data. So first, we need to look up the
        // expected hash for this height from the database. This function blocks until it succeeds,
        // rather than returning errors, so we need to retry if we get transient database errors.
        let hash = loop {
            match client
                .query_opt(
                    "SELECT hash FROM header WHERE height = $1",
                    &[&(height as i64)],
                )
                .await
            {
                Ok(Some(row)) => {
                    let hash_str: String = row.get("hash");
                    match hash_str.parse() {
                        Ok(hash) => break hash,
                        Err(err) => {
                            // We got something from the database that doesn't represent a block
                            // hash. This should never happen, since we are fully in control of what
                            // we put _in_ the database, but the best we can do in this case is
                            // treat it as a database error.
                            tracing::warn!("invalid hash {hash_str} for block {height}: {err}");
                            continue;
                        }
                    }
                }
                Ok(None) => {
                    // If the query succeeded, but the requested header was not there, we need to
                    // fetch the missing leaf (which includes the header) in order to find the block
                    // hash that we are supposed to be looking up.
                    let leaf = self.leaves.fetch(client, errors, height).await;
                    break leaf.block_hash();
                }
                Err(err) => {
                    // The query failed for some reason. We don't know if the leaf is available or
                    // not, so don't fetch it. Just retry, and this query should eventually succeed.
                    tracing::warn!("error looking up header {height} to find block hash: {err}");
                    continue;
                }
            }
        };

        // Send a missing block error event to trigger the external fetcher to retrieve the block.
        let event = ErrorEvent {
            error: QueryError::Missing,
            context: Some(ErrorContext::MissingBlock(hash)),
        };
        if errors.send_async(event.clone()).await.is_err() {
            // If the send fails, it means there is no fetcher task waiting on the other end of this
            // stream. This means the missing block will never be fetched, and we will never yield
            // the next block in this stream. There is nothing we can do about that but complain.
            tracing::error!("failed to send error event {event:?}, stream may block forever");
        }

        // Wait for the requested block.
        self.wait(height).await
    }

    async fn wait(&mut self, height: usize) -> BlockQueryData<Types> {
        // If we're waiting on a block, we must have already found the corresponding leaf.
        // Therefore, we do not care about messages queued up in the leaf channel, and we can drain
        // it to free up memory.
        self.leaves.drain();

        // Wait for the desired block to be provided. Blocks may be inserted out of order, so we
        // need to loop until we receive the correct one.
        loop {
            let block = match self.blocks.next().await {
                Some(block) => block,
                None => {
                    // The stream can only end if the send end of the channel has been dropped. In
                    // this case, the send end is owned by the data source itself, so this can only
                    // happen if the entire data source/server is being shut down, but some async
                    // task which receives from the channel has not been properly cleaned up. In
                    // this case it is appropriate to panic and kill the task.
                    panic!("block fetching channel closed");
                }
            };
            if block.height() == height as u64 {
                break block;
            }
        }
    }
}

#[async_trait]
impl<Types, I> Resource<Types, I> for LeafQueryData<Types, I>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
{
    type ConsensusObject = Leaf<Types, I>;
    type Fetcher = VersionedReceiver<LeafQueryData<Types, I>>;

    async fn get<ID>(client: &Client, id: ID) -> QueryResult<Self>
    where
        ID: Into<ResourceId<Self::ConsensusObject>> + Send,
    {
        let (where_clause, param): (&str, Box<dyn ToSql + Send + Sync>) = match id.into() {
            ResourceId::Number(n) => ("height = $1", Box::new(n as i64)),
            ResourceId::Hash(h) => ("hash = $1", Box::new(h.to_string())),
        };
        let query = format!("SELECT {LEAF_COLUMNS} FROM leaf WHERE {where_clause}");
        let row = client
            .query_opt(&query, &[&*param])
            .await
            .map_err(|err| QueryError::Error {
                message: err.to_string(),
            })?
            .context(NotFoundSnafu)?;
        parse_leaf(row)
    }

    async fn get_range<R>(
        client: &Client,
        range: R,
    ) -> QueryResult<BoxStream<'static, QueryResult<Self>>>
    where
        R: RangeBounds<usize> + Send,
    {
        let (where_clause, params) = bounds_to_where_clause(range, "height");
        let query = format!("SELECT {LEAF_COLUMNS} FROM leaf{where_clause} ORDER BY height ASC");
        let rows = client
            .query_raw(&query, params)
            .await
            .map_err(|err| QueryError::Error {
                message: err.to_string(),
            })?;

        Ok(rows
            .map(|res| {
                parse_leaf(res.map_err(|err| QueryError::Error {
                    message: err.to_string(),
                })?)
            })
            .boxed())
    }
}

#[async_trait]
impl<Types, I> Resource<Types, I> for BlockQueryData<Types>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    Deltas<Types, I>: Resolvable<Block<Types>>,
{
    type ConsensusObject = Block<Types>;
    type Fetcher = BlockFetcher<Types, I>;

    async fn get<ID>(client: &Client, id: ID) -> QueryResult<Self>
    where
        ID: Into<ResourceId<Self::ConsensusObject>> + Send,
    {
        let (where_clause, param): (&str, Box<dyn ToSql + Send + Sync>) = match id.into() {
            ResourceId::Number(n) => ("h.height = $1", Box::new(n as i64)),
            ResourceId::Hash(h) => ("h.hash = $1", Box::new(h.to_string())),
        };
        // ORDER BY h.height ASC ensures that if there are duplicate blocks, we return the first one.
        let query = format!(
            "SELECT {BLOCK_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON h.height = p.height
              WHERE {where_clause}
              ORDER BY h.height ASC
              LIMIT 1"
        );
        let row = client
            .query_opt(&query, &[&*param])
            .await
            .map_err(|err| QueryError::Error {
                message: err.to_string(),
            })?
            .context(NotFoundSnafu)?;
        parse_block(row)
    }

    async fn get_range<R>(
        client: &Client,
        range: R,
    ) -> QueryResult<BoxStream<'static, QueryResult<Self>>>
    where
        R: RangeBounds<usize> + Send,
    {
        let (where_clause, params) = bounds_to_where_clause(range, "h.height");
        let query = format!(
            "SELECT {BLOCK_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON h.height = p.height
              {where_clause}
              ORDER BY h.height ASC"
        );
        let rows = client
            .query_raw(&query, params)
            .await
            .map_err(|err| QueryError::Error {
                message: err.to_string(),
            })?;

        Ok(rows
            .map(|res| {
                parse_block(res.map_err(|err| QueryError::Error {
                    message: err.to_string(),
                })?)
            })
            .boxed())
    }
}

/// Convert range bounds to a SQL where clause constraining a given column.
///
/// Returns the where clause as a string and a list of query parameters. We assume that there are no
/// other parameters in the query; that is, parameters in the where clause will start from $1.
fn bounds_to_where_clause<R>(range: R, column: &str) -> (String, Vec<i64>)
where
    R: RangeBounds<usize>,
{
    let mut bounds = vec![];
    let mut params = vec![];

    match range.start_bound() {
        Bound::Included(n) => {
            params.push(*n as i64);
            bounds.push(format!("{column} >= ${}", params.len()));
        }
        Bound::Excluded(n) => {
            params.push(*n as i64);
            bounds.push(format!("{column} > ${}", params.len()));
        }
        Bound::Unbounded => {}
    }
    match range.end_bound() {
        Bound::Included(n) => {
            params.push(*n as i64);
            bounds.push(format!("{column} <= ${}", params.len()));
        }
        Bound::Excluded(n) => {
            params.push(*n as i64);
            bounds.push(format!("{column} < ${}", params.len()));
        }
        Bound::Unbounded => {}
    }

    let mut where_clause = bounds.join(" AND ");
    if !where_clause.is_empty() {
        where_clause = format!(" WHERE {where_clause}");
    }

    (where_clause, params)
}
