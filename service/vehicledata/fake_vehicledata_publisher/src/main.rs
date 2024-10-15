#![allow(missing_docs)]
// Copyright 2023 Google LLC

use crate::demo_sequence::create_demo_sequence;
use crate::grpc::create_grpc_client;
use core::time::Duration;
use log::info;
use utils::*;

mod demo_sequence;
mod grpc;
mod utils;

#[tokio::main]
async fn main() -> Result<(), ()> {
    info!("HAR-SDV service running.");

    // Create GRPC client.
    let mut service = create_grpc_client("127.0.0.1:7002".to_string());

    loop {
        let steps = create_demo_sequence();

        play_steps(&mut service, steps).await;
        // Wait a little and start again.
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
