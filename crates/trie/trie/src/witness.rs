use crate::{
    hashed_cursor::HashedCursorFactory, prefix_set::TriePrefixSetsMut, proof::Proof,
    trie_cursor::TrieCursorFactory, HashedPostState,
};
use alloy_primitives::{keccak256, Bytes, B256};
use alloy_rlp::{BufMut, Decodable, Encodable};
use itertools::Either;
use reth_execution_errors::{StateProofError, TrieWitnessError};
use reth_primitives::constants::EMPTY_ROOT_HASH;
use reth_trie_common::{
    BranchNode, BranchNodeCompact, HashBuilder, Nibbles, TrieAccount, TrieNode, CHILD_INDEX_RANGE,
};
use std::collections::{BTreeMap, HashMap, HashSet};

/// State transition witness for the trie.
#[derive(Debug)]
pub struct TrieWitness<T, H> {
    /// The cursor factory for traversing trie nodes.
    trie_cursor_factory: T,
    /// The factory for hashed cursors.
    hashed_cursor_factory: H,
    /// A set of prefix sets that have changes.
    prefix_sets: TriePrefixSetsMut,
    /// Recorded witness.
    witness: HashMap<B256, Bytes>,
}

impl<T, H> TrieWitness<T, H> {
    /// Creates a new witness generator.
    pub fn new(trie_cursor_factory: T, hashed_cursor_factory: H) -> Self {
        Self {
            trie_cursor_factory,
            hashed_cursor_factory,
            prefix_sets: TriePrefixSetsMut::default(),
            witness: HashMap::default(),
        }
    }

    /// Set the trie cursor factory.
    pub fn with_trie_cursor_factory<TF>(self, trie_cursor_factory: TF) -> TrieWitness<TF, H> {
        TrieWitness {
            trie_cursor_factory,
            hashed_cursor_factory: self.hashed_cursor_factory,
            prefix_sets: self.prefix_sets,
            witness: self.witness,
        }
    }

    /// Set the hashed cursor factory.
    pub fn with_hashed_cursor_factory<HF>(self, hashed_cursor_factory: HF) -> TrieWitness<T, HF> {
        TrieWitness {
            trie_cursor_factory: self.trie_cursor_factory,
            hashed_cursor_factory,
            prefix_sets: self.prefix_sets,
            witness: self.witness,
        }
    }

    /// Set the prefix sets. They have to be mutable in order to allow extension with proof target.
    pub fn with_prefix_sets_mut(mut self, prefix_sets: TriePrefixSetsMut) -> Self {
        self.prefix_sets = prefix_sets;
        self
    }
}

