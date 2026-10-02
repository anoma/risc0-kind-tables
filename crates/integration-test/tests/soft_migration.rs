//! The soft-migration gate: on a fork of the staging chain, a V1 resource converts and leaves through the current
//! forwarder, in transactions proven against the staging table this source generates. It needs the forked protocol
//! adapter to store the generated commitment and the current forwarder to hold the V1 balance, which a promotion into
//! `staging` requires. The fixture is alice's V1 USDC resource on Sepolia, so a fork of another chain skips, and every
//! run consumes the same resource, because it runs on a fork.

#![cfg(feature = "e2e")]

use std::path::PathBuf;
use std::str::FromStr;

use alloy::primitives::{Address, B256, U256};
use anoma_pa_evm_integration_test::keychain::EvmSigner;
use anoma_pa_evm_integration_test::state::chains::chain_id;
use anoma_pa_testkit::environment::{CommitmentTree, Environment, ProtocolAdapter};
use anoma_pa_testkit::fixtures::identities;
use anoma_pa_testkit::{execute_tx, prove_actions};
use anoma_risc0_kind_tables::{Caip2ChainId, table};
use anoma_rm_risc0::Digest;
use anoma_rm_risc0::compliance::KindTableEntry;
use anoma_rm_risc0::constants::{init_kind_table_from_entries, kind_table_hash};
use anoma_rm_risc0::merkle_path::MerklePath;
use anoma_rm_risc0::resource::Resource;
use anomapay_erc20_forwarder_bindings::addresses::Environment as ForwarderEnvironment;
use anomapay_erc20_forwarder_bindings::contract::erc20_forwarder;
use anomapay_erc20_forwarder_integration_test::deploy::erc20_bindings::erc20_example;
use anomapay_erc20_forwarder_integration_test::fixtures::{transfer, unwrap};
use anyhow::{Context, ensure};
use rstest::*;
use serde_json::Value;
use transfer_witness::{ValueInfo, calculate_persistent_value_ref};

type EvmE2eEnv = anoma_pa_evm_integration_test::envs::e2e::Environment;

/// Alice's V1 USDC resource, with its path in the tree the protocol adapter holds since the state migration.
struct V1Resource {
    chain_id: u64,
    token: Address,
    resource: Resource,
    nullifier: B256,
    path: MerklePath,
}

fn b256(fixture: &Value, key: &str) -> anyhow::Result<B256> {
    let hex = fixture[key].as_str().with_context(|| format!("no {key}"))?;
    Ok(B256::from_str(hex)?)
}

