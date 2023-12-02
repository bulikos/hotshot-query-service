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

//! Persistent storage and sources of data consumed by APIs.
//!
//! The APIs provided by this query service are generic over the implementation which actually
//! retrieves data in answer to queries. We call this implementation a _data source_. This module
//! defines a data source and provides several pre-built implementations:
//! * [`FileSystemDataSource`]
//! * [`SqlDataSource`]
//! * [`DataSource`], a generalization of the above
//!
//! We also provide combinators for modularly adding functionality to existing data sources:
//! * [`ExtensibleDataSource`]
//!

use crate::{
    availability::{AvailabilityDataSource, QueryableBlock},
    Block,
};
use async_trait::async_trait;
use derivative::Derivative;
use fetcher::AvailabilityFetcher;
use hotshot_types::traits::node_implementation::{NodeImplementation, NodeType};
use std::fmt::Debug;
use storage::AvailabilityStorage;

mod extension;
pub mod fetcher;
mod fs;
mod ledger_log;
pub mod sql;
pub mod storage;
mod update;

pub use extension::ExtensibleDataSource;
#[cfg(feature = "file-system-data-source")]
pub use fs::FileSystemDataSource;
#[cfg(feature = "sql-data-source")]
pub use sql::SqlDataSource;
pub use update::{UpdateDataSource, VersionedDataSource};

/// The most basic kind of data source.
///
/// A data source is constructed modularly by combining a [storage] implementation with a [fetcher].
/// The former allows the query service to store the data it has persistently in an easily
/// accessible storage medium, such as the local file system or a database. This allows it to answer
/// queries efficiently and to maintain its state across restarts. The latter allows the query
/// service to fetch data that is missing from its storage from an external data availability
/// provider, such as the Tiramisu DA network or another instance of the query service.
///
/// These two components of a data source are combined in [`DataSource`], which is the lowest level
/// kind of data source available. It simply uses the storage implementation to fetch data when
/// available, and fills in everything else using the fetcher. Various kinds of data sources can be
/// constructed out of [`DataSource`] by changing the storage and fetcher implementations used, and
/// more complex data sources can be built on top using data source combinators.
#[derive(Derivative)]
#[derivative(
    Clone(bound = "Storage: Clone"),
    Debug(bound = "Storage: Debug, Fetcher: Debug")
)]
pub struct DataSource<Storage, Fetcher> {
    storage: Storage,
    fetcher: Arc<Fetcher>,
}

impl<Storage, Fetcher> DataSource<Storage, Fetcher> {
    pub fn new(storage: Storage, fetcher: Fetcher) -> Self {
        Self {
            storage,
            fetcher: Arc::new(fetcher),
        }
    }
}

#[async_trait]
impl<Types, I, Storage> AvailabilityDataSource<Types, I> for DataSource<Storage, Fetcher>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    Block<Types>: QueryableBlock,
    Storage: AvailabilityStorage<Types, I>,
    Fetcher: AvailabilityFetcher<Types, I>,
{
    type LeafRange<R> = FetchStream<Storage::LeafRange<R>, Fetcher>;
    type BlockRange<R>: Stream<Item = Fetch<BlockQueryData<Types>>> + Unpin + Send + 'static
    where
        R: RangeBounds<usize> + Send;

    async fn get_leaf<ID>(&self, id: ID) -> Fetch<LeafQueryData<Types, I>>
    where
        ID: Into<LeafId<Types, I>> + Send + Sync;
    async fn get_block<ID>(&self, id: ID) -> Fetch<BlockQueryData<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync;

    async fn get_leaf_range<R>(&self, range: R) -> Self::LeafRange<R>
    where
        R: RangeBounds<usize> + Send + 'static;
    async fn get_block_range<R>(&self, range: R) -> Self::BlockRange<R>
    where
        R: RangeBounds<usize> + Send + 'static;

    /// Returns the block containing a transaction with the given `hash` and the transaction's
    /// position in the block.
    async fn get_block_with_transaction(
        &self,
        hash: TransactionHash<Types>,
    ) -> Fetch<(BlockQueryData<Types>, TransactionIndex<Types>)>;

    async fn subscribe_blocks(&self, from: usize) -> BoxStream<'static, BlockQueryData<Types>> {
        self.get_block_range(from..)
            .await
            .then(Fetch::resolve)
            .boxed()
    }

    async fn subscribe_leaves(&self, from: usize) -> BoxStream<'static, LeafQueryData<Types, I>> {
        self.get_leaf_range(from..)
            .await
            .then(Fetch::resolve)
            .boxed()
    }
}

