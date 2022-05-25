// Copyright (c) 2022, Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use jsonrpsee_http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee_http_server::{HttpServerBuilder, HttpServerHandle, RpcModule};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::{num::NonZeroUsize, path::Path};
use sui::{
    config::{
        Config, GatewayConfig, GatewayType, WalletConfig, SUI_GATEWAY_CONFIG, SUI_NETWORK_CONFIG,
        SUI_WALLET_CONFIG,
    },
    keystore::{KeystoreType, SuiKeystore},
    sui_commands::SuiNetwork,
    wallet_commands::{WalletCommands, WalletContext},
};
use sui_config::{builder::ConfigBuilder, GenesisConfig, PersistedConfig};
use sui_gateway::api::RpcGatewayApiServer;
use sui_gateway::api::RpcReadApiServer;
use sui_gateway::api::RpcTransactionBuilderServer;
use sui_gateway::rpc_gateway::{
    create_client, GatewayReadApiImpl, RpcGatewayImpl, TransactionBuilderImpl,
};
use sui_types::base_types::SuiAddress;
use tempfile::TempDir;
const NUM_VALIDAOTR: usize = 4;

pub async fn start_test_network(
    working_dir: &Path,
    genesis_config: Option<GenesisConfig>,
) -> Result<SuiNetwork, anyhow::Error> {
    std::fs::create_dir_all(working_dir)?;
    let working_dir = working_dir.to_path_buf();
    let network_path = working_dir.join(SUI_NETWORK_CONFIG);
    let wallet_path = working_dir.join(SUI_WALLET_CONFIG);
    let keystore_path = working_dir.join("wallet.key");
    let db_folder_path = working_dir.join("client_db");

    let mut builder =
        ConfigBuilder::new(&working_dir).committee_size(NonZeroUsize::new(NUM_VALIDAOTR).unwrap());

    if let Some(genesis_config) = genesis_config {
        builder = builder.initial_accounts_config(genesis_config);
    }

    let network_config = builder.build();
    let accounts = network_config
        .account_keys
        .iter()
        .map(|key| SuiAddress::from(key.public_key_bytes()))
        .collect::<Vec<_>>();
    let network = SuiNetwork::start(&network_config).await?;

    let network_config = network_config.persisted(&network_path);
    network_config.save()?;
    let mut keystore = SuiKeystore::default();
    for key in &network_config.account_keys {
        keystore.add_key(SuiAddress::from(key.public_key_bytes()), key.copy())?;
    }
    keystore.set_path(&keystore_path);
    keystore.save()?;

    let validators = network_config.validator_set().to_owned();
    let active_address = accounts.get(0).copied();

    GatewayConfig {
        db_folder_path: db_folder_path.clone(),
        validator_set: validators.clone(),
        ..Default::default()
    }
    .persisted(&working_dir.join(SUI_GATEWAY_CONFIG))
    .save()?;

    // Create wallet config with stated authorities port
    WalletConfig {
        accounts,
        keystore: KeystoreType::File(keystore_path),
        gateway: GatewayType::Embedded(GatewayConfig {
            db_folder_path,
            validator_set: validators,
            ..Default::default()
        }),
        active_address,
    }
    .persisted(&wallet_path)
    .save()?;

    // Return network handle
    Ok(network)
}

pub async fn setup_network_and_wallet_in_working_dir(
    working_dir: &TempDir,
) -> Result<(SuiNetwork, WalletContext, SuiAddress), anyhow::Error> {
    let network = start_test_network(working_dir.path(), None).await?;

    // Create Wallet context.
    let wallet_conf = working_dir.path().join(SUI_WALLET_CONFIG);
    let mut context = WalletContext::new(&wallet_conf)?;
    let address = context.config.accounts.first().cloned().unwrap();

    // Sync client to retrieve objects from the network.
    WalletCommands::SyncClientState {
        address: Some(address),
    }
    .execute(&mut context)
    .await?;
    Ok((network, context, address))
}

pub async fn setup_network_and_wallet(
) -> Result<(SuiNetwork, WalletContext, SuiAddress), anyhow::Error> {
    let working_dir = tempfile::tempdir()?;
    setup_network_and_wallet_in_working_dir(&working_dir).await
}

async fn start_rpc_gateway(
    config_path: &Path,
) -> Result<(SocketAddr, HttpServerHandle), anyhow::Error> {
    let server = HttpServerBuilder::default().build("127.0.0.1:0").await?;
    let addr = server.local_addr()?;
    let client = create_client(config_path)?;
    let mut module = RpcModule::new(());
    module.merge(RpcGatewayImpl::new(client.clone()).into_rpc())?;
    module.merge(GatewayReadApiImpl::new(client.clone()).into_rpc())?;
    module.merge(TransactionBuilderImpl::new(client.clone()).into_rpc())?;

    let handle = server.start(module)?;
    Ok((addr, handle))
}

pub async fn start_rpc_test_network(
    genesis_config: Option<GenesisConfig>,
) -> Result<TestNetwork, anyhow::Error> {
    let working_dir = tempfile::tempdir()?.path().to_path_buf();
    let _network = start_test_network(&working_dir, genesis_config).await?;
    let (server_addr, rpc_server_handle) =
        start_rpc_gateway(&working_dir.join(SUI_GATEWAY_CONFIG)).await?;
    let mut wallet_conf: WalletConfig =
        PersistedConfig::read(&working_dir.join(SUI_WALLET_CONFIG))?;
    let rpc_url = format!("http://{}", server_addr);
    let accounts = wallet_conf.accounts.clone();
    wallet_conf.gateway = GatewayType::RPC(rpc_url.clone());
    wallet_conf
        .persisted(&working_dir.join(SUI_WALLET_CONFIG))
        .save()?;

    let http_client = HttpClientBuilder::default().build(rpc_url)?;
    Ok(TestNetwork {
        _network,
        _rpc_server: rpc_server_handle,
        accounts,
        http_client,
        working_dir,
    })
}

pub struct TestNetwork {
    _network: SuiNetwork,
    _rpc_server: HttpServerHandle,
    pub accounts: Vec<SuiAddress>,
    pub http_client: HttpClient,
    pub working_dir: PathBuf,
}
