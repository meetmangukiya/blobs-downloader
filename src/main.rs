use clap::Parser;
use futures::future::TryJoinAll;
use reqwest::StatusCode;
use std::{error::Error, io::Write};

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let from_slot = args.from_slot;
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

    for slot in (from_slot..=to_slot).step_by(concurrency) {
        let mut handles = vec![];

        for slot in slot..(slot + concurrency) {
            handles.push(tokio::spawn(download_blob_sidecars(
                api_url.to_string(),
                slot,
                10,
                5000,
            )));
        }

        let join_handles = handles.into_iter().collect::<TryJoinAll<_>>();
        let (val,) = tokio::join!(join_handles);
        let val = val?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|item| BlobsDataToWrite {
                slot,
                data: item.data,
            })
            .collect::<Vec<_>>();

        write_data(&val[..])?;
        println!(
            "blobs downloaded for {slot}..{} [{}%]",
            slot + concurrency,
            (slot - from_slot) as f64 / (n_slots as f64) * 100.0
        );
    }

    Ok(())
}