/// Generic tests we can instantiate for all the data sources.
#[cfg(any(test, feature = "testing"))]
#[espresso_macros::generic_tests]
pub mod data_source_tests {
    use crate::{
        availability::{block_size, BlockQueryData, Fetch, LeafQueryData},
        status::MempoolQueryData,
        testing::{
            consensus::MockNetwork,
            mocks::{
                MockBlock, MockNodeImpl, MockState, MockTransaction, MockTypes, TestableDataSource,
            },
            setup_test, sleep,
        },
        Leaf, QuorumCertificate,
    };
    use async_std::sync::RwLock;
    use bincode::Options;
    use commit::Committable;
    use futures::StreamExt;
    use hotshot_types::{
        data::{LeafType, ViewNumber},
        traits::{election::SignedCertificate, state::ConsensusTime, Block, State},
    };
    use hotshot_utils::bincode::bincode_opts;
    use std::collections::{HashMap, HashSet};
    use std::ops::{Bound, RangeBounds};
    use std::time::Duration;

    async fn get_non_empty_blocks(
        ds: &RwLock<impl TestableDataSource>,
    ) -> Vec<(
        LeafQueryData<MockTypes, MockNodeImpl>,
        BlockQueryData<MockTypes>,
    )> {
        let ds = ds.read().await;
        ds.get_leaf_range(..)
            .await
            .zip(ds.get_block_range(..).await)
            .filter_map(|entry| async move {
                match entry {
                    (Fetch::Ready(leaf), Fetch::Ready(block)) if !block.is_empty() => {
                        Some((leaf, block))
                    }
                    _ => None,
                }
            })
            .collect()
            .await
    }

    async fn validate(ds: &RwLock<impl TestableDataSource>) {
        let ds = ds.read().await;

        // Check the consistency of every block/leaf pair. Keep track of blocks and transactions
        // we've seen so we can detect duplicates.
        // TODO eliminate duplicate blocks:
        // https://github.com/EspressoSystems/hotshot-query-service/issues/284
        let mut seen_blocks = HashMap::new();
        let mut seen_transactions = HashMap::new();
        let mut leaves = ds.get_leaf_range(..).await.enumerate();
        while let Some((i, leaf)) = leaves.next().await {
            let leaf = leaf.await;
            assert_eq!(leaf.height(), i as u64);
            assert_eq!(leaf.hash(), leaf.leaf().commit());

            // Check indices.
            assert_eq!(leaf, ds.get_leaf(i).await.await);
            assert_eq!(leaf, ds.get_leaf(leaf.hash()).await.await);
            assert!(ds
                .get_proposals(&leaf.proposer(), None)
                .await
                .unwrap()
                .contains(&leaf));

            let Ok(block) = ds.get_block(i).await.try_resolve() else {
                continue;
            };
            assert_eq!(leaf.block_hash(), block.hash());
            assert_eq!(block.height(), i as u64);
            assert_eq!(block.hash(), block.block().commit());
            assert_eq!(block.size(), block_size::<MockTypes>(block.block()));

            // Check indices.
            assert_eq!(block, ds.get_block(i).await.await);
            // We should be able to look up the block by hash unless it is a duplicate. For
            // duplicate blocks, this function returns the index of the first duplicate.
            let ix = seen_blocks.entry(block.hash()).or_insert(i as u64);
            assert_eq!(ds.get_block(block.hash()).await.await.height(), *ix);

            for (j, txn) in block.block().iter().enumerate() {
                // We should be able to look up the transaction by hash unless it is a duplicate.
                // For duplicate transactions, this function returns the index of the first
                // duplicate.
                let ix = seen_transactions
                    .entry(txn.commit())
                    .or_insert((i as u64, j));
                let (block, pos) = ds.get_block_with_transaction(txn.commit()).await.await;
                assert_eq!((block.height(), pos), *ix);
            }
        }

        // Validate the list of proposals for every distinct proposer ID in the chain.
        for proposer in ds
            .get_leaf_range(..)
            .await
            .then(|leaf| async move { leaf.await.proposer() })
            .collect::<HashSet<_>>()
            .await
        {
            let proposals = ds.get_proposals(&proposer, None).await.unwrap();
            // We found `proposer` by getting the proposer ID of a leaf, so there must be at least
            // one proposal from this proposer.
            assert!(!proposals.is_empty());
            // If we select with a limit, we should get the most recent `limit` proposals in
            // chronological order.
            let suffix = ds
                .get_proposals(&proposer, Some(proposals.len() / 2))
                .await
                .unwrap();
            assert_eq!(suffix.len(), proposals.len() / 2);
            assert!(proposals.ends_with(&suffix));

            // Check that the proposer ID of every leaf indexed by `proposer` is `proposer`, and
            // that the list of proposals is in chronological order.
            let mut prev_height = None;
            for leaf in proposals {
                assert_eq!(proposer, leaf.proposer());
                if let Some(prev_height) = prev_height {
                    assert!(prev_height < leaf.height());
                }
                prev_height = Some(leaf.height());
            }
        }
    }

