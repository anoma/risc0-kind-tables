use super::*;

#[test]
fn the_embedded_list_carries_the_devnet_test_mint_under_its_caip2_key() {
    let [token] = spl_on(SolanaCluster::Devnet) else {
        panic!("expected exactly one supported mint on solana-devnet");
    };
    assert_eq!(
        token.mint.to_string(),
        "9EHEFzyuY7sZEzTVm7C3uMkNZFMgm5ZeWjGjirZ3MVfr"
    );
    assert_eq!(token.decimals, 6);
    assert!(
        on(SolanaCluster::Devnet).is_empty(),
        "a Solana chain has no ERC20 tokens"
    );
}
