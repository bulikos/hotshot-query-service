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

use super::Fetch;
use crate::{
    availability::{
        block_size, BlockQueryData, LeafQueryData, MissingBlockContext, MissingLeafContext,
        QueryableBlock,
    },
    Block, Deltas, Error, QueryError, QueryErrorSnafu, QueryResult, Resolvable,
};
use async_trait::async_trait;
use commit::Committable;
use hotshot_types::traits::node_implementation::{NodeImplementation, NodeType};
use snafu::ensure;
use surf_disco::{Client, Url};

/// Data availability provider backed by another instance of this query service.
///
/// This fetcher implements the [`Fetch`](super::Fetch) protocol by querying the REST API provided
/// by another instance of this query service to try and retrieve missing objects.
#[derive(Clone, Debug)]
pub struct QueryServiceFetcher {
    client: Client<Error>,
}

impl QueryServiceFetcher {
    pub async fn new(url: Url) -> Self {
        let client = Client::new(url);
        client.connect(None).await;
        Self { client }
    }
}

#[async_trait]
impl<Types, I> Fetch<Types, I, BlockQueryData<Types>> for QueryServiceFetcher
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
{
    async fn fetch(&self, req: &MissingBlockContext<Types>) -> QueryResult<BlockQueryData<Types>> {
        let res: BlockQueryData<Types> = self
            .client
            .get(&format!("availability/block/hash/{}", req.hash))
            .send()
            .await
            .map_err(server_error)?;
        // We only care about the block itself. The rest of the information in the `BlockQueryData`
        // is already known to us via `req`, which originated in this node and is thus more
        // trustworthy than the data in `res` anyways.
        let block = res.block;

        // Verify that the data we retrieved is consistent with the request we made.
        ensure!(
            block.commit() == req.hash,
            QueryErrorSnafu {
                message: format!(
                    "retrieved block commitment {} does not match expected hash {}",
                    block.commit(),
                    req.hash
                ),
            }
        );
        let size = block_size::<Types>(&block);

        Ok(BlockQueryData {
            block,
            size,
            hash: req.hash,
            height: req.height,
            timestamp: req.timestamp,
        })
    }
}

#[async_trait]
impl<Types, I> Fetch<Types, I, LeafQueryData<Types, I>> for QueryServiceFetcher
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    Deltas<Types, I>: Resolvable<Block<Types>>,
{
    async fn fetch(&self, req: &MissingLeafContext) -> QueryResult<LeafQueryData<Types, I>> {
        let leaf: LeafQueryData<Types, I> = self
            .client
            .get(&format!("availability/leaf/{}", req.height))
            .send()
            .await
            .map_err(server_error)?;

        // TODO we should also download a chain of QCs justifying the inclusion of `leaf` in the
        // chain at the requested height. However, HotShot currently lacks a good light client API
        // to verify this chain, so for now we just trust the other server.
        // https://github.com/EspressoSystems/HotShot/issues/2137

        Ok(leaf)
    }
}