    #[async_std::test]
    pub async fn test_update<D: TestableDataSource>() {
        setup_test();

        let mut network = MockNetwork::<D>::init().await;
        let ds = network.data_source();

        network.start().await;
        assert_eq!(get_non_empty_blocks(&ds).await, vec![]);

        // Submit a few blocks and make sure each one gets reflected in the query service and
        // preserves the consistency of the data and indices.
        let mut blocks = { ds.read().await.subscribe_blocks(0).await.enumerate() };
        for nonce in 0..3 {
            let txn = MockTransaction { nonce };
            network.submit_transaction(txn).await;

            // Wait for the transaction to be finalized.
            let (i, block) = loop {
                tracing::info!("waiting for block {nonce}");
                let (i, block) = blocks.next().await.unwrap();
                if !block.is_empty() {
                    break (i, block);
                }
                tracing::info!("block {i} is empty");
            };

            assert_eq!(ds.read().await.get_block(i).await.await, block);
            validate(&ds).await;
        }

        // Check that all the updates have been committed to storage, not simply held in memory: we
        // should be able to read the same data if we connect an entirely new data source to the
        // underlying storage.
        {
            // Lock the original data source to prevent concurrent updates.
            let ds = ds.read().await;
            let storage = D::connect(network.storage()).await;
            assert_eq!(
                ds.get_block_range(..)
                    .await
                    .then(Fetch::resolve)
                    .collect::<Vec<_>>()
                    .await,
                storage
                    .get_block_range(..)
                    .await
                    .then(Fetch::resolve)
                    .collect::<Vec<_>>()
                    .await
            );
            assert_eq!(
                ds.get_leaf_range(..)
                    .await
                    .then(Fetch::resolve)
                    .collect::<Vec<_>>()
                    .await,
                storage
                    .get_leaf_range(..)
                    .await
                    .then(Fetch::resolve)
                    .collect::<Vec<_>>()
                    .await
            );
        }

        network.shut_down().await;
    }

