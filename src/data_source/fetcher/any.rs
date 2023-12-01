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

use derivative::Derivative;

/// Adaptor combining multiple data availability fetchers.
///
/// This fetcher adaptor implements the [`Fetcher`](super::Fetcher) protocol by fetching requested
/// objects from several different underlying fetchers. If any of the underlying sources have the
/// object, the request will eventually succeed.
///
/// This can be used to combine multiple instances of the same kind of fetch, like using
/// [`QueryServiceFetcher`](super::QueryServiceFetcher) to request objects from a number of
/// different query services. It can also be used to search different kinds of data providers for
/// the same object, like searching for a block both in another instance of the query service and in
/// the HotShot DA committee. Finally, [`AnyFetcher`] can be used to combine a fetcher which only
/// fetches blocks and one which only fetches leaves into a fetcher which fetches both, and thus
/// implements [`AvailabilityFetcher`](super::AvailabilityFetcher).
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
/// # {
/// use hotshot_query_service::data_source::fetcher::{AnyFetcher, QueryServiceFetcher};
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
pub struct AnyFetcher;
