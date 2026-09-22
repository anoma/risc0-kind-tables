//! On-chain validation of the Solana sections of `tokens.json`: every supported mint exists, is an SPL token mint,
//! and reports the recorded decimals. A mint carries no symbol or name on chain; those are review context. Runs on
//! every pull request and push, so a bad mint never merges anywhere.
use anoma_risc0_kind_tables::{ChainTokens, SolanaCluster, tokens};
use anoma_risc0_kind_tables_integration_test::solana_rpc;
use anyhow::{Context, Result, ensure};

#[tokio::test]
async fn every_supported_mint_reports_its_recorded_decimals() -> Result<()> {
    for (chain, supported) in tokens::all() {
        let ChainTokens::Spl(supported) = supported else {
            continue; // ERC20 tokens are validated over their chain's RPC by the EVM token validation test.
        };
        let cluster = SolanaCluster::try_from(chain)?;
        let rpc = solana_rpc(cluster);
        for token in supported {
            let context = || format!("{} ({}) on {}", token.symbol, token.mint, cluster.name());
            let mint = rpc
                .mint(&token.mint)
                .await
                .with_context(context)?
                .with_context(|| format!("no mint account at {}", context()))?;
            ensure!(
                mint.decimals == token.decimals,
                "{}: decimals is {}",
                context(),
                mint.decimals
            );
        }
    }
    Ok(())
}
