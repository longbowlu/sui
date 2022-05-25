// Copyright (c) 2022, Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{authority_active::ActiveAuthority, checkpoints::checkpoint_tests::TestSetup};

use std::{time::Duration, collections::BTreeSet};
use sui_types::messages::ExecutionStatus;

use crate::checkpoints::checkpoint_tests::checkpoint_tests_setup;

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn checkpoint_active_flow() {
    let setup = checkpoint_tests_setup(20, Duration::from_millis(200)).await;

    let TestSetup {
        committee: _committee,
        authorities,
        mut transactions,
        aggregator,
    } = setup;

    // Start active part of authority.
    for inner_state in authorities.clone() {
        let clients = aggregator.authority_clients.clone();
        let _active_handle = tokio::task::spawn(async move {
            let active_state =
                ActiveAuthority::new(inner_state.authority.clone(), clients).unwrap();
            active_state.spawn_all_active_processes().await
        });
    }

    let sender_aggregator = aggregator.clone();
    let _end_of_sending_join = tokio::task::spawn(async move {
        while let Some(t) = transactions.pop() {
            let (_cert, effects) = sender_aggregator
                .execute_transaction(&t)
                .await
                .expect("All ok.");

            // Check whether this is a success?
            assert!(matches!(effects.status, ExecutionStatus::Success { .. }));
            println!("Execute at {:?}", tokio::time::Instant::now());

            // Add some delay between transactions
            tokio::time::sleep(Duration::from_millis(412)).await;
        }
    });

    // Wait for all the sending to happen.
    _end_of_sending_join.await.expect("all ok");

    // Wait for a batch to go through
    // (We do not really wait, we jump there since real-time is not running).
    tokio::time::sleep(Duration::from_secs(10)).await;

    let mut value_set = BTreeSet::new();
    for a in authorities {
        let next_checkpoint_sequence = a.authority._checkpoints.as_ref().unwrap().lock().next_checkpoint();
        assert!(next_checkpoint_sequence >= 2, "Expected {} > 2", next_checkpoint_sequence);
        value_set.insert(next_checkpoint_sequence);
    }

    // After the end all authorities are the same
    assert!(value_set.len() == 1, "Got set {:?}", value_set);


}
