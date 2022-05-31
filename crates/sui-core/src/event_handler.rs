// Copyright (c) 2021, Facebook, Inc. and its affiliates
// Copyright (c) 2022, Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use move_bytecode_utils::module_cache::SyncModuleCache;
use sui_types::{
    event::{Event, EventEnvelope},
    error::{SuiError, SuiResult}, messages::TransactionEffects,
};
use futures::future::join_all;
use move_bytecode_utils::layout::TypeLayoutBuilder;
use crate::authority::{ArcWrapper, AuthorityStore};
use crate::streamer::Streamer;
use tokio::sync::mpsc::{self, Sender};
use move_core_types::{
    language_storage::TypeTag,
    value::{MoveStruct, MoveTypeLayout},
};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::{debug, error};

const EVENT_DISPATCH_BUFFER_SIZE: usize = 1000;

pub fn get_unixtime_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Travelling in time machine")
        .as_millis()
}

pub struct EventHandler {
    module_cache: SyncModuleCache<ArcWrapper<AuthorityStore>>,
    sender: Sender<EventEnvelope>,
}

impl EventHandler {
    pub async fn new(validator_store: Arc<AuthorityStore>) -> Self {
        let (tx, rx) = mpsc::channel::<EventEnvelope>(EVENT_DISPATCH_BUFFER_SIZE);
        // tokio::spawn(EventHandler::disperser(rx));
        Streamer::spawn(rx);
        Self {
            module_cache: SyncModuleCache::new(ArcWrapper(validator_store)),
            sender: tx,
        }
    }

    pub async fn process_events(&self, effects: &TransactionEffects) {
        let futures = effects.events.iter().map(|e| self.process_event(e));
        let results = join_all(futures).await;
        for r in &results {
            if let Err(e) = r {
                error!(error =? e, "Failed to send EventEnvolope to dispatch");
            }
        }
    }

    pub async fn process_event(&self, event: &Event) -> SuiResult {
        debug!("@@@@@@@@@@@@@ event : {:?}", event);
        let envolope = match event {
            Event::MoveEvent { type_, contents} => {
                let typestruct = TypeTag::Struct(type_.clone());
                let layout =
                    TypeLayoutBuilder::build_with_fields(&typestruct, &self.module_cache).map_err(|e| {
                        SuiError::ObjectSerializationError {
                            error: e.to_string(),
                        }
                    })?;
                match layout {
                    MoveTypeLayout::Struct(l) => {
                        let s = MoveStruct::simple_deserialize(contents, &l).map_err(|e| {
                            SuiError::ObjectSerializationError {
                                error: e.to_string(),
                            }
                        })?;
                        // if let MoveStruct::WithFields(fields) = &s {
                        //     for (i, v) in fields {
                        //         let i_str = format!("{}", i);
                        //         let v_str = format!("{}", v);
                        //         debug!("@@@@@@@@@@@@@ identifier: {} ", i_str);
                        //         debug!("@@@@@@@@@@@@@ move value: {} ", v_str);
                        //         // format!("{}", a_type_tag)
                        //     }
                        // }
                        let json_v = serde_json::to_value(&s).map_err(|e| SuiError::ObjectSerializationError {
                            error: e.to_string(),
                        })?;

                        EventEnvelope::new(
                            get_unixtime_ms(),
                            None,
                            event.clone(),
                            Some(json_v),
                        )
                    }
                    _ => unreachable!(
                        "We called build_with_types on Struct type, should get a struct layout"
                    ),
                } 
            },
            // Event::TransferObject {object_id, version, destination_addr, type_} => {

            // },
            _ => { return Ok(()); }
        };
        // debug!("@@@@@@@@@@@@@ event struct: {:?} ", &s);
        debug!("@@@@@@@@@@@@@ sending..");
        self.sender.send(envolope).await.map_err(|e| SuiError::EventFailedToDispatch {error: e.to_string()})
        // if let Err(e) = self.sender.send(envolope).await {
        //     error!(event =? event, "Failed to send EventEnvolope to be dispatched");
        //     return );
        // }
        // Ok(())
        
    }
}