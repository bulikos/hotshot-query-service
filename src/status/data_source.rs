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

use super::query_data::MempoolQueryData;
use crate::{
    availability::LeafQueryData,
    metrics::{MetricsError, PrometheusMetrics},
    QueryError, QueryResult,
};
use async_trait::async_trait;
use hotshot_types::traits::{
    metrics::Metrics,
    node_implementation::{NodeImplementation, NodeType},
    signature_key::EncodedPublicKey,
};

#[async_trait]
pub trait StatusDataSource<Types, I>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
{
    async fn block_height(&self) -> QueryResult<usize>;

    async fn get_proposals(
        &self,
        proposer: &EncodedPublicKey,
        limit: Option<usize>,
    ) -> QueryResult<Vec<LeafQueryData<Types, I>>>;
    async fn count_proposals(&self, proposer: &EncodedPublicKey) -> QueryResult<usize>;

    fn metrics(&self) -> &PrometheusMetrics;

    fn consensus_metrics(&self) -> QueryResult<PrometheusMetrics> {
        self.metrics()
            .get_subgroup(["consensus"])
            .map_err(metrics_err)
    }

    async fn mempool_info(&self) -> QueryResult<MempoolQueryData> {
        Ok(MempoolQueryData {
            transaction_count: self
                .consensus_metrics()?
                .get_gauge("outstanding_transactions")
                .map_err(metrics_err)?
                .get() as u64,
            memory_footprint: self
                .consensus_metrics()?
                .get_gauge("outstanding_transactions_memory_size")
                .map_err(metrics_err)?
                .get() as u64,
        })
    }

    async fn success_rate(&self) -> QueryResult<f64> {
        let total_views = self
            .consensus_metrics()?
            .get_gauge("current_view")
            .map_err(metrics_err)?
            .get() as f64;
        // By definition, a successful view is any which committed a block.
        Ok(self.block_height().await? as f64 / total_views)
    }

    /// Export all available metrics in the Prometheus text format.
    async fn export_metrics(&self) -> QueryResult<String> {
        self.metrics().prometheus().map_err(metrics_err)
    }
}

pub trait UpdateStatusData<Types, I> {
    fn populate_metrics(&self) -> Box<dyn Metrics>;
}

impl<Types, I, T> UpdateStatusData<Types, I> for T
where
    Types: NodeType,
    I: NodeImplementation<Types>,
    T: StatusDataSource<Types, I>,
{
    fn populate_metrics(&self) -> Box<dyn Metrics> {
        Box::new(self.metrics().clone())
    }
}

fn metrics_err(err: MetricsError) -> QueryError {
    QueryError::Error {
        message: err.to_string(),
    }
}
