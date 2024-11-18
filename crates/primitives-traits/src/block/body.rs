//! Block body abstraction.

use alloc::fmt;

use reth_codecs::Compact;

use crate::{FullSignedTx, InMemorySize, MaybeSerde, SignedTransaction};

/// Helper trait that unifies all behaviour required by transaction to support full node operations.
pub trait FullBlockBody: BlockBody<Transaction: FullSignedTx> + Compact {}

impl<T> FullBlockBody for T where T: BlockBody<Transaction: FullSignedTx> + Compact {}

/// Abstraction for block's body.
#[auto_impl::auto_impl(&, Arc)]
pub trait BlockBody:
    Send
    + Sync
    + Unpin
    + Clone
    + Default
    + fmt::Debug
    + PartialEq
    + Eq
    + alloy_rlp::Encodable
    + alloy_rlp::Decodable
    + InMemorySize
    + MaybeSerde
{
    /// Signed transaction, committed in block.
    type Transaction: SignedTransaction;

    /// Returns reference to transactions in block.
    fn transactions(&self) -> &[Self::Transaction];
}
