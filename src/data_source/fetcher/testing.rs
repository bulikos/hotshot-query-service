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

#![cfg(any(test, feature = "testing"))]

use super::{BlockRequest, Fetchable, Fetcher, LeafRequest};
use crate::{
    availability::{BlockQueryData, LeafQueryData, QueryableBlock},
    Block, Deltas, QueryResult, Resolvable,
};
use async_compatibility_layer::async_primitives::broadcast::{channel, BroadcastSender};
use async_std::sync::{Arc, RwLock};
use async_trait::async_trait;
use derivative::Derivative;
use hotshot_types::traits::node_implementation::{NodeImplementation, NodeType};
use std::fmt::Debug;

/// Adaptor to add test-only functionality to an existing fetcher.
///
/// [`TestFetcher`] wraps an existing fetcher `F` and adds some additional functionality which can
/// be useful in tests, such as the ability to inject delays into the handling of fetch requests.
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Debug(bound = "F: Debug"))]
pub struct TestFetcher<F> {
    inner: Arc<F>,
    unblock: Arc<RwLock<Option<BroadcastSender<()>>>>,
}

impl<F> TestFetcher<F> {
    pub fn new(inner: F) -> Self {
        Self {
            inner: Arc::new(inner),
            unblock: Default::default(),
        }
    }

    /// Delay fetch requests until [`unblock`](Self::unblock).
    ///
    /// Fetch requests started after this method returns will block without completing until
    /// [`unblock`](Self::unblock) is called. This can be useful for tests to examine the state of a
    /// data source _before_ a fetch request completes, to check that the subsequent fetch actually
    /// has an effect.
    pub async fn block(&self) {
        let mut unblock = self.unblock.write().await;
        if unblock.is_none() {
            *unblock = Some(channel().0);
        }
    }

    /// Allow blocked fetch requests to proceed.
    ///
    /// Fetch requests which are blocked as a result of a preceding call to [`block`](Self::block)
    /// will become unblocked.
    pub async fn unblock(&self) {
        let mut unblock = self.unblock.write().await;
        if let Some(unblock) = unblock.take() {
            unblock.send_async(()).await.ok();
        }
    }
}

#[async_trait]
impl<Types, I, F> Fetcher<Types, I, BlockQueryData<Types>> for TestFetcher<F>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    F: Fetcher<Types, I, BlockQueryData<Types>>,
{
    async fn fetch(&self, req: &BlockRequest<Types>) -> QueryResult<BlockQueryData<Types>> {
        fetch(self, req).await
    }
}

#[async_trait]
impl<Types, I, F> Fetcher<Types, I, LeafQueryData<Types, I>> for TestFetcher<F>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    Deltas<Types, I>: Resolvable<Block<Types>>,
    F: Fetcher<Types, I, LeafQueryData<Types, I>>,
{
    async fn fetch(&self, req: &LeafRequest) -> QueryResult<LeafQueryData<Types, I>> {
        fetch(self, req).await
    }
}

async fn fetch<Types, I, F, T>(fetcher: &TestFetcher<F>, req: &T::Request) -> QueryResult<T>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    T: Fetchable<Types, I>,
    F: Fetcher<Types, I, T>,
{
    // Block the request if the user has called `block`.
    let handle = {
        match fetcher.unblock.read().await.as_ref() {
            Some(unblock) => Some(unblock.handle_async().await),
            None => None,
        }
    };
    if let Some(mut handle) = handle {
        tracing::info!("request for {req:?} will block until manually unblocked");
        handle.recv_async().await.ok();
        tracing::info!("request for {req:?} unblocked");
    }

    // Do the request.
    fetcher.inner.fetch(req).await
}
