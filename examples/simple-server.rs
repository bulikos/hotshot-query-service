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

//! Simple HotShot query server
//!
//! This example demonstrates the most basic usage of hotshot-query-service. It starts a small
//! consensus network with two nodes and connects a query service to each node. It runs each query
//! server on local host. The program continues until it is manually killed.

use async_compatibility_layer::logging::{setup_backtrace, setup_logging};
use async_std::sync::Arc;
use clap::Parser;
use futures::future::{join_all, try_join_all};
use hotshot::{
    traits::{
        implementations::{MasterMap, MemoryCommChannel, MemoryNetwork, MemoryStorage},
        NodeImplementation,
    },
    types::{Message, SignatureKey, SystemContextHandle},
    HotShotInitializer, SystemContext,
};
use hotshot_query_service::{
    data_source, run_standalone_service,
    testing::mocks::{MockBlock, MockMembership, MockNodeImpl, MockTypes, TestableDataSource},
    Error,
};
use hotshot_signature_key::bn254::{BN254Priv, BN254Pub};
use hotshot_types::{
    traits::{
        election::Membership,
        node_implementation::{ExchangesType, NodeType},
    },
    ExecutionType, HotShotConfig,
};
use std::{num::NonZeroUsize, time::Duration};

const NUM_NODES: usize = 2;

#[derive(Parser)]
struct Options {
    /// Port on which to host the query service for the first consensus node.
    #[clap(long, default_value = "18080")]
    port1: u16,

    /// Port on which to host the query service for the second consensus node.
    #[clap(long, default_value = "28080")]
    port2: u16,
}

#[cfg(not(target_os = "windows"))]
type DataSource = data_source::SqlDataSource;

// To use SqlDataSource, we need to run the `postgres` Docker image, which doesn't work on Windows.
#[cfg(target_os = "windows")]
type DataSource = data_source::FileSystemDataSource<MockTypes, MockNodeImpl>;

type Db = <DataSource as TestableDataSource>::Storage;

#[cfg(not(target_os = "windows"))]
async fn init_db() -> Db {
    Db::init().await
}

#[cfg(target_os = "windows")]
async fn init_db() -> Db {
    Db::new("simple-server-db").unwrap()
}

#[cfg(not(target_os = "windows"))]
async fn init_data_source(db: &Db) -> DataSource {
    data_source::sql::Config::default()
        .user("postgres")
        .password("password")
        .port(db.port())
        .connect()
        .await
        .unwrap()
}

#[cfg(target_os = "windows")]
async fn init_data_source(db: &Db) -> DataSource {
    DataSource::create(db.path()).unwrap()
}

#[async_std::main]
async fn main() -> Result<(), Error> {
    setup_logging();
    setup_backtrace();

    let opt = Options::parse();

    // Start databases for the query services.
    let dbs = join_all((0..NUM_NODES).map(|_| init_db())).await;

    // Create the data sources for the query services.
    let data_sources = join_all(dbs.iter().map(init_data_source)).await;

    // Start consensus.
    let nodes = init_consensus(&data_sources).await;

    // Start the servers.
    try_join_all(
        data_sources
            .into_iter()
            .zip(nodes)
            .zip([opt.port1, opt.port2])
            .map(|((data_source, node), port)| async move {
                let opt = hotshot_query_service::Options {
                    port,
                    ..Default::default()
                };
                run_standalone_service(opt, data_source, node).await
            }),
    )
    .await?;

    Ok(())
}

async fn init_consensus(
    data_sources: &[DataSource],
) -> Vec<SystemContextHandle<MockTypes, MockNodeImpl>> {
    let priv_keys = (0..data_sources.len())
        .map(|_| BN254Priv::generate())
        .collect::<Vec<_>>();
    let pub_keys = priv_keys
        .iter()
        .map(BN254Pub::from_private)
        .collect::<Vec<_>>();
    let stake_table = pub_keys
        .iter()
        .map(|key| key.get_stake_table_entry(1u64))
        .collect::<Vec<_>>();
    let config = HotShotConfig {
        total_nodes: NonZeroUsize::new(stake_table.len()).unwrap(),
        known_nodes: pub_keys,
        start_delay: 0,
        round_start_delay: 0,
        next_view_timeout: 10000,
        timeout_ratio: (11, 10),
        propose_min_round_time: Duration::from_secs(0),
        propose_max_round_time: Duration::from_secs(2),
        min_transactions: 1,
        max_transactions: NonZeroUsize::new(100).unwrap(),
        num_bootstrap: 0,
        execution_type: ExecutionType::Continuous,
        election_config: None,
        da_committee_size: stake_table.len(),
        known_nodes_with_stake: stake_table,
    };
    let master_map = MasterMap::new();
    join_all(priv_keys.into_iter().zip(data_sources).enumerate().map(
        |(node_id, (priv_key, data_source))| {
            init_node(
                node_id,
                priv_key,
                data_source,
                config.clone(),
                master_map.clone(),
            )
        },
    ))
    .await
}

async fn init_node(
    node_id: usize,
    priv_key: BN254Priv,
    data_source: &impl TestableDataSource,
    config: HotShotConfig<
        BN254Pub,
        <BN254Pub as SignatureKey>::StakeTableEntry,
        <MockTypes as NodeType>::ElectionConfigType,
    >,
    master_map: Arc<MasterMap<Message<MockTypes, MockNodeImpl>, BN254Pub>>,
) -> SystemContextHandle<MockTypes, MockNodeImpl> {
    let pub_key = BN254Pub::from_private(&priv_key);
    let election_config = MockMembership::default_election_config(config.total_nodes.get() as u64);
    let network = Arc::new(MemoryNetwork::new(
        pub_key,
        data_source.populate_metrics(),
        master_map.clone(),
        None,
    ));
    let consensus_channel = MemoryCommChannel::new(network.clone());
    let da_channel = MemoryCommChannel::new(network.clone());
    let view_sync_channel = MemoryCommChannel::new(network.clone());

    let exchanges = <MockNodeImpl as NodeImplementation<MockTypes>>::Exchanges::create(
        config.known_nodes_with_stake.clone(),
        config.known_nodes.clone(),
        (election_config.clone(), election_config.clone()),
        (consensus_channel, view_sync_channel, da_channel),
        pub_key,
        pub_key.get_stake_table_entry(1u64),
        priv_key.clone(),
    );

    SystemContext::init(
        pub_key,
        priv_key,
        node_id as u64,
        config,
        MemoryStorage::empty(),
        exchanges,
        HotShotInitializer::from_genesis(MockBlock::genesis()).unwrap(),
        data_source.populate_metrics(),
    )
    .await
    .unwrap()
    .0
}
