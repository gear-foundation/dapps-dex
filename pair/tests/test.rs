use primitive_types::U256;
use utils::{prelude::*, FungibleToken};

mod utils;

#[test]
fn swaps() {
    const USERS: &[u64] = &[5];
    const INIT_AMOUNT: u128 = 10000;
    const INIT_LIQ: u128 = INIT_AMOUNT / 2;
    const CLEAN_INIT_LIQ: u128 = INIT_AMOUNT / 2 - 1000;
    const SWAP_AMOUNT: u128 = 1000;
    const SPENT_BLOCKS: u32 = 1;

    let system = utils::initialize_system();

    let mut fungible_token_b = FungibleToken::initialize(&system);
    let mut fungible_token_a = FungibleToken::initialize(&system);

    // Initialization of the contracts

    let mut factory = Factory::initialize(&system, 0, 0, 3).succeed();
    let actor_pair = (fungible_token_a.actor_id(), fungible_token_b.actor_id());
    let pair_actor = factory.create_pair(actor_pair).succeed((actor_pair, 1));
    let mut pair = Pair(system.get_program(pair_actor));

    // Checking the initialization results

    factory.state().pair(actor_pair).eq(pair.actor_id());
    pair.state().factory().eq(factory.actor_id());
    pair.state().token().eq(actor_pair);

    fungible_token_a.mint(USERS[0], INIT_AMOUNT);
    fungible_token_b.mint(USERS[0], INIT_AMOUNT);
    fungible_token_a.approve(USERS[0], pair.actor_id(), INIT_LIQ + SWAP_AMOUNT);
    fungible_token_b.approve(USERS[0], pair.actor_id(), INIT_LIQ);

    // Adding liquidity

    pair.add_liquidity(USERS[0], (INIT_LIQ, INIT_LIQ), (0, 0), USERS[0])
        .succeed((USERS[0], (INIT_LIQ, INIT_LIQ), CLEAN_INIT_LIQ));

    // Checking the adding results

    pair.state().balance_of(USERS[0]).eq(CLEAN_INIT_LIQ);
    pair.state().reserve().eq((INIT_LIQ, INIT_LIQ));

    //

    let out_amount = pair
        .state()
        .calculate_out_amount(SwapKind::AForB, SWAP_AMOUNT)
        .0
        .unwrap();

    system.spend_blocks(SPENT_BLOCKS);
    pair.swap_exact_tokens_for_tokens(USERS[0], (SWAP_AMOUNT, 0), USERS[0], SwapKind::AForB)
        .succeed((
            USERS[0],
            (SWAP_AMOUNT, out_amount),
            USERS[0],
            SwapKind::AForB,
        ));

    // fungible_token_a
    //     .balance(pair.actor_id())
    //     .contains(INIT_LIQ + SWAP_AMOUNT);
    // fungible_token_b
    //     .balance(pair.actor_id())
    //     .contains(INIT_LIQ - out_amount);
    // fungible_token_a
    //     .balance(USERS[0])
    //     .contains(INIT_LIQ - SWAP_AMOUNT);
    // fungible_token_b
    //     .balance(USERS[0])
    //     .contains(INIT_LIQ + out_amount);
    // pair.state().price().eq((price, price));
    // pair.state()
    //     .reserve()
    //     .eq((INIT_LIQ + SWAP_AMOUNT, INIT_LIQ - out_amount));

    // let in_amount =

    // system.spend_blocks(1);
    // pair.swap_tokens_for_exact_tokens(USERS[0], SWAP_AMOUNT, USERS[0], SwapKind::AForB
    // ).succeed((USERS[0], (in_amoint, SWAP_AMOUNT)));

    // let amount_with_fee = SWAP_AMOUNT * 997;
    // let out_amount = amount_with_fee * INIT_LIQ / (INIT_LIQ * 1000 + amount_with_fee);
    // fungible_token_a
    //     .balance(pair.actor_id())
    //     .contains(INIT_LIQ + SWAP_AMOUNT);
    // fungible_token_b
    //     .balance(pair.actor_id())
    //     .contains(INIT_LIQ - out_amount);
    // fungible_token_a
    //     .balance(USERS[0])
    //     .contains(INIT_LIQ - SWAP_AMOUNT);
    // fungible_token_b
    //     .balance(USERS[0])
    //     .contains(INIT_LIQ + out_amount);
    // let price = (U256::from(INIT_LIQ) << U256::from(128u64)) / U256::from(INIT_LIQ)
    //     * U256::from(INIT_LIQ - CLEAN_INIT_LIQ);
    // pair.state().price().eq((price, price));
    // pair.state()
    //     .reserve()
    //     .eq((INIT_LIQ + SWAP_AMOUNT, INIT_LIQ - out_amount));
}
