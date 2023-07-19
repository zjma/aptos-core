// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    modifiers::create_swarm_cpu_stress,
    multi_region_network_test::create_multi_region_swarm_network_chaos, LoadDestination,
    NetworkLoadTest,
};
use anyhow::Error;
use aptos_forge::{
    NetworkContext, NetworkTest, Result, Swarm, SwarmChaos, SwarmCpuStress, SwarmNetEm, Test,
};
use aptos_logger::info;
use aptos_sdk::move_types::account_address::AccountAddress;
use aptos_types::PeerId;
use rand::{
    rngs::{OsRng, StdRng},
    seq::SliceRandom,
    Rng, SeedableRng,
};
use tokio::runtime::Runtime;

/// A simple test that adds multiple public fullnodes (PFNs) to the swarm
/// and submits transactions through them. Network emulation chaos can also
/// be configured for all nodes in the swarm.
#[derive(Default)]
pub struct PFNPerformance {
    add_cpu_chaos: bool,
    add_network_emulation: bool,
    shuffle_rng_seed: [u8; 32],
}

impl PFNPerformance {
    pub fn new(add_cpu_chaos: bool, add_network_emulation: bool) -> Self {
        // Create a random seed for the shuffle RNG
        let shuffle_rng_seed: [u8; 32] = OsRng.gen();

        Self {
            add_cpu_chaos,
            add_network_emulation,
            shuffle_rng_seed,
        }
    }

    /// Creates CPU chaos for the swarm. Note: CPU chaos is added
    /// to all validators, VFNs and PFNs in the swarm.
    fn create_cpu_chaos(&self, swarm: &mut dyn Swarm) -> SwarmCpuStress {
        // Gather and shuffle all peers IDs (so that we get random CPU chaos)
        let shuffled_peer_ids = self.gather_and_shuffle_peer_ids(swarm);

        // Create CPU chaos for the swarm
        create_swarm_cpu_stress(shuffled_peer_ids, None)
    }

    /// Creates network emulation chaos for the swarm. Note: network chaos
    /// is added to all validators, VFNs and PFNs in the swarm.
    fn create_network_emulation_chaos(&self, swarm: &mut dyn Swarm) -> SwarmNetEm {
        // Gather and shuffle all peers IDs (so that we get random network emulation)
        let shuffled_peer_ids = self.gather_and_shuffle_peer_ids(swarm);

        // Create network emulation chaos for the swarm
        // TODO: VFNs and VNs need to be colocated
        let shuffled_peer_ids = shuffled_peer_ids.iter().map(|id| vec![*id]).collect();
        create_multi_region_swarm_network_chaos(shuffled_peer_ids, None)
    }

    /// Gathers and shuffles all peer IDs in the swarm
    fn gather_and_shuffle_peer_ids(&self, swarm: &mut dyn Swarm) -> Vec<AccountAddress> {
        // Identify the validators and fullnodes in the swarm
        let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
        let fullnode_peer_ids = swarm.full_nodes().map(|v| v.peer_id()).collect::<Vec<_>>();

        // Gather and shuffle all peers IDs
        let mut all_peer_ids = validator_peer_ids
            .iter()
            .chain(fullnode_peer_ids.iter())
            .cloned()
            .collect::<Vec<_>>();
        all_peer_ids.shuffle(&mut StdRng::from_seed(self.shuffle_rng_seed));

        all_peer_ids
    }
}

impl Test for PFNPerformance {
    fn name(&self) -> &'static str {
        "PFNPerformance"
    }
}

impl NetworkTest for PFNPerformance {
    fn run(&self, ctx: &mut NetworkContext<'_>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}

impl NetworkLoadTest for PFNPerformance {
    /// We must override the setup function to: (i) create PFNs in
    /// the swarm; and (ii) use those PFNs as the load destination.
    fn setup(&self, ctx: &mut NetworkContext) -> Result<LoadDestination> {
        // Add the PFNs to the swarm
        let num_pfns = 7;
        let pfn_peer_ids = create_and_add_pfns(ctx, num_pfns)?;

        // Add CPU chaos to the swarm
        if self.add_cpu_chaos {
            let cpu_chaos = self.create_cpu_chaos(ctx.swarm());
            ctx.swarm().inject_chaos(SwarmChaos::CpuStress(cpu_chaos))?;
        }

        // Add network emulation to the swarm
        if self.add_network_emulation {
            let network_chaos = self.create_network_emulation_chaos(ctx.swarm());
            ctx.swarm().inject_chaos(SwarmChaos::NetEm(network_chaos))?;
        }

        // Use the PFNs as the load destination
        Ok(LoadDestination::Peers(pfn_peer_ids))
    }

    fn finish(&self, swarm: &mut dyn Swarm) -> Result<()> {
        // Remove CPU chaos from the swarm
        if self.add_cpu_chaos {
            let cpu_chaos = self.create_cpu_chaos(swarm);
            swarm.remove_chaos(SwarmChaos::CpuStress(cpu_chaos))?;
        }

        // Remove network emulation from the swarm
        if self.add_network_emulation {
            let network_chaos = self.create_network_emulation_chaos(swarm);
            swarm.remove_chaos(SwarmChaos::NetEm(network_chaos))?;
        }

        Ok(())
    }
}

/// Adds a number of PFNs to the network and returns the peer IDs
fn create_and_add_pfns(ctx: &mut NetworkContext, num_pfns: u64) -> Result<Vec<PeerId>, Error> {
    info!("Creating {} public fullnodes!", num_pfns);

    // Identify the version for the PFNs
    let swarm = ctx.swarm();
    let pfn_version = swarm.versions().max().unwrap();

    // Create the PFN swarm
    let runtime = Runtime::new().unwrap();
    let pfn_peer_ids: Vec<AccountAddress> = (0..num_pfns)
        .map(|i| {
            // Create a config for the PFN. Note: this needs to be done here
            // because the config will generate a unique peer ID for the PFN.
            let pfn_config = swarm.get_default_pfn_node_config();

            // Add the PFN to the swarm
            let peer_id = runtime
                .block_on(swarm.add_full_node(&pfn_version, pfn_config))
                .unwrap();

            // Verify the PFN was added
            if swarm.full_node(peer_id).is_none() {
                panic!(
                    "Failed to locate PFN {:?} in the swarm! Peer ID: {:?}",
                    i, peer_id
                );
            }

            // Return the peer ID
            info!("Created PFN {:?} with peer ID: {:?}", i, peer_id);
            peer_id
        })
        .collect();

    Ok(pfn_peer_ids)
}
