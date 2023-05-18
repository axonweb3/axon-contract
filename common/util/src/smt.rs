pub fn verify_2layer_smt_stake(
    stake_infos: &BTreeSet<StakeInfoObject>,
    epoch: u64,
    epoch_proof: &Vec<u8>,
    epoch_root: &[u8; 32],
) -> Result<(), Error> {
    // construct old stake smt root & verify
    let mut tree_buf = [Pair::default(); 100];
    let mut tree = Tree::new(&mut tree_buf);
    stake_infos.iter().for_each(|stake_info| {
        let _ = tree
            .update(
                &bytes_to_h256(&stake_info.identity.to_vec()),
                &bytes_to_h256(&stake_info.stake_amount.to_le_bytes().to_vec()),
            )
            .map_err(|err| {
                debug!("update smt tree error: {}", err);
                Error::MerkleProof
            });
    });

    let proof = [0u8; 32];
    let stake_root = tree.calculate_root(&proof)?; // epoch smt value

    let mut tree_buf = [Pair::default(); 100];
    let mut epoch_tree = Tree::new(&mut tree_buf[..]);
    epoch_tree
        .update(&bytes_to_h256(&epoch.to_le_bytes().to_vec()), &stake_root)
        .map_err(|err| {
            debug!("update smt tree error: {}", err);
            Error::MerkleProof
        })?;
    epoch_tree
        .verify(&epoch_root, &epoch_proof)
        .map_err(|err| {
            debug!("verify top smt error: {}", err);
            Error::OldStakeInfosErr
        })?;
    Ok(())
}

pub fn verify_2layer_smt_delegate(
    delegate_infos: &BTreeSet<DelegateInfoObject>,
    epoch: u64,
    epoch_proof: &Vec<u8>,
    epoch_root: &[u8; 32],
) -> Result<(), Error> {
    // construct old stake smt root & verify
    let mut tree_buf = [Pair::default(); 100];
    let mut tree = Tree::new(&mut tree_buf);
    delegate_infos.iter().for_each(|stake_info| {
        let _ = tree
            .update(
                &bytes_to_h256(&stake_info.addr.to_vec()),
                &bytes_to_h256(&stake_info.amount.to_le_bytes().to_vec()),
            )
            .map_err(|err| {
                debug!("update smt tree error: {}", err);
                Error::MerkleProof
            });
    });

    let proof = [0u8; 32];
    let stake_root = tree.calculate_root(&proof)?; // epoch smt value

    let mut tree_buf = [Pair::default(); 100];
    let mut epoch_tree = Tree::new(&mut tree_buf[..]);
    epoch_tree
        .update(&bytes_to_h256(&epoch.to_le_bytes().to_vec()), &stake_root)
        .map_err(|err| {
            debug!("update smt tree error: {}", err);
            Error::MerkleProof
        })?;
    epoch_tree
        .verify(&epoch_root, &epoch_proof)
        .map_err(|err| {
            debug!("verify top smt error: {}", err);
            Error::OldStakeInfosErr
        })?;
    Ok(())
}

// pub fn verify_2layer_smt(stake_proof: MerkleProof, stake_root: H256, staker_identity: Vec<u8>, old_stake: u128,
//                          epoch_proof: MerkleProof, epoch_root: H256, epoch: u64) -> Result<(), Error> {
//     if verify_smt(stake_proof, &stake_root, staker_identity.to_h256(), old_stake.to_h256()) {
//         return Err(Error::IllegalInputStakeInfo);
//     }

//     if verify_smt(epoch_proof, &epoch_root, epoch.to_h256(), stake_root) {
//         Err(Error::IllegalInputStakeInfo)
//     } else {
//         Ok(())
//     }
// }

// pub fn verify_smt(proof: MerkleProof, root: &H256, key: H256, value: H256) -> bool {
//     let leaves = vec![(key, value)];
//     proof.verify::<Blake2bHasher>(root, leaves).unwrap()
// }
