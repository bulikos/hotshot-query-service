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
//! chronological order, and queries that depend on the missing data will start to succeed as soon
//! as the data is available. Data can be fetched from any external data availability service,
//! including the HotShot CDN, the data availability committee which is responsible for providing a
//! particular payload, or another instance of this same query service.
//!
//! This module defines an abstract interface [`Fetch`], which allows data to be fetched from any
//! data availability provider, as well as various implementations for different data sources,
//! including:
//! * [`QueryServiceFetcher`]
//!
//! We also provide combinators for modularly adding functionality to existing fetchers:
//! * [`AnyFetcher`]
//! * [`TestFetcher`](testing::TestFetcher)
//!
//! Finally, we provide an async task [`run`], which attaches a data fetcher to a [`data
//! source`](crate::data_source) so that it can resolve missing data errors encountered by the data
//! source by fetching the missing data and providing it to the data source.
//!

use crate::{
    availability::{
        AvailabilityDataSource, BlockHash, BlockQueryData, ErrorContext, LeafQueryData,
        MissingBlockContext, MissingLeafContext, QueryableBlock, UpdateAvailabilityData,
    },
    data_source::VersionedDataSource,
    Block, Deltas, QueryResult, Resolvable,
};
use async_std::{
    sync::{Arc, Mutex, RwLock},
    task::spawn,
};
use async_trait::async_trait;
use futures::stream::StreamExt;
use hotshot_types::traits::node_implementation::{NodeImplementation, NodeType};
use std::{collections::HashSet, fmt::Debug, hash::Hash};

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

impl FetchRequest for MissingLeafContext {
    type Key = usize;

    fn key(&self) -> Self::Key {
        self.height
    }
}

impl<Types> FetchRequest for MissingBlockContext<Types>
where
    Types: NodeType,
{
    type Key = BlockHash<Types>;

    fn key(&self) -> Self::Key {
        self.hash
    }
}

/// A resource which can be fethced from an external data availability provider.
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
    async fn insert<D>(self, data_source: &mut D) -> Result<(), D::Error>
    where
        D: UpdateAvailabilityData<Types, I> + Send;
}

#[async_trait]
impl<Types, I> Fetchable<Types, I> for LeafQueryData<Types, I>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    Deltas<Types, I>: Resolvable<Block<Types>>,
{
    type Request = MissingLeafContext;

    async fn insert<D>(self, data_source: &mut D) -> Result<(), D::Error>
    where
        D: UpdateAvailabilityData<Types, I> + Send,
    {
        data_source.insert_leaf(self).await
    }
}

#[async_trait]
impl<Types, I> Fetchable<Types, I> for BlockQueryData<Types>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
{
    type Request = MissingBlockContext<Types>;

    async fn insert<D>(self, data_source: &mut D) -> Result<(), D::Error>
    where
        D: UpdateAvailabilityData<Types, I> + Send,
    {
        data_source.insert_block(self).await
    }
}

/// An interface for retrieving resources from external data availability providers.
#[async_trait]
pub trait Fetch<Types, I, T>: Send + Sync + 'static
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    T: Fetchable<Types, I>,
{
    async fn fetch(&self, req: &T::Request) -> QueryResult<T>;
}

#[derive(Debug)]
struct Fetcher<R: FetchRequest, F, D> {
    pending: Arc<Mutex<HashSet<R::Key>>>,
    inner: Arc<F>,
    data_source: Arc<RwLock<D>>,
}