fn server_error(err: Error) -> QueryError {
    QueryError::Error {
        message: err.to_string(),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        availability::{define_api, leaf_height, AvailabilityDataSource},
        fetcher::{run, testing::TestFetcher},
        testing::{
            consensus::{MockDataSource, MockNetwork, MINIMUM_NODES},
            setup_test, sleep,
        },
    };
    use async_std::{sync::Arc, task::spawn};
    use futures::stream::StreamExt;
    use hotshot::types::EventType;
    use portpicker::pick_unused_port;
    use std::time::Duration;
    use tide_disco::App;

    #[async_std::test]
    async fn test_fetch_on_decide() {
        setup_test();

        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-DA node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
        let mut app = App::<_, Error>::with_state(network.data_source());
        app.register_module("availability", define_api(&Default::default()).unwrap())
            .unwrap();
        spawn(app.serve(format!("0.0.0.0:{port}")));

        // Attach a fetcher to the non-DA node.
        let data_source = network.non_da_data_source();
        let fetcher = Arc::new(TestFetcher::new(
            QueryServiceFetcher::new(format!("http://localhost:{port}").parse().unwrap()).await,
        ));
        spawn(run(fetcher.clone(), data_source.clone()));

        // Block requests to the fetcher so that we can verify that without the fetcher, the non-DA
        // node does _not_ get block data from consensus.
        fetcher.block().await;

        // Start consensus.
        network.start().await;

        // Wait for there to be a decide.
        let mut leaves = { data_source.read().await.subscribe_leaves(0).await.unwrap() };
        leaves.next().await;

        // Give the block even some extra time to propagate, and check that we still can't get it
        // from the non-DA node, since the fetcher is blocked. This just ensures the integrity of
        // the test by checking the node didn't mysteriously get the block from somewhere else, so
        // that when we unblock the fetcher and the node finally gets the block, we know it came
        // from the fetcher.
        sleep(Duration::from_secs(1)).await;
        let err = { data_source.read().await.get_block(0).await.unwrap_err() };
        assert!(matches!(err, QueryError::Missing), "{err}");

        // Unblock the request and see that we eventually receive the block in the non-DA node.
        fetcher.unblock().await;
        let mut blocks = { data_source.read().await.subscribe_blocks(0).await.unwrap() };
        let block = blocks.next().await.unwrap();
        assert_eq!(block, {
            data_source.read().await.get_block(0).await.unwrap()
        });
    }

    #[async_std::test]
    async fn test_catchup() {
        setup_test();

        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-DA node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
        let mut app = App::<_, Error>::with_state(network.data_source());
        app.register_module("availability", define_api(&Default::default()).unwrap())
            .unwrap();
        spawn(app.serve(format!("0.0.0.0:{port}")));

        // Start all but the non-DA node.
        network.start_nodes(..MINIMUM_NODES).await;

        // Wait for there to be a decide.
        let mut leaves = {
            network
                .data_source()
                .read()
                .await
                .subscribe_leaves(0)
                .await
                .unwrap()
        };
        leaves.next().await;

        // Attach a fetcher to the non-DA node. Block requests so that we can verify that without
        // the fetcher, the non-DA node does _not_ get block data from consensus.
        let data_source = network.non_da_data_source();
        let fetcher = Arc::new(TestFetcher::new(
            QueryServiceFetcher::new(format!("http://localhost:{port}").parse().unwrap()).await,
        ));
        fetcher.block().await;
        spawn(run(fetcher.clone(), data_source.clone()));

        // Start consensus on the non-DA node and wait for it to catch up and start generating
        // decides.
        let mut events = network
            .non_da_handle()
            .get_event_stream(Default::default())
            .await
            .0;
        network.start_nodes(MINIMUM_NODES..).await;
        let block_height = loop {
            let event = events.next().await.unwrap();
            if let EventType::Decide { leaf_chain, .. } = event.event {
                break leaf_height(&leaf_chain[0]);
            }
        };

        // Without the fetcher, we missed the first decide event.
        let err = { data_source.read().await.get_block(0).await.unwrap_err() };
        assert!(matches!(err, QueryError::Missing), "{err}");
        let err = { data_source.read().await.get_leaf(0).await.unwrap_err() };
        assert!(matches!(err, QueryError::Missing), "{err}");

        // Unblock the fetcher, then we should eventually be able to get all the blocks we missed.
        fetcher.unblock().await;

        let data_source = data_source.read().await;
        let mut blocks = data_source.subscribe_blocks(0).await.unwrap();
        let mut leaves = data_source.subscribe_leaves(0).await.unwrap();
        for i in 0..=block_height {
            let block = blocks.next().await.unwrap();
            let leaf = leaves.next().await.unwrap();
            assert_eq!(block.height(), i);
            assert_eq!(leaf.height(), i);
            assert_eq!(block, data_source.get_block(i as usize).await.unwrap());
            assert_eq!(leaf, data_source.get_leaf(i as usize).await.unwrap());
        }
    }
}
