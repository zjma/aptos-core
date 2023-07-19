// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account::create::DEFAULT_FUNDED_COINS,
    common::{
        types::{CliCommand, CliTypedResult, FaucetOptions, ProfileOptions, RestOptions},
        utils::{fund_account, wait_for_transactions},
    },
};
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;

/// Fund an account with tokens from a faucet
///
/// This will create an account if it doesn't exist with the faucet.  This is mostly useful
/// for local development and devnet.
#[derive(Debug, Parser)]
pub struct FundWithFaucet {
    /// Address to fund
    ///
    /// If the account wasn't previously created, it will be created when being funded
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub(crate) account: Option<AccountAddress>,

    /// Number of Octas to fund the account from the faucet
    ///
    /// The amount added to the account may be limited by the faucet, and may be less
    /// than the amount requested.
    #[clap(long, default_value_t = DEFAULT_FUNDED_COINS)]
    pub(crate) amount: u64,

    #[clap(flatten)]
    pub(crate) faucet_options: FaucetOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
}

#[async_trait]
impl CliCommand<String> for FundWithFaucet {
    fn command_name(&self) -> &'static str {
        "FundWithFaucet"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let address = if let Some(account) = self.account {
            account
        } else {
            self.profile_options.account_address()?
        };
        let hashes = fund_account(
            self.faucet_options.faucet_url(&self.profile_options)?,
            self.amount,
            address,
        )
        .await?;
        let client = self.rest_options.client(&self.profile_options)?;
        wait_for_transactions(&client, hashes).await?;
        return Ok(format!(
            "Added {} Octas to account {}",
            self.amount, address
        ));
    }
}
