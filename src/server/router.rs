use axum::{
    extract::{Extension, Path, Query},
    handler::Handler,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use crate::{
    database::SignedTx,
    error::ChainError,
    node::{Node, Peer},
    types::Hash,
};

type ArcNode<P> = Arc<RwLock<Node<P>>>;

pub fn new_router<P>(node: ArcNode<P>) -> Router
where
    P: Peer + Send + Sync + 'static,
{
    Router::new()
        .route("/blocks", get(get_blocks::<P>))
        .route("/blocks/:number", get(get_block::<P>))
        .route("/balances", get(get_balances::<P>))
        .route("/txs", post(add_tx::<P>))
        .route("/peer/ping", post(ping_peer::<P>))
        .route("/peer/status", get(get_peer_status::<P>))
        .fallback(not_found.into_service())
        .layer(Extension(node))
}

#[derive(Debug, Serialize)]
struct OkResp {
    success: bool,
}

impl OkResp {
    pub fn new() -> Self {
        Self { success: true }
    }
}

#[derive(Debug, Deserialize)]
struct GetBlocksReq {
    offset: usize,
}

async fn get_blocks<P: Peer + Send + Sync + 'static>(
    Query(params): Query<GetBlocksReq>,
    Extension(node): Extension<ArcNode<P>>,
) -> Result<impl IntoResponse, ChainError> {
    let blocks = node.read().unwrap().get_blocks(params.offset)?;
    Ok(Json(blocks))
}

async fn get_block<P: Peer + Send + Sync + 'static>(
    Path(number): Path<u64>,
    Extension(node): Extension<ArcNode<P>>,
) -> Result<impl IntoResponse, ChainError> {
    let block = node.read().unwrap().get_block(number)?;
    Ok(Json(block))
}

#[derive(Debug, Serialize)]
struct BalancesResp {
    hash: Hash,
    balances: HashMap<String, u64>,
}

async fn get_balances<P: Peer + Send + Sync + 'static>(
    Extension(node): Extension<ArcNode<P>>,
) -> impl IntoResponse {
    let node = node.read().unwrap();

    Json(BalancesResp {
        hash: node.latest_block_hash(),
        balances: node.get_balances(),
    })
}

#[derive(Debug, Deserialize)]
struct AddTxReq {
    from: String,
    to: String,
    value: u64,
}

async fn add_tx<P: Peer + Send + Sync + 'static>(
    Json(tx): Json<AddTxReq>,
    Extension(node): Extension<ArcNode<P>>,
) -> Result<impl IntoResponse, ChainError> {
    node.write().unwrap().add_tx(&tx.from, &tx.to, tx.value)?;

    Ok(Json(OkResp::new()))
}

#[derive(Debug, Deserialize)]
struct PingPeerReq {
    addr: String,
}

async fn ping_peer<P: Peer + Send + Sync + 'static>(
    Json(peer): Json<PingPeerReq>,
    Extension(node): Extension<ArcNode<P>>,
) -> Result<impl IntoResponse, ChainError> {
    node.write().unwrap().add_peer(&peer.addr)?;

    Ok(Json(OkResp::new()))
}

#[derive(Debug, Serialize)]
struct PeerStatusResp {
    hash: Hash,
    number: u64,
    peers: HashSet<SocketAddr>,
    pending_txs: Vec<SignedTx>,
}

async fn get_peer_status<P: Peer + Send + Sync + 'static>(
    Extension(node): Extension<ArcNode<P>>,
) -> impl IntoResponse {
    let node = node.read().unwrap();

    Json(PeerStatusResp {
        hash: node.latest_block_hash(),
        number: node.latest_block_number(),
        peers: node.peers.clone(),
        pending_txs: node.get_pending_txs(),
    })
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Not Found")
}