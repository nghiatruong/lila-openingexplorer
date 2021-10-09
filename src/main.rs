pub mod api;
pub mod db;
pub mod indexer;
pub mod model;
pub mod util;

use crate::{
    api::{Error, PersonalQuery, PersonalResponse},
    db::Database,
    indexer::IndexerStub,
    model::PersonalKeyBuilder,
};
use axum::{
    extract::{Extension, Query},
    handler::get,
    response::Json,
    AddExtensionLayer, Router,
};
use shakmaty::{
    CastlingMode,
    Position,
    fen::Fen,
    variant::VariantPosition,
    zobrist::ZobristHash,
};
use clap::Clap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clap)]
struct Opt {
    #[clap(long = "bind", default_value = "127.0.0.1:9000")]
    bind: SocketAddr,
    #[clap(long = "db", default_value = "_db")]
    db: PathBuf,
}

#[tokio::main]
async fn main() {
    let opt = Opt::parse();

    let db = Arc::new(Database::open(opt.db).expect("db"));

    let (indexer, join_handle) = IndexerStub::spawn(db.clone());

    let app = Router::new()
        .route("/personal", get(personal))
        .layer(AddExtensionLayer::new(db))
        .layer(AddExtensionLayer::new(indexer));

    axum::Server::bind(&opt.bind)
        .serve(app.into_make_service())
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.expect("wait for ctrl-c");
        })
        .await
        .expect("bind");

    join_handle.await.expect("indexer");
}

async fn personal(
    Extension(db): Extension<Arc<Database>>,
    Extension(indexer): Extension<IndexerStub>,
    Query(query): Query<PersonalQuery>,
) -> Result<Json<PersonalResponse>, Error> {
    if true {
        let status = indexer.index_player(query.player.clone()).await?;
    }

    let mut pos = VariantPosition::from_setup(query.variant.into(), &Fen::from(query.fen), CastlingMode::Chess960)?;
    for uci in query.play {
        let m = uci.to_move(&pos)?;
        pos.play_unchecked(&m);
    }

    let key = PersonalKeyBuilder::with_user_pov(&query.player.into(), query.color).with_zobrist(pos.zobrist_hash());
    let queryable = db.queryable();
    dbg!(queryable.db.get_cf(queryable.cf_personal, dbg!(key.prefix())).expect("get cf personal"));
    Ok(Json(PersonalResponse {}))
}