    #[async_std::test]
    pub async fn test_metrics<D: TestableDataSource>() {
        setup_test();

        let mut network = MockNetwork::<D>::init().await;
        let ds = network.data_source();

        {
            // With consensus paused, check that the success rate returns NaN (since the block
            // height, the numerator, and view number, the denominator, are both 0).
            assert!(ds.read().await.success_rate().await.unwrap().is_nan());
            // Check that block height is initially zero.
            assert_eq!(ds.read().await.block_height().await.unwrap(), 0);
        }

        // Submit a transaction, and check that it is reflected in the mempool.
        let txn = MockTransaction { nonce: 0 };
        network.submit_transaction(txn.clone()).await;
        loop {
            let mempool = { ds.read().await.mempool_info().await.unwrap() };
            let expected = MempoolQueryData {
                transaction_count: 1,
                memory_footprint: bincode_opts().serialized_size(&txn).unwrap(),
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
        {
            assert_eq!(
                ds.read().await.mempool_info().await.unwrap(),
                MempoolQueryData {
                    transaction_count: 1,
                    memory_footprint: bincode_opts().serialized_size(&txn).unwrap(),
                }
            );
        }

        // Submitting the same transaction should not affect the mempool.
        network.submit_transaction(txn.clone()).await;
        sleep(Duration::from_secs(3)).await;
        {
            assert_eq!(
                ds.read().await.mempool_info().await.unwrap(),
                MempoolQueryData {
                    transaction_count: 1,
                    memory_footprint: bincode_opts().serialized_size(&txn).unwrap(),
                }
            );
        }

        // Start consensus and wait for the transaction to be finalized.
        let mut blocks = { ds.read().await.subscribe_blocks(0).await };
        network.start().await;
        loop {
            if !blocks.next().await.unwrap().is_empty() {
                break;
            }
        }

        {
            // Check that block height and success rate have been updated. Note that we can only
            // check if success rate is positive. We don't know exactly what it is because we can't
            // know how many views have elapsed without race conditions. Similarly, we can only
            // check that block height is at least 2, because we know that the genesis block and our
            // transaction's block have both been committed, but we can't know how many empty blocks
            // were committed.
            let success_rate = ds.read().await.success_rate().await.unwrap();
            // TODO re-enable this check once HotShot is populating view metrics again
            //      https://github.com/EspressoSystems/HotShot/issues/2066
            // assert!(success_rate.is_finite(), "{success_rate}");
            assert!(success_rate > 0.0, "{success_rate}");
            assert!(ds.read().await.block_height().await.unwrap() >= 2);
        }

        {
            // Check that the transaction is no longer reflected in the mempool.
            assert_eq!(
                ds.read().await.mempool_info().await.unwrap(),
                MempoolQueryData {
                    transaction_count: 0,
                    memory_footprint: 0,
                }
            );
        }
    }

    #[async_std::test]
    pub async fn test_range<D: TestableDataSource>() {
        setup_test();

        let mut network = MockNetwork::<D>::init().await;
        let ds = network.data_source();
        network.start().await;

        // Wait for there to be at least 3 blocks, then lock the state so the block height can't
        // change during the test.
        let (ds, block_height) = loop {
            let ds = ds.read().await;
            let block_height = ds.block_height().await.unwrap();
            if block_height >= 3 {
                break (ds, block_height as u64);
            }
        };

        // Query for a variety of ranges testing all cases of included, excluded, and unbounded
        // starting and ending bounds
        do_range_test(&*ds, 1..=2, 1..3).await; // (inclusive, inclusive)
        do_range_test(&*ds, 1..3, 1..3).await; // (inclusive, exclusive)
        do_range_test(&*ds, 1.., 1..block_height).await; // (inclusive, unbounded)
        do_range_test(&*ds, ..=2, 0..3).await; // (unbounded, inclusive)
        do_range_test(&*ds, ..3, 0..3).await; // (unbounded, exclusive)
        do_range_test(&*ds, .., 0..block_height).await; // (unbounded, unbounded)
        do_range_test(&*ds, ExRange(0..=2), 1..3).await; // (exclusive, inclusive)
        do_range_test(&*ds, ExRange(0..3), 1..3).await; // (exclusive, exclusive)
        do_range_test(&*ds, ExRange(0..), 1..block_height).await; // (exclusive, unbounded)
    }

    async fn do_range_test<D, R, I>(ds: &D, range: R, expected_indices: I)
    where
        D: TestableDataSource,
        R: RangeBounds<usize> + Clone + Send + 'static,
        I: IntoIterator<Item = u64>,
    {
        let mut leaves = ds.get_leaf_range(range.clone()).await;
        let mut blocks = ds.get_block_range(range).await;

        for i in expected_indices {
            let leaf = leaves.next().await.unwrap().await;
            let block = blocks.next().await.unwrap().await;
            assert_eq!(leaf.height(), i);
            assert_eq!(block.height(), i);
        }
    }

    // A wrapper around a range that turns the lower bound from inclusive to exclusive.
    #[derive(Clone, Copy, Debug)]
    struct ExRange<R>(R);

    impl<R: RangeBounds<usize>> RangeBounds<usize> for ExRange<R> {
        fn start_bound(&self) -> Bound<&usize> {
            match self.0.start_bound() {
                Bound::Included(x) => Bound::Excluded(x),
                Bound::Excluded(x) => Bound::Excluded(x),
                Bound::Unbounded => Bound::Excluded(&0),
            }
        }

        fn end_bound(&self) -> Bound<&usize> {
            self.0.end_bound()
        }
    }

    #[async_std::test]
    pub async fn test_revert<D: TestableDataSource>() {
        setup_test();

        let storage = D::create(0).await;
        let mut ds = D::connect(&storage).await;

        // Mock up some consensus data.
        let block = MockBlock::new();
        let time = ViewNumber::genesis();
        let state = MockState::default().append(&block, &time).unwrap();
        let mut qc = QuorumCertificate::<MockTypes, MockNodeImpl>::genesis();
        let mut leaf = Leaf::<MockTypes, MockNodeImpl>::new(time, qc.clone(), block.clone(), state);
        leaf.set_height(1);

        qc.leaf_commitment = leaf.commit();
        let block = BlockQueryData::new::<MockNodeImpl>(&leaf, &qc, block).unwrap();
        let leaf = LeafQueryData::new(leaf, qc).unwrap();

        // Insert, but do not commit, some data and check that we can read it back.
        ds.insert_leaf(leaf.clone()).await.unwrap();
        ds.insert_block(block.clone()).await.unwrap();

        assert_eq!(ds.block_height().await.unwrap(), 1);
        assert_eq!(leaf, ds.get_leaf(0).await.await);
        assert_eq!(block, ds.get_block(0).await.await);

        // Revert the changes.
        ds.revert().await;
        assert_eq!(ds.block_height().await.unwrap(), 0);
        assert!(matches!(ds.get_leaf(0).await, Fetch::Pending(_)));
        assert!(matches!(ds.get_block(0).await, Fetch::Pending(_)));
    }
}
