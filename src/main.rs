use beacon_node::beacon_chain::{
    store::{HotColdDB, KeyValueStore, KeyValueStoreOp, LevelDB, StoreConfig},
    types::{ChainSpec, EthSpec, Hash256, MainnetEthSpec},
};
use clap::Parser;
use futures::future::TryJoinAll;
use reqwest::StatusCode;
use slog::{o, Drain};
use std::{error::Error, io::Write, path::PathBuf, sync::Arc};

mod types;
use types::*;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    api_url: String,

    #[arg(short, long)]
    from_slot: usize,

    #[arg(short, long)]
    to_slot: Option<usize>,

    #[arg(short, long, default_value_t = 20)]
    concurrency: usize,

    #[arg(short, long)]
    data_dir: PathBuf,
}

fn get_url<'a>(base_url: &'a str, path: &'a str) -> String {
    format!(
        "{base_url}{}",
        if base_url.ends_with("/") {
            path.to_string()
        } else {
            format!("/{path}")
        }
    )
}

async fn download_blob_sidecars(
    api_url: String,
    slot: usize,
    n_retry: usize,
    sleep_ms: u64,
) -> anyhow::Result<BlobSidecarsResponse> {
    for i in 0..n_retry {
        let data = reqwest::get(get_url(
            &api_url,
            &format!("eth/v1/beacon/blob_sidecars/{slot}"),
        ))
        .await;
        if data.is_err() {
            // poor man's retry
            tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
            continue;
        };
        let data = data?;
        // NOT_FOUND is returned when the proposer of given slot missed to propose the block
        if let StatusCode::NOT_FOUND = data.status() {
            return Ok(BlobSidecarsResponse { data: vec![] });
        }
        return Ok(data.json::<BlobSidecarsResponse>().await?);
    }
    Err(anyhow::format_err!(
        "{n_retry} retries failed for slot {slot}"
    ))
}

async fn get_current_headers(api_url: &str) -> anyhow::Result<BlockHeadersData> {
    let data = reqwest::get(get_url(api_url, "eth/v1/beacon/headers"))
        .await?
        .json::<BlockHeadersData>()
        .await?;
    Ok(data)
}

async fn get_block_root_for_slot(api_url: String, slot: usize) -> anyhow::Result<String> {
    let data = reqwest::get(get_url(
        &api_url,
        format!("eth/v1/beacon/headers/{slot}").as_str(),
    ))
    .await?;
    if let StatusCode::NOT_FOUND = data.status() {
        return Ok("".to_string());
    }
    let data = data.json::<SingleBlockHeaderData>().await?;
    Ok(data.data.root)
}

async fn get_current_slot_number(api_url: &str) -> anyhow::Result<usize> {
    get_current_headers(api_url)
        .await?
        .data
        .first()
        .map_or(Err(anyhow::format_err!("no headers found")), |header| {
            Ok(header.header.message.slot.parse::<usize>()?)
        })
}

fn write_data(data: &[BlobsDataToWrite]) -> anyhow::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("blobs-data.jsonl")?;
    let to_write = data
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    file.write_all(to_write.as_bytes())?;
    Ok(())
}

const DANCUN_SLOT: usize = 8626176;

fn write_blobs<E: EthSpec>(
    store: Arc<HotColdDB<E, LevelDB<E>, LevelDB<E>>>,
    data: &[BlobsDataToWrite],
) -> anyhow::Result<()> {
    let mut batch = vec![];
    data.iter()
        .for_each(|x| store.blobs_as_kv_store_ops(&x.root, Into::<_>::into(x), &mut batch));
    store
        .blobs_db
        .do_atomically(batch)
        .expect("writing blobs to do failed");
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let from_slot = if args.from_slot < DANCUN_SLOT {
        println!(
            "warn: using {DANCUN_SLOT} instead of {} since blobs didnt exist before that slot",
            args.from_slot
        );
        DANCUN_SLOT
    } else {
        args.from_slot
    };
    let api_url = &args.api_url;
    let to_slot = if let Some(slot) = args.to_slot {
        slot
    } else {
        let current_slot = get_current_slot_number(api_url).await?;
        println!("head slot: {current_slot}");
        current_slot
    };
    let concurrency = args.concurrency;
    let n_slots = to_slot - from_slot;
    let data_dir = args.data_dir;

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = slog::Logger::root(drain, o!());

    let store = HotColdDB::open(
        &data_dir.join("chain_db"),
        &data_dir.join("freezer_db"),
        &data_dir.join("blobs_db"),
        |_: Arc<HotColdDB<MainnetEthSpec, LevelDB<_>, LevelDB<_>>>, _, _| Ok(()),
        StoreConfig {
            prune_blobs: false,
            ..Default::default()
        },
        ChainSpec::mainnet(),
        logger,
    )
    .unwrap();

    for slot in (from_slot..=to_slot).step_by(concurrency) {
        let mut handles = vec![];
        let mut root_handles = vec![];

        for slot in slot..(slot + concurrency) {
            handles.push(tokio::spawn(download_blob_sidecars(
                api_url.to_string(),
                slot,
                10,
                5000,
            )));
            root_handles.push(tokio::spawn(get_block_root_for_slot(
                api_url.to_string(),
                slot,
            )));
        }

        let join_handles = handles.into_iter().collect::<TryJoinAll<_>>();
        let root_handles = root_handles.into_iter().collect::<TryJoinAll<_>>();
        let (val, roots) = tokio::join!(join_handles, root_handles);
        let val = val?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .collect::<Vec<_>>();
        let roots = roots?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|x| x.parse::<Hash256>())
            .collect::<Result<Vec<_>, _>>()?;
        println!("roots: {:?}", roots);
        let val = roots
            .into_iter()
            .zip(val.into_iter())
            .map(|(root, data)| BlobsDataToWrite {
                slot,
                data: data.data,
                root,
            })
            .collect::<Vec<_>>();
        println!("writing blobs to level db now");
        // .map(|item| BlobsDataToWrite {
        //     slot,
        //     data: item.data,
        // })

        // write_data(&val[..])?;
        write_blobs(store.clone(), &val[..])?;
        println!(
            "blobs downloaded for {slot}..{} [{}%]",
            slot + concurrency,
            (slot - from_slot) as f64 / (n_slots as f64) * 100.0
        );

        break;
    }

    Ok(())
}