impl<R, F, D> Fetcher<R, F, D>
where
    R: FetchRequest,
{
    fn new(inner: Arc<F>, data_source: Arc<RwLock<D>>) -> Self {
        Self {
            inner,
            data_source,
            pending: Default::default(),
        }
    }

    async fn fetch<Types, I, T>(&mut self, req: R)
    where
        Types: NodeType,
        I: NodeImplementation<Types>,
        Block<Types>: QueryableBlock,
        D: UpdateAvailabilityData<Types, I> + VersionedDataSource + Send + Sync + 'static,
        F: Fetch<Types, I, T>,
        T: Fetchable<Types, I, Request = R>,
    {
        // Check if we need to fetch this request (that is, if a fetch of the same resource is not
        // already in progress). If so, insert the unique `key` for the request into `pending`,
        // which gives this particular call the unique responsibility of spawning a task to do the
        // fetch.
        let needs_fetch = { self.pending.lock().await.insert(req.key()) };
        if !needs_fetch {
            return;
        }

        // Spawn a task to do the fetch in the background, and pass the fetched object to the data
        // source upon completion.
        let pending = self.pending.clone();
        let fetcher = self.inner.clone();
        let data_source = self.data_source.clone();
        spawn(async move {
            tracing::info!("fetching request {req:?}");

            // Keep trying until we get the desired object.
            let obj = loop {
                match fetcher.fetch(&req).await {
                    Ok(obj) => break obj,
                    Err(err) => {
                        tracing::warn!("failed to fetch requested object {req:?}: {err}");
                        continue;
                    }
                }
            };
            tracing::info!("retrieved requested object for {req:?}: {obj:?}");

            // Send the object to the data source. If we encounter errors, we must retry: we cannot
            // complete this request until the data source has accepted the new object, otherwise
            // some task which is waiting on the object may never become unblocked, and there is
            // nothing to trigger another fetch of the same object.
            loop {
                let mut ds = data_source.write().await;
                if let Err(err) = obj.clone().insert(&mut *ds).await {
                    tracing::error!("error inserting fetched object: {err}");
                    // Revert any partial changes which were made to the data source.
                    ds.revert().await;
                    continue;
                }
                if let Err(err) = ds.commit().await {
                    tracing::error!("error committing fetched object: {err}");
                    // Revert any partial changes which were made to the data source.
                    ds.revert().await;
                    continue;
                }
                break;
            }
            // Mark this request complete.
            {
                pending.lock().await.remove(&req.key());
            }
        });
    }
}

/// Help a data source recover from errors by fetching missing objects.
///
/// This function is meant to be spawned and run as a background task: it will not return unless the
/// linked data source is shut down completely.
///
/// The function will subscribe to error notifications from `data_source`. Each time `data_source`
/// encounters an error, if the error is due to a missing resource, this function will use `fetcher`
/// to retrieve the resource from an external service. Upon success, it will insert the resource
/// back into `data_source`, which may allow `data_source` to recover from the original error or
/// avoid future errors.
///
/// # Examples
///
/// Initialize a data source and attach a fetcher to fill in missing data from another query
/// service.
///
/// ```
/// # use hotshot_query_service::{availability::QueryableBlock, Block, Deltas, Resolvable};
/// # use hotshot_types::traits::node_implementation::{NodeImplementation, NodeType};
/// # async fn doc<Types, I>() -> anyhow::Result<()>
/// # where
/// #   Types: NodeType,
/// #   I: NodeImplementation<Types>,
/// #   Block<Types>: QueryableBlock,
/// #   Deltas<Types, I>: Resolvable<Block<Types>>,
/// # {
/// use async_std::{
///     sync::{Arc, RwLock},
///     task::spawn,
/// };
/// use hotshot_query_service::{
///     data_source::sql::Config,
///     fetcher::{self, QueryServiceFetcher},
/// };
///
/// let data_source = Config::default().connect::<Types, I>().await?;
/// let da = QueryServiceFetcher::new("https://backup.query-service".parse()?).await;
/// spawn(fetcher::run(Arc::new(da), Arc::new(RwLock::new(data_source))));
/// # Ok(())
/// # }
/// ```
pub async fn run<Types, I, F, D>(fetcher: Arc<F>, data_source: Arc<RwLock<D>>)
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    Deltas<Types, I>: Resolvable<Block<Types>>,
    F: Fetch<Types, I, BlockQueryData<Types>> + Fetch<Types, I, LeafQueryData<Types, I>>,
    D: AvailabilityDataSource<Types, I>
        + UpdateAvailabilityData<Types, I>
        + VersionedDataSource
        + Send
        + Sync
        + 'static,
{
    let mut blocks = Fetcher::new(fetcher.clone(), data_source.clone());
    let mut leaves = Fetcher::new(fetcher.clone(), data_source.clone());

    // Subscribe to errors encountered by the data source, handle the ones that can be handled by
    // fetching missing data.
    let mut errors = { data_source.read().await.subscribe_errors().await };
    while let Some(event) = errors.next().await {
        match event.context {
            Some(ErrorContext::MissingBlock(ctx)) => {
                blocks.fetch::<_, _, BlockQueryData<Types>>(ctx).await;
            }
            Some(ErrorContext::MissingLeaf(ctx)) => {
                leaves.fetch::<_, _, LeafQueryData<Types, I>>(ctx).await;
            }
            None => {
                // No context, nothing we can do about this error.
                tracing::debug!("fetcher task got an event we cannot handle: {event:?}");
            }
        }
    }
    // Usually the error stream is infinite, if it ends something unusual is going on.
    tracing::warn!("unexpected end of data source error stream, fetcher task will exit");
}