impl<T, H> TrieWitness<T, H>
where
    T: TrieCursorFactory + Clone,
    H: HashedCursorFactory + Clone,
{
    /// Compute the state transition witness for the trie. Gather all required nodes
    /// to apply `state` on top of the current trie state.
    ///
    /// # Arguments
    ///
    /// `state` - state transition containing both modified and touched accounts and storage slots.
    pub fn compute(
        mut self,
        state: HashedPostState,
    ) -> Result<HashMap<B256, Bytes>, TrieWitnessError> {
        let proof_targets = HashMap::from_iter(
            state
                .accounts
                .keys()
                .map(|hashed_address| (*hashed_address, HashSet::default()))
                .chain(state.storages.iter().map(|(hashed_address, storage)| {
                    (*hashed_address, storage.storage.keys().copied().collect())
                })),
        );
        let mut account_multiproof =
            Proof::new(self.trie_cursor_factory.clone(), self.hashed_cursor_factory.clone())
                .with_prefix_sets_mut(self.prefix_sets.clone())
                .with_targets(proof_targets.clone())
                .multiproof()?;

        // Attempt to compute state root from proofs and gather additional
        // information for the witness.
        let mut account_rlp = Vec::with_capacity(128);
        let mut account_trie_nodes = BTreeMap::default();
        for (hashed_address, hashed_slots) in proof_targets {
            let storage_multiproof =
                account_multiproof.storages.remove(&hashed_address).unwrap_or_default();

            // Gather and record account trie nodes.
            let account = state
                .accounts
                .get(&hashed_address)
                .ok_or(TrieWitnessError::MissingAccount(hashed_address))?;
            let value = if account.is_some() || storage_multiproof.root != EMPTY_ROOT_HASH {
                account_rlp.clear();
                TrieAccount::from((account.unwrap_or_default(), storage_multiproof.root))
                    .encode(&mut account_rlp as &mut dyn BufMut);
                Some(account_rlp.clone())
            } else {
                None
            };
            let key = Nibbles::unpack(hashed_address);
            let proof = account_multiproof.account_subtree.iter().filter(|e| key.starts_with(e.0));
            account_trie_nodes.extend(target_nodes(
                key.clone(),
                value,
                proof,
                Some(&mut self.witness),
            )?);

            // Gather and record storage trie nodes for this account.
            let mut storage_trie_nodes = BTreeMap::default();
            let storage = state.storages.get(&hashed_address);
            for hashed_slot in hashed_slots {
                let slot_key = Nibbles::unpack(hashed_slot);
                let slot_value = storage
                    .and_then(|s| s.storage.get(&hashed_slot))
                    .filter(|v| !v.is_zero())
                    .map(|v| alloy_rlp::encode_fixed_size(v).to_vec());
                let proof = storage_multiproof.subtree.iter().filter(|e| slot_key.starts_with(e.0));
                storage_trie_nodes.extend(target_nodes(
                    slot_key.clone(),
                    slot_value,
                    proof,
                    Some(&mut self.witness),
                )?);
            }

            let (storage_root, _) =
                next_root_from_proofs(storage_trie_nodes, false, |key: Nibbles| {
                    // Right pad the target with 0s.
                    let mut padded_key = key.pack();
                    padded_key.resize(32, 0);
                    let target_key = B256::from_slice(&padded_key);
                    let mut proof = Proof::new(
                        self.trie_cursor_factory.clone(),
                        self.hashed_cursor_factory.clone(),
                    )
                    .with_prefix_sets_mut(self.prefix_sets.clone())
                    .with_target((hashed_address, HashSet::from([target_key])))
                    .storage_multiproof(hashed_address)?;

                    // The subtree only contains the proof for a single target.
                    let node = proof
                        .subtree
                        .remove(&key)
                        .ok_or(TrieWitnessError::MissingTargetNode(key))?;
                    self.witness.insert(keccak256(node.as_ref()), node.clone()); // record in witness
                    Ok(node)
                })?;
            debug_assert_eq!(storage_multiproof.root, storage_root);
        }

        next_root_from_proofs(account_trie_nodes, false, |key: Nibbles| {
            // Right pad the target with 0s.
            let mut padded_key = key.pack();
            padded_key.resize(32, 0);
            let mut proof =
                Proof::new(self.trie_cursor_factory.clone(), self.hashed_cursor_factory.clone())
                    .with_prefix_sets_mut(self.prefix_sets.clone())
                    .with_target((B256::from_slice(&padded_key), HashSet::default()))
                    .multiproof()?;

            // The subtree only contains the proof for a single target.
            let node = proof
                .account_subtree
                .remove(&key)
                .ok_or(TrieWitnessError::MissingTargetNode(key))?;
            self.witness.insert(keccak256(node.as_ref()), node.clone()); // record in witness
            Ok(node)
        })?;

        Ok(self.witness)
    }
}

/// Returned branch node children with keys in order.
fn branch_node_children(prefix: Nibbles, node: &BranchNode) -> Vec<(Nibbles, B256)> {
    let mut children = Vec::with_capacity(node.state_mask.count_ones() as usize);
    let mut stack_ptr = node.as_ref().first_child_index();
    for index in CHILD_INDEX_RANGE {
        if node.state_mask.is_bit_set(index) {
            let mut child_path = prefix.clone();
            child_path.push(index);
            children.push((child_path, B256::from_slice(&node.stack[stack_ptr][1..])));
            stack_ptr += 1;
        }
    }
    children
}

