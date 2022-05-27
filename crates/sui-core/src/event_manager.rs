// Copyright (c) 2021, Facebook, Inc. and its affiliates
// Copyright (c) 2022, Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use move_bytecode_utils::module_cache::ModuleCache;

pub struct EventStore {

}

pub struct EventManager {
    subscriptions: BTreeMap<EventType, Sender<SuiEvent>>,
    event_store: Arc<EventStore>,
}