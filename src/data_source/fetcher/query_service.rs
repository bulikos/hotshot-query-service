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

use super::{BlockRequest, Fetcher, LeafRequest};
use crate::{
    availability::{block_size, BlockQueryData, LeafQueryData, QueryableBlock},
    Block, Deltas, Error, QueryError, QueryErrorSnafu, QueryResult, Resolvable,
};
use async_trait::async_trait;
use commit::Committable;
use hotshot_types::traits::node_implementation::{NodeImplementation, NodeType};
use snafu::ensure;
use surf_disco::{Client, Url};

/// Data availability provider backed by another instance of this query service.
///
/// This fetcher implements the [`Fetcher`](super::Fetcher) protocol by querying the REST API
/// provided by another instance of this query service to try and retrieve missing objects.
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
impl<Types, I> Fetcher<Types, I, BlockQueryData<Types>> for QueryServiceFetcher
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
{
    async fn fetch(&self, req: &BlockRequest<Types>) -> QueryResult<BlockQueryData<Types>> {
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
impl<Types, I> Fetcher<Types, I, LeafQueryData<Types, I>> for QueryServiceFetcher
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    Deltas<Types, I>: Resolvable<Block<Types>>,
{
    async fn fetch(&self, req: &LeafRequest) -> QueryResult<LeafQueryData<Types, I>> {
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
