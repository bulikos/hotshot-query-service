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

#![cfg(feature = "metrics-data-source")]

use crate::{metrics::PrometheusMetrics, status::StatusDataSource, QueryError, QueryResult};
use async_trait::async_trait;

/// A minimal data source for the status API provided in this crate, with no persistent storage.
///
/// [`MetricsDataSource`] uses the metrics provided by HotShot to implement [`StatusDataSource`]. It
/// updates automatically whenever HotShot updates its metrics. All of the state for the metrics
/// data source is kept in memory, so it does not require a persistent storage backend. This makes
/// [`MetricsDataSource`] an attractive choice when only the status API is desired, not the
/// availability API.
///
/// Since all the state required by [`MetricsDataSource`] is updated automatically by HotShot, there
/// is no need to spawn an update loop to update the data source with new events, as is required
/// with full archival data sources like [`SqlDataSource`](super::SqlDataSource). Instead,
/// [`MetricsDataSource`] will be populated with useful data as long as its
/// [`populate_metrics`](crate::status::UpdateStatusData::populate_metrics) is used to initialize
/// HotShot:
///
/// ```
/// # use hotshot::SystemContext;
/// # use hotshot_query_service::{
/// #   data_source::MetricsDataSource,
/// #   status::UpdateStatusData,
/// #   testing::mocks::{MockNodeImpl as AppNodeImpl, MockTypes as AppTypes},
/// #   Error,
/// # };
/// # use hotshot_types::consensus::ConsensusMetricsValue;
/// # async fn doc() -> Result<(), hotshot_query_service::Error> {
/// let data_source = MetricsDataSource::default();
/// let (mut hotshot, _) = SystemContext::<AppTypes, AppNodeImpl>::init(
/// #   panic!(), panic!(), panic!(), panic!(), panic!(), panic!(), panic!(), panic!(),
///     ConsensusMetricsValue::new(&*data_source.populate_metrics()),
///     // Other fields omitted
/// ).await.map_err(Error::internal)?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default)]
pub struct MetricsDataSource {
    metrics: PrometheusMetrics,
}

#[async_trait]
impl StatusDataSource for MetricsDataSource {
    async fn block_height(&self) -> QueryResult<usize> {
        let last_synced_height = self
            .consensus_metrics()?
            .get_gauge("last_synced_block_height")
            .map_err(|err| QueryError::Error {
                message: err.to_string(),
            })?
            .get();
        if last_synced_height == 0 {
            // The block height must always be at least one, since the genesis block is assumed to
            // exist by default. We need to specially handle the case where we havent received any
            // decide events yet, since in this case the height will be 0, and the genesis block
            // itself does not trigger a decide event.
            //
            // This is required for consistency with the other data sources, which insert the
            // genesis block at data source creation time, so that their block height is never 0.
            Ok(1)
        } else {
            Ok(last_synced_height)
        }
    }

    fn metrics(&self) -> &PrometheusMetrics {
        &self.metrics
    }
}

#[cfg(any(test, feature = "testing"))]
mod impl_testable_data_source {
    use super::*;
    use crate::testing::mocks::{DataSourceLifeCycle, MockTypes};
    use hotshot::types::Event;

    #[async_trait]
    impl DataSourceLifeCycle for MetricsDataSource {
        type Storage = PrometheusMetrics;

        async fn create(_node_id: usize) -> Self::Storage {
            Default::default()
        }

        async fn connect(storage: &Self::Storage) -> Self {
            Self {
                metrics: storage.clone(),
            }
        }

        async fn handle_event(&mut self, _event: &Event<MockTypes>) {}
    }
}

#[cfg(test)]
mod test {
    use super::super::status_tests;
    use super::MetricsDataSource;

    // For some reason this is the only way to import the macro defined in another module of this
    // crate.
    use crate::*;

    instantiate_status_tests!(MetricsDataSource);
}
