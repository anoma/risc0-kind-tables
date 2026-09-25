//! The recorded forwarders serve the recorded protocol adapters. On every chain the protocol adapter bindings record a
//! proxy for, the ERC20 and generic call forwarders recorded for the same environment must accept calls from that
//! proxy. A table built from a forwarder of another protocol adapter carries labels that no transaction on this one
//! can use. Nothing here has to be installed for it to hold, so it runs on every pull request and push.

use alloy::primitives::Address;
use alloy::sol;
use alloy_chains::NamedChain;
use anoma_generic_call_forwarder_bindings::addresses::{
    Environment as GenericCallEnvironment, generic_call_forwarder_address,
};
use anoma_pa_evm_bindings::addresses::{Environment, protocol_adapter_deployments_map};
use anoma_risc0_kind_tables::Caip2ChainId;
use anoma_risc0_kind_tables_integration_test::provider;
use anomapay_erc20_forwarder_bindings::addresses::{
    Environment as Erc20Environment, erc20_forwarder_address,
};
use anyhow::{Context, Result, ensure};

sol! {
    #[sol(rpc)]
    interface IForwarder {
        function getProtocolAdapter() external view returns (address protocolAdapter);
    }
}

fn erc20_forwarder(environment: Environment, chain: &NamedChain) -> Option<Address> {
    let environment = match environment {
        Environment::Staging => Erc20Environment::Staging,
        Environment::Production => Erc20Environment::Production,
    };
    erc20_forwarder_address(environment, chain)
}

fn generic_call_forwarder(environment: Environment, chain: &NamedChain) -> Option<Address> {
    let environment = match environment {
        Environment::Staging => GenericCallEnvironment::Staging,
        Environment::Production => GenericCallEnvironment::Production,
    };
    generic_call_forwarder_address(environment, chain)
}

/// Lists every recorded forwarder that accepts calls from another protocol adapter than the proxy recorded for its
/// chain and environment.
async fn forwarders_of_another_protocol_adapter(
    forwarder_of: fn(Environment, &NamedChain) -> Option<Address>,
) -> Result<Vec<String>> {
    let mut mismatches = Vec::new();
    for environment in [Environment::Staging, Environment::Production] {
        for (chain, protocol_adapter) in protocol_adapter_deployments_map(environment) {
            let Some(forwarder) = forwarder_of(environment, &chain) else {
                continue;
            };
            let provider = provider(&Caip2ChainId::from(chain))?;
            let accepted = IForwarder::new(forwarder, &provider)
                .getProtocolAdapter()
                .call()
                .await
                .with_context(|| format!("{environment:?} {chain}: forwarder {forwarder}"))?;
            if accepted != protocol_adapter {
                mismatches.push(format!(
                    "{environment:?} {chain}: forwarder {forwarder} accepts calls from {accepted}, not from the recorded protocol adapter {protocol_adapter}"
                ));
            }
        }
    }
    Ok(mismatches)
}

#[tokio::test]
async fn the_recorded_erc20_forwarders_accept_calls_from_the_recorded_protocol_adapters()
-> Result<()> {
    let mismatches = forwarders_of_another_protocol_adapter(erc20_forwarder).await?;
    ensure!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    Ok(())
}

#[tokio::test]
async fn the_recorded_generic_call_forwarders_accept_calls_from_the_recorded_protocol_adapters()
-> Result<()> {
    let mismatches = forwarders_of_another_protocol_adapter(generic_call_forwarder).await?;
    ensure!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    Ok(())
}
