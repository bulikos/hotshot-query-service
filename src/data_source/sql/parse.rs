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

//! Retrieving and parsing data from a SQL database.

use crate::{
    availability::{BlockQueryData, LeafQueryData, QueryableBlock},
    Block, Leaf, MissingSnafu, QueryError, QueryResult, QuorumCertificate,
};
use hotshot_types::traits::node_implementation::{NodeImplementation, NodeType};
use snafu::OptionExt;
use time::OffsetDateTime;
use tokio_postgres::Row;

/// The columns which must be selected in order to parse a [`Row`] using [`parse_leaf`].
pub(super) const LEAF_COLUMNS: &str = "leaf, qc";

/// Interpret a row of a SQL table as a [`LeafQueryData`].
pub(super) fn parse_leaf<Types, I>(row: Row) -> QueryResult<LeafQueryData<Types, I>>
where
    Types: NodeType,
    I: NodeImplementation<Types>,
{
    let leaf = row.try_get("leaf").map_err(|err| QueryError::Error {
        message: format!("error extracting leaf from query results: {err}"),
    })?;
    let leaf: Leaf<Types, I> = serde_json::from_value(leaf).map_err(|err| QueryError::Error {
        message: format!("malformed leaf: {err}"),
    })?;

    let qc = row.try_get("qc").map_err(|err| QueryError::Error {
        message: format!("error extracting QC from query results: {err}"),
    })?;
    let qc: QuorumCertificate<Types, I> =
        serde_json::from_value(qc).map_err(|err| QueryError::Error {
            message: format!("malformed QC: {err}"),
        })?;

    Ok(LeafQueryData { leaf, qc })
}

/// The columns which must be selected in order to parse a [`Row`] using [`parse_block`].
pub(super) const BLOCK_COLUMNS: &str = "h.hash AS hash, h.height AS height, h.timestamp AS timestamp, p.size AS payload_size, p.data AS payload_data";

/// Interpret a row of a SQL table as a [`LeafQueryData`].
pub(super) fn parse_block<Types>(row: Row) -> QueryResult<BlockQueryData<Types>>
where
    Types: NodeType,
    Block<Types>: QueryableBlock,
{
    // First, check if we have the payload for this block yet.
    let size: Option<i32> = row
        .try_get("payload_size")
        .map_err(|err| QueryError::Error {
            message: format!("error extracting payload size from query results: {err}"),
        })?;
    let data: Option<Vec<u8>> = row
        .try_get("payload_data")
        .map_err(|err| QueryError::Error {
            message: format!("error extracting payload data from query results: {err}"),
        })?;
    let (size, data) = size.zip(data).context(MissingSnafu)?;
    let size = size as u64;

    // Reconstruct the full block.
    let block = bincode::deserialize(&data).map_err(|err| QueryError::Error {
        message: format!("malformed payload data: {err}"),
    })?;

    // Reconstruct the query data by adding metadata.
    let hash: String = row.try_get("hash").map_err(|err| QueryError::Error {
        message: format!("error extracting block hash from query results: {err}"),
    })?;
    let hash = hash.parse().map_err(|err| QueryError::Error {
        message: format!("malformed block hash: {err}"),
    })?;
    let height: i64 = row.try_get("height").map_err(|err| QueryError::Error {
        message: format!("error extracting block height from query results: {err}"),
    })?;
    let height = height as u64;
    let timestamp: OffsetDateTime = row.try_get("timestamp").map_err(|err| QueryError::Error {
        message: format!("error extracting block height from query results: {err}"),
    })?;
    let timestamp = timestamp.unix_timestamp_nanos();

    Ok(BlockQueryData {
        block,
        size,
        hash,
        height,
        timestamp,
    })
}
