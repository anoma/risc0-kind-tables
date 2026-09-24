//! On-chain validation of `tokens.json`: every supported token exists and reports the recorded identity — an
//! ERC20 contract its symbol, name and decimals; an SPL token mint its decimals.
//! Runs on every pull request and push, so a bad address never merges anywhere.

use alloy::providers::Provider;
use alloy::sol;
use anoma_risc0_kind_tables::{Chain, ChainTokens, tokens};
use anoma_risc0_kind_tables_integration_test::{provider, solana_rpc};
use anyhow::{Context, Result, ensure};

sol! {
    #[sol(rpc)]
    interface IERC20Metadata {
        function name() external view returns (string);
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
    }
}

#[tokio::test]
async fn every_supported_token_reports_its_recorded_identity() -> Result<()> {
    for (&chain, supported) in tokens::all() {
        match (chain, supported) {
            (Chain::Evm(named), ChainTokens::Erc20(supported)) => {
                let provider = provider(named)?;
                for token in supported {
                    let context = || format!("{} ({}) on {chain}", token.symbol, token.address);
                    let code = provider
                        .get_code_at(token.address)
                        .await
                        .with_context(context)?;
                    ensure!(!code.is_empty(), "no contract at {}", context());
                    let contract = IERC20Metadata::new(token.address, &provider);
                    let symbol = contract.symbol().call().await.with_context(context)?;
                    let name = contract.name().call().await.with_context(context)?;
                    let decimals = contract.decimals().call().await.with_context(context)?;
                    ensure!(symbol == token.symbol, "{}: symbol is {symbol}", context());
                    ensure!(name == token.name, "{}: name is {name}", context());
                    ensure!(
                        decimals == token.decimals,
                        "{}: decimals is {decimals}",
                        context()
                    );
                }
            }
            // A mint reports only its decimals on chain; symbol and name are review context.
            (Chain::Solana(cluster), ChainTokens::Spl(supported)) => {
                let rpc = solana_rpc(cluster);
                for token in supported {
                    let context = || format!("{} ({}) on {chain}", token.symbol, token.mint);
                    let decimals = rpc
                        .mint_decimals(&token.mint)
                        .await
                        .with_context(context)?
                        .with_context(|| format!("no mint account at {}", context()))?;
                    ensure!(
                        decimals == token.decimals,
                        "{}: decimals is {decimals}",
                        context()
                    );
                }
            }
            (chain, _) => anyhow::bail!("{chain}: tokens of the other chain kind"),
        }
    }
    Ok(())
}