/// Reads the fixture and checks that the resource opens to its commitment and carries alice's value ref.
fn v1_resource() -> anyhow::Result<V1Resource> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/sepolia-v1-usdc-resource.json");
    let fixture: Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;

    let resource: Resource = serde_json::from_value(fixture["resource"].clone())?;
    let commitment = b256(&fixture, "commitment")?;
    ensure!(
        resource.commitment().as_bytes() == commitment.as_slice(),
        "the fixture resource opens to {}, not to {commitment}",
        resource.commitment()
    );
    let alice = identities::alice()?;
    let value_ref = calculate_persistent_value_ref(&ValueInfo {
        auth_pk: alice.auth_verifying_key(),
        encryption_pk: alice.encryption_pk,
    });
    ensure!(
        resource.value_ref == value_ref,
        "the fixture resource carries the value ref {}, alice's is {value_ref}",
        resource.value_ref
    );

    let path = fixture["merklePath"]
        .as_array()
        .context("no merkle path")?
        .iter()
        .map(|node| {
            let sibling = b256(node, "sibling")?;
            let on_right = node["leafIsOnRight"].as_bool().context("no side")?;
            Ok((Digest::from_bytes(sibling.0), on_right))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let path = MerklePath::from_path(&path);
    let root = b256(&fixture, "commitmentTreeRoot")?;
    ensure!(
        path.root(&resource.commitment()).as_bytes() == root.as_slice(),
        "the fixture path does not lead to the root {root}"
    );

    Ok(V1Resource {
        chain_id: fixture["chainId"].as_u64().context("no chain ID")?,
        token: Address::from_str(fixture["token"].as_str().context("no token")?)?,
        resource,
        nullifier: b256(&fixture, "nullifier")?,
        path,
    })
}

/// The fixture resource, if the fork is of its chain and the resource is unspent there.
async fn unspent_v1_resource(env: &EvmE2eEnv) -> anyhow::Result<Option<V1Resource>> {
    let v1 = v1_resource()?;
    if chain_id(env)? != v1.chain_id {
        eprintln!("skipped: the fixture resource is on chain {}", v1.chain_id);
        return Ok(None);
    }
    let spent = env
        .protocol_adapter
        .pa
        .isNullifierContained(v1.nullifier)
        .call()
        .await?;
    ensure!(
        !spent,
        "a transaction on chain consumed the fixture resource; replace it"
    );
    Ok(Some(v1))
}

/// Loads the staging table this source generates for the fork's chain, before a fixture reads the table, and checks
/// that the forked protocol adapter stores its commitment.
async fn load_the_generated_table(env: &EvmE2eEnv) -> anyhow::Result<()> {
    let chain = Caip2ChainId::eip155(chain_id(env)?);
    let table = table::staging::table(chain.clone())
        .with_context(|| format!("{chain}: no staging table"))?;
    init_kind_table_from_entries(table.entries.iter().map(KindTableEntry::from).collect())
        .context("failed to load the kind table")?;
    let loaded = kind_table_hash().context("no kind table loaded")?;
    let stored = env
        .protocol_adapter
        .pa
        .getKindTableCommitment()
        .call()
        .await
        .context("failed to query the kind table commitment")?;
    ensure!(
        stored.as_slice() == loaded.as_bytes(),
        "{chain}: the protocol adapter stores {stored}, the source generates {loaded}"
    );
    Ok(())
}

/// The recorded staging ERC20 forwarder, which the generated table names as the current forwarder. It must serve the
/// forked protocol adapter and hold at least the quantity of the V1 resource.
async fn current_forwarder(env: &EvmE2eEnv, v1: &V1Resource) -> anyhow::Result<Address> {
    let provider = env.protocol_adapter.pa.provider();
    let forwarder = erc20_forwarder(provider, ForwarderEnvironment::Staging)
        .await
        .context("no staging ERC20 forwarder recorded for the fork's chain")?;
    let address = *forwarder.address();
    let served = forwarder.getProtocolAdapter().call().await?;
    let protocol_adapter = *env.protocol_adapter.pa.address();
    ensure!(
        served == protocol_adapter,
        "the current forwarder {address} serves the protocol adapter {served}, the fork's is {protocol_adapter}"
    );
    let held = erc20_example(v1.token, provider.clone())
        .balanceOf(address)
        .call()
        .await?;
    let quantity = U256::from(v1.resource.quantity);
    ensure!(
        held >= quantity,
        "the current forwarder {address} holds {held} of {}, the V1 resource carries {quantity}",
        v1.token
    );
    Ok(address)
}

#[rstest]
#[case::e2e_test(EvmE2eEnv::setup_bare())]
#[tokio::test]
async fn unwrap_releases_a_v1_resource_through_the_current_forwarder(
    #[future(awt)]
    #[case]
    env: anyhow::Result<EvmE2eEnv>,
) -> anyhow::Result<()> {
    let mut env = env.context("env setup failed")?;
    let Some(v1) = unspent_v1_resource(&env).await? else {
        return Ok(());
    };
    load_the_generated_table(&env).await?;
    let forwarder = current_forwarder(&env, &v1).await?;
    let alice = identities::alice()?;
    let recipient = alice.address();
    let quantity = U256::from(v1.resource.quantity);

    let unwrap = unwrap::build(
        v1.resource,
        forwarder,
        v1.token,
        0,
        Some(v1.path),
        unwrap::Overrides {
            owner: Some(alice),
            ..unwrap::Overrides::default()
        },
    )
    .context("failed to build the unwrap action")?;
    let tx = prove_actions(&env, &[unwrap.witnesses])
        .await
        .context("failed to prove the unwrap action")?;

    let usdc = erc20_example(v1.token, env.protocol_adapter.pa.provider().clone());
    let recipient_before = usdc.balanceOf(recipient).call().await?;
    let forwarder_before = usdc.balanceOf(forwarder).call().await?;

    execute_tx(&mut env, tx)
        .await
        .context("failed to execute the unwrap action")?;

    assert_eq!(
        usdc.balanceOf(recipient).call().await? - recipient_before,
        quantity
    );
    assert_eq!(
        forwarder_before - usdc.balanceOf(forwarder).call().await?,
        quantity
    );
    assert!(
        env.protocol_adapter
            .pa
            .isNullifierContained(v1.nullifier)
            .call()
            .await?
    );
    Ok(())
}

#[rstest]
#[case::e2e_test(EvmE2eEnv::setup_bare())]
#[tokio::test]
async fn transfer_converts_a_v1_resource_into_a_current_resource(
    #[future(awt)]
    #[case]
    env: anyhow::Result<EvmE2eEnv>,
) -> anyhow::Result<()> {
    let mut env = env.context("env setup failed")?;
    let Some(v1) = unspent_v1_resource(&env).await? else {
        return Ok(());
    };
    load_the_generated_table(&env).await?;
    let forwarder = current_forwarder(&env, &v1).await?;
    let quantity = U256::from(v1.resource.quantity);
    let usdc = erc20_example(v1.token, env.protocol_adapter.pa.provider().clone());
    let forwarder_before = usdc.balanceOf(forwarder).call().await?;

    // Alice's V1 resource becomes bob's resource under the current forwarder's label. No token moves.
    let transfer = transfer::build(
        v1.resource,
        forwarder,
        v1.token,
        0,
        Some(v1.path),
        transfer::Overrides::default(),
    )
    .context("failed to build the transfer action")?;
    let tx = prove_actions(&env, &[transfer.witnesses])
        .await
        .context("failed to prove the transfer action")?;
    execute_tx(&mut env, tx)
        .await
        .context("failed to execute the transfer action")?;

    assert!(
        env.protocol_adapter
            .pa
            .isNullifierContained(v1.nullifier)
            .call()
            .await?
    );
    assert_eq!(usdc.balanceOf(forwarder).call().await?, forwarder_before);

    // Bob unwraps the current resource like any other.
    let path = env
        .protocol_adapter()
        .commitment_tree()
        .path_to(transfer.created_persistent.commitment())
        .context("failed to generate the unwrap merkle path")?;
    let unwrap = unwrap::build(
        transfer.created_persistent,
        forwarder,
        v1.token,
        1,
        Some(path),
        unwrap::Overrides::default(),
    )
    .context("failed to build the unwrap action")?;
    let tx = prove_actions(&env, &[unwrap.witnesses])
        .await
        .context("failed to prove the unwrap action")?;

    let bob = identities::bob()?.address();
    let bob_before = usdc.balanceOf(bob).call().await?;
    execute_tx(&mut env, tx)
        .await
        .context("failed to execute the unwrap action")?;

    assert_eq!(usdc.balanceOf(bob).call().await? - bob_before, quantity);
    assert_eq!(
        forwarder_before - usdc.balanceOf(forwarder).call().await?,
        quantity
    );
    Ok(())
}
