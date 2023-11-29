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

use super::{Fetch, Fetchable};
use crate::{
    availability::{
        BlockQueryData, LeafQueryData, MissingBlockContext, MissingLeafContext, QueryableBlock,
    },
    Block, Deltas, QueryError, QueryResult, Resolvable,
};
use async_std::sync::Arc;
use async_trait::async_trait;
use derivative::Derivative;
use hotshot_types::traits::node_implementation::{NodeImplementation, NodeType};
use std::fmt::Debug;

/// Blanket trait combining [`Debug`] and [`Fetch`].
///
/// This is necessary to create a fetcher trait object (`dyn Fetch`, see [`BlockFetcher`] and
/// [`LeafFetcher`]) which also implements [`Debug`], since trait objects can only have one non-auto
/// trait bound.
trait DebugFetch<Types, I, T>: Fetch<Types, I, T> + Debug
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    T: Fetchable<Types, I>,
{
}

impl<Types, I, T, F> DebugFetch<Types, I, T> for F
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    T: Fetchable<Types, I>,
    F: Fetch<Types, I, T> + Debug,
{
}

type BlockFetcher<Types, I> = Arc<dyn DebugFetch<Types, I, BlockQueryData<Types>>>;
type LeafFetcher<Types, I> = Arc<dyn DebugFetch<Types, I, LeafQueryData<Types, I>>>;

/// Adaptor combining multiple data availability fetchers.
///
/// This fetcher adaptor implements the [`Fetch`](super::Fetch) protocol by fetching requested
/// objects from several different underlying fetchers. If any of the underlying sources have the
/// object, the request will eventually succeed.
///
/// This can be used to combine multiple instances of the same kind of fetch, like using
/// [`QueryServiceFetcher`](super::QueryServiceFetcher) to request objects from a number of
/// different query services. It can also be used to search different kinds of data providers for
/// the same object, like searching for a block both in another instance of the query service and in
/// the HotShot DA committee. Finally, [`AnyFetcher`] can be used to combine a fetcher which only
/// fetches blocks and one which only fetches leaves into a fetcher which fetches both, and which
/// can thus be used with [`fetcher::run`](super::run).
///
/// # Examples
///
/// Fetching from multiple query services, for resiliency.
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
/// use hotshot_query_service::fetcher::{AnyFetcher, QueryServiceFetcher};
///
/// let qs1 = QueryServiceFetcher::new("https://backup.query-service.1".parse()?).await;
/// let qs2 = QueryServiceFetcher::new("https://backup.query-service.2".parse()?).await;
/// let fetcher = AnyFetcher::<Types, I>::default()
///     .with_fetcher(qs1)
///     .with_fetcher(qs2);
/// # Ok(())
/// # }
/// ```
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Debug(bound = ""), Default(bound = ""))]
pub struct AnyFetcher<Types, I>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
{
    block_fetchers: Vec<BlockFetcher<Types, I>>,
    leaf_fetchers: Vec<LeafFetcher<Types, I>>,
}

#[async_trait]
impl<Types, I> Fetch<Types, I, BlockQueryData<Types>> for AnyFetcher<Types, I>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
{
    async fn fetch(&self, req: &MissingBlockContext<Types>) -> QueryResult<BlockQueryData<Types>> {
        any_fetch(&self.block_fetchers, req).await
    }
}

#[async_trait]
impl<Types, I> Fetch<Types, I, LeafQueryData<Types, I>> for AnyFetcher<Types, I>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    Deltas<Types, I>: Resolvable<Block<Types>>,
{
    async fn fetch(&self, req: &MissingLeafContext) -> QueryResult<LeafQueryData<Types, I>> {
        any_fetch(&self.leaf_fetchers, req).await
    }
}

impl<Types, I> AnyFetcher<Types, I>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    Deltas<Types, I>: Resolvable<Block<Types>>,
{
    /// Add a sub-fetcher which fetches both blocks and leaves.
    pub fn with_fetcher<F>(mut self, fetcher: F) -> Self
    where
        F: Fetch<Types, I, BlockQueryData<Types>>
            + Fetch<Types, I, LeafQueryData<Types, I>>
            + Debug,
    {
        let fetcher = Arc::new(fetcher);
        self.block_fetchers.push(fetcher.clone());
        self.leaf_fetchers.push(fetcher);
        self
    }

    /// Add a sub-fetcher which fetches blocks.
    pub fn with_block_fetcher<F>(mut self, fetcher: F) -> Self
    where
        F: Fetch<Types, I, BlockQueryData<Types>> + Debug,
    {
        self.block_fetchers.push(Arc::new(fetcher));
        self
    }

    /// Add a sub-fetcher which fetches leaves.
    pub fn with_leaf_fetcher<F>(mut self, fetcher: F) -> Self
    where
        F: Fetch<Types, I, LeafQueryData<Types, I>> + Debug,
    {
        self.leaf_fetchers.push(Arc::new(fetcher));
        self
    }
}

async fn any_fetch<Types, I, F, T>(fetchers: &[Arc<F>], req: &T::Request) -> QueryResult<T>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    T: Fetchable<Types, I>,
    F: Fetch<Types, I, T> + ?Sized,
{
    // There's a policy question of how to decide when to try each fetcher: all in parallel, in
    // serial, or a combination. For now, we do the simplest thing of trying each in order, in
    // serial. This has the best performance in the common case when we succeed on the first
    // fetcher: low latency, and no undue burden on the other fetchers. However, a more complicated
    // strategy where we slowly ramp up the parallelism as more and more requests fail may provide
    // better worst-case latency.
    for (i, f) in fetchers.iter().enumerate() {
        match f.fetch(req).await {
            Ok(obj) => return Ok(obj),
            Err(err) => {
                tracing::warn!(
                    "error fetching request {req:?} from fetcher {i}/{}: {err}",
                    fetchers.len()
                );
                continue;
            }
        }
    }

    // If all fetchers failed, we already logged the specific errors from each one. It suffices to
    // just return a generic error.
    Err(QueryError::NotFound)
}
