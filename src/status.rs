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

//! Queries for evolving and node-specific data.
//!
//! This API module defines a query API which complements the queries provided by the
//! [availability](crate::availability) module. While the availability module provides only data
//! which is immutable and which has global agreement across consensus, this module provides insight
//! into data and metrics which are constantly changing. It also surfaces information from the point
//! of view of a particular HotShot node, which other nodes may not agree on or even know about.

use crate::{api::load_api, QueryError};
use clap::Args;
use derive_more::From;
use futures::FutureExt;
use hotshot_types::traits::{
    node_implementation::{NodeImplementation, NodeType},
    signature_key::EncodedPublicKey,
};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::fmt::Display;
use std::path::PathBuf;
use tide_disco::{api::ApiError, method::ReadState, Api, RequestError, StatusCode};

pub(crate) mod data_source;
pub(crate) mod query_data;
pub use data_source::*;
pub use query_data::*;

#[derive(Args, Default)]
pub struct Options {
    #[arg(long = "status-api-path", env = "HOTSHOT_STATUS_API_PATH")]
    pub api_path: Option<PathBuf>,

    /// Additional API specification files to merge with `status-api-path`.
    ///
    /// These optional files may contain route definitions for application-specific routes that have
    /// been added as extensions to the basic status API.
    #[arg(
        long = "status-extension",
        env = "HOTSHOT_STATUS_EXTENSIONS",
        value_delimiter = ','
    )]
    pub extensions: Vec<toml::Value>,
}

#[derive(Clone, Debug, From, Snafu, Deserialize, Serialize)]
pub enum Error {
    Request {
        source: RequestError,
    },

    #[snafu(display("error fetching proposals by {proposer}: {source}"))]
    #[from(ignore)]
    QueryProposals {
        source: QueryError,
        proposer: EncodedPublicKey,
    },

    Internal {
        reason: String,
    },
}

impl Error {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Request { .. } => StatusCode::BadRequest,
            Self::QueryProposals { source, .. } => source.status(),
            Self::Internal { .. } => StatusCode::InternalServerError,
        }
    }
}

fn internal<M: Display>(msg: M) -> Error {
    Error::Internal {
        reason: msg.to_string(),
    }
}