/// Decodes and unrolls all nodes from the proof. Returns only sibling nodes
/// in the path of the target and the final leaf node with updated value.
pub fn target_nodes<'b>(
    key: Nibbles,
    value: Option<Vec<u8>>,
    proof: impl IntoIterator<Item = (&'b Nibbles, &'b Bytes)>,
    mut witness: Option<&mut HashMap<B256, Bytes>>,
) -> Result<BTreeMap<Nibbles, Either<B256, Vec<u8>>>, StateProofError> {
    let mut trie_nodes = BTreeMap::default();
    for (path, encoded) in proof {
        // Record the node in witness.
        if let Some(witness) = &mut witness {
            witness.insert(keccak256(encoded.as_ref()), encoded.clone());
        }

        let mut next_path = path.clone();
        match TrieNode::decode(&mut &encoded[..])? {
            TrieNode::Branch(branch) => {
                next_path.push(key[path.len()]);
                let children = branch_node_children(path.clone(), &branch);
                for (child_path, node_hash) in children {
                    if !key.starts_with(&child_path) {
                        trie_nodes.insert(child_path, Either::Left(node_hash));
                    }
                }
            }
            TrieNode::Extension(extension) => {
                next_path.extend_from_slice(&extension.key);
            }
            TrieNode::Leaf(leaf) => {
                next_path.extend_from_slice(&leaf.key);
                if next_path != key {
                    trie_nodes.insert(next_path.clone(), Either::Right(leaf.value.clone()));
                }
            }
        };
    }

    if let Some(value) = value {
        trie_nodes.insert(key, Either::Right(value));
    }

    Ok(trie_nodes)
}

/// TODO:
pub fn next_root_from_proofs(
    trie_nodes: BTreeMap<Nibbles, Either<B256, Vec<u8>>>,
    retain_updates: bool,
    mut trie_node_provider: impl FnMut(Nibbles) -> Result<Bytes, TrieWitnessError>,
) -> Result<(B256, revm::primitives::HashMap<Nibbles, BranchNodeCompact>), TrieWitnessError> {
    // Ignore branch child hashes in the path of leaves or lower child hashes.
    let mut keys = trie_nodes.keys().peekable();
    let mut ignored = HashSet::<Nibbles>::default();
    while let Some(key) = keys.next() {
        if keys.peek().map_or(false, |next| next.starts_with(key)) {
            ignored.insert(key.clone());
        }
    }

    let mut hash_builder = HashBuilder::default().with_updates(retain_updates);
    let mut trie_nodes = trie_nodes.into_iter().filter(|e| !ignored.contains(&e.0)).peekable();
    while let Some((path, value)) = trie_nodes.next() {
        match value {
            Either::Left(branch_hash) => {
                let parent_branch_path = path.slice(..path.len() - 1);
                if hash_builder.key.starts_with(&parent_branch_path) ||
                    trie_nodes
                        .peek()
                        .map_or(false, |next| next.0.starts_with(&parent_branch_path))
                {
                    hash_builder.add_branch(path, branch_hash, false);
                } else {
                    // Parent is a branch node that needs to be turned into an extension node.
                    let mut path = path.clone();
                    loop {
                        let node = trie_node_provider(path.clone())?;
                        match TrieNode::decode(&mut &node[..])? {
                            TrieNode::Branch(branch) => {
                                let children = branch_node_children(path, &branch);
                                for (child_path, branch_hash) in children {
                                    hash_builder.add_branch(child_path, branch_hash, false);
                                }
                                break
                            }
                            TrieNode::Leaf(leaf) => {
                                let mut child_path = path;
                                child_path.extend_from_slice(&leaf.key);
                                hash_builder.add_leaf(child_path, &leaf.value);
                                break
                            }
                            TrieNode::Extension(ext) => {
                                path.extend_from_slice(&ext.key);
                            }
                        }
                    }
                }
            }
            Either::Right(leaf_value) => {
                hash_builder.add_leaf(path, &leaf_value);
            }
        }
    }

    let root = hash_builder.root();
    let (_, updates) = hash_builder.split();
    Ok((root, updates))
}
