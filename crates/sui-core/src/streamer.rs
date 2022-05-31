// Copyright (c) 2021, Facebook, Inc. and its affiliates
// Copyright (c) 2022, Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
use sui_types::event::EventEnvelope;
use tokio::{
    sync::mpsc::Receiver,
    task::JoinHandle,
};
use tracing::debug;

pub struct Streamer {
    event_queue: Receiver<EventEnvelope>,
}

impl Streamer {
    pub fn spawn(rx: Receiver<EventEnvelope>) -> JoinHandle<()> {
        tokio::spawn(async move {
            Self { event_queue: rx }
            .stream()
            .await
        })
    }

    pub async fn stream(&mut self) {
        while let Some(event_envelope) = self.event_queue.recv().await {
            debug!("@@@@@@@@@@@@@ got event! {:?}", event_envelope);
            if let Some(json_value) = &event_envelope.move_event_json_contents {
                debug!("@@@@@@@@@@@@@ json value: {} ", json_value);
                debug!("@@@@@@@@@@@@@ object id : {} ", json_value["object_id"]["bytes"]);
                debug!("@@@@@@@@@@@@@ non existent 1: {} ", json_value["object_haha"]);
                debug!("@@@@@@@@@@@@@ non existent 2: {} ", json_value["object_id"]["haha"]);
            }
        }
    }
}