pub fn define_api<State, Types, I>(options: &Options) -> Result<Api<State, Error>, ApiError>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    State: 'static + Send + Sync + ReadState,
    <State as ReadState>::State: Send + Sync + StatusDataSource<Types, I>,
{
    let mut api = load_api::<State, Error>(
        options.api_path.as_ref(),
        include_str!("../api/status.toml"),
        options.extensions.clone(),
    )?;
    api.with_version("0.0.1".parse().unwrap())
        .get("latest_block_height", |_, state| {
            async { state.block_height().await.map_err(internal) }.boxed()
        })?
        .get("count_proposals", |req, state| {
            async move {
                let proposer = req.blob_param("proposer_id")?;
                state
                    .count_proposals(&proposer)
                    .await
                    .context(QueryProposalsSnafu { proposer })
            }
            .boxed()
        })?
        .get("get_proposals", |req, state| {
            async move {
                let proposer = req.blob_param("proposer_id")?;
                let limit = req.opt_integer_param("count")?;
                state
                    .get_proposals(&proposer, limit)
                    .await
                    .context(QueryProposalsSnafu { proposer })
            }
            .boxed()
        })?
        .get("mempool_info", |_, state| {
            async { state.mempool_info().await.map_err(internal) }.boxed()
        })?
        .get("success_rate", |_, state| {
            async { state.success_rate().await.map_err(internal) }.boxed()
        })?
        .get("metrics", |_, state| {
            async { state.export_metrics().await.map_err(internal) }.boxed()
        })?;
    Ok(api)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        data_source::{ExtensibleDataSource, FileSystemDataSource},
        testing::{
            consensus::{MockDataSource, MockNetwork},
            mocks::{MockNodeImpl, MockTransaction, MockTypes},
            setup_test, sleep,
        },
        Error,
    };
    use async_std::{sync::RwLock, task::spawn};
    use bincode::Options as _;
    use futures::FutureExt;
    use hotshot_utils::bincode::bincode_opts;
    use portpicker::pick_unused_port;
    use std::time::Duration;
    use surf_disco::Client;
    use tempdir::TempDir;
    use tide_disco::App;
    use toml::toml;

    #[async_std::test]
    async fn test_api() {
        setup_test();

        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start the web server.
        let port = pick_unused_port().unwrap();
        let mut app = App::<_, Error>::with_state(network.data_source());
        app.register_module("status", define_api(&Default::default()).unwrap())
            .unwrap();
        spawn(app.serve(format!("0.0.0.0:{}", port)));

        // Start a client.
        let client =
            Client::<Error>::new(format!("http://localhost:{}/status", port).parse().unwrap());
        assert!(client.connect(Some(Duration::from_secs(60))).await);

        // Submit a transaction. We have not yet started the validators, so this transaction will
        // stay in the mempool, allowing us to check the mempool endpoint.
        let txn = MockTransaction { nonce: 0 };
        let txn_size = bincode_opts().serialized_size(&txn).unwrap();
        network.submit_transaction(txn.clone()).await;
        loop {
            let mempool = client
                .get::<MempoolQueryData>("mempool_info")
                .send()
                .await
                .unwrap();
            let expected = MempoolQueryData {
                transaction_count: 1,
                memory_footprint: txn_size,
            };
            if mempool == expected {
                break;
            }
            tracing::info!(
                "waiting for mempool to reflect transaction (currently {:?})",
                mempool
            );
            sleep(Duration::from_secs(1)).await;
        }
        assert_eq!(
            client
                .get::<u64>("latest_block_height")
                .send()
                .await
                .unwrap(),
            0
        );

        // Test Prometheus export.
        let prometheus = client.get::<String>("metrics").send().await.unwrap();
        let lines = prometheus.lines().collect::<Vec<_>>();
        assert!(
            lines.contains(&"consensus_outstanding_transactions 1"),
            "Missing consensus_outstanding_transactions in metrics:\n{}",
            prometheus
        );
        assert!(
            lines.contains(
                &format!(
                    "consensus_outstanding_transactions_memory_size {}",
                    txn_size
                )
                .as_str()
            ),
            "Missing consensus_outstanding_transactions_memory_size in metrics:\n{}",
            prometheus
        );

        // Start the validators and wait for the block to be finalized.
        network.start().await;
        while client
            .get::<MempoolQueryData>("mempool_info")
            .send()
            .await
            .unwrap()
            .transaction_count
            > 0
        {
            tracing::info!("waiting for transaction to be finalized");
            sleep(Duration::from_secs(1)).await;
        }

        // Check updated block height. There can be a brief delay between the mempool statistics
        // being updated and the decide event being published. Retry this a few times until it
        // succeeds.
        while client
            .get::<u64>("latest_block_height")
            .send()
            .await
            .unwrap()
            == 0
        {
            tracing::info!("waiting for block height to update");
            sleep(Duration::from_secs(1)).await;
        }
        let success_rate = client.get::<f64>("success_rate").send().await.unwrap();
        // If metrics are populating correctly, we should get a finite number. If not, we might get
        // NaN or infinity due to division by 0.
        // TODO re-enable this check once HotShot is populating view metrics again
        //      https://github.com/EspressoSystems/HotShot/issues/2066
        // assert!(success_rate.is_finite(), "{success_rate}");
        // We know at least some views have been successful, since we finalized a block.
        assert!(success_rate > 0.0, "{success_rate}");

        network.shut_down().await;
    }

    #[async_std::test]
    async fn test_extensions() {
        setup_test();

        let dir = TempDir::new("test_status_extensions").unwrap();
        let data_source = ExtensibleDataSource::new(
            FileSystemDataSource::<MockTypes, MockNodeImpl>::create(dir.path()).unwrap(),
            0,
        );

        let extensions = toml! {
            [route.post_ext]
            PATH = ["/ext/:val"]
            METHOD = "POST"
            ":val" = "Integer"

            [route.get_ext]
            PATH = ["/ext"]
            METHOD = "GET"
        };

        let mut api = define_api::<
            RwLock<ExtensibleDataSource<FileSystemDataSource<MockTypes, MockNodeImpl>, u64>>,
            MockTypes,
            MockNodeImpl,
        >(&Options {
            extensions: vec![extensions.into()],
            ..Default::default()
        })
        .unwrap();
        api.get("get_ext", |_, state| {
            async move { Ok(*state.as_ref()) }.boxed()
        })
        .unwrap()
        .post("post_ext", |req, state| {
            async move {
                *state.as_mut() = req.integer_param("val")?;
                Ok(())
            }
            .boxed()
        })
        .unwrap();

        let mut app = App::<_, Error>::with_state(RwLock::new(data_source));
        app.register_module("status", api).unwrap();

        let port = pick_unused_port().unwrap();
        spawn(app.serve(format!("0.0.0.0:{}", port)));

        let client =
            Client::<Error>::new(format!("http://localhost:{}/status", port).parse().unwrap());
        assert!(client.connect(Some(Duration::from_secs(60))).await);

        assert_eq!(client.get::<u64>("ext").send().await.unwrap(), 0);
        client.post::<()>("ext/42").send().await.unwrap();
        assert_eq!(client.get::<u64>("ext").send().await.unwrap(), 42);

        // Ensure we can still access the built-in functionality.
        assert_eq!(
            client
                .get::<u64>("latest_block_height")
                .send()
                .await
                .unwrap(),
            0
        );
    }
}
