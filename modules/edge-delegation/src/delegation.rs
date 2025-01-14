// Copyright 2018 Commonwealth Labs, Inc.
// This file is part of Edgeware.

// Edgeware is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Edgeware is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Edgeware.  If not, see <http://www.gnu.org/licenses/>.

#[cfg(feature = "std")]
extern crate serde;

// Needed for deriving `Serialize` and `Deserialize` for various types.
// We only implement the serde traits for std builds - they're unneeded
// in the wasm runtime.
#[cfg(feature = "std")]

extern crate parity_codec as codec;
extern crate substrate_primitives as primitives;
extern crate sr_std as rstd;
extern crate srml_support as runtime_support;
extern crate sr_primitives as runtime_primitives;
extern crate sr_io as runtime_io;
extern crate srml_system as system;

use rstd::prelude::*;
use system::ensure_signed;
use runtime_support::{StorageMap};
use runtime_support::dispatch::Result;

pub trait Trait: system::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		/// A function which delegates the sender's vote to a target account.
		///
		/// Delegates the vote for all coin-weighted or individual identity based votes.
		pub fn delegate_to(origin, to: T::AccountId) -> Result {
			let _sender = ensure_signed(origin)?;
			// Check that no delegation cycle exists and that the depth is valid
			ensure!(!Self::is_invalid_delegation(&_sender, to.clone(), 1), "Invalid delegation");
			// Update the delegate of _sender -> Some(to)
			<DelegatesOf<T>>::insert(&_sender, &to);
			// Update the delegates of to to include _sender
			if let Some(mut delegates) = <DelegatesTo<T>>::get(to.clone()) {
				delegates.push(_sender.clone());
				<DelegatesTo<T>>::insert(to.clone(), delegates);
			} else {
				<DelegatesTo<T>>::insert(to.clone(), vec![_sender.clone()]);
			}
			
			// Fire delegation event
			Self::deposit_event(RawEvent::Delegated(_sender, to));

			Ok(())
		}

		/// A function which undelegates a sender's vote from an account
		///
		/// Undelegates a sender's vote and removes the backlink from the delegation graph.
		pub fn undelegate_from(origin, from: T::AccountId) -> Result {
			let _sender = ensure_signed(origin)?;
			// Check sender is not delegating to itself
			ensure!(_sender != from, "Invalid undelegation");
			// Update the delegate to the sender, None type throws an error due to missing Trait bound
			<DelegatesOf<T>>::remove(&_sender);
			// Update the delegates to remove _sender (i.e. remove backlink)
			if let Some(mut delegates) = <DelegatesTo<T>>::get(from.clone()) {
				let index = delegates.iter().position(|d| d == &_sender.clone()).unwrap();
				delegates.remove(index);

				if delegates.len() == 0 {
					<DelegatesTo<T>>::remove(from.clone());
				} else {
					<DelegatesTo<T>>::insert(from.clone(), delegates);	
				}
			}
			// Fire delegation event
			Self::deposit_event(RawEvent::Undelegated(_sender, from));

			Ok(())
		}
	}
}

impl<T: Trait> Module<T> {
	/// Implement rudimentary DFS to find if "to"'s delegation ever leads to "from"
	pub fn is_invalid_delegation(from: &T::AccountId, to: T::AccountId, length: u32) -> bool {
		// If length is greater than allowed depth, reject
		if length > Self::delegation_depth() {
			return true;
		}
		// Loop over delegation path of "to" to check if "from" exists
		if from == &to {
			return true;
		}
		match Self::delegate_of(&to) {
			Some(delegate) => Self::is_invalid_delegation(from, delegate, length + 1),
			None => false,
		}
	}

	/// Get the last node at the end of a delegation path for a given account
	pub fn get_sink_delegator(start: T::AccountId) -> T::AccountId {
		match Self::delegate_of(&start) {
			Some(delegate) => Self::get_sink_delegator(delegate),
			None => start,
		}
	}

	/// Tallies the "sink" delegators along a delegation path for each account
	pub fn tally_delegation(accounts: Vec<T::AccountId>) -> Vec<(T::AccountId, T::AccountId)> {
		accounts.into_iter()
			.map(|a| (a.clone(), Self::get_sink_delegator(a)))
			.collect()
	}
}

decl_event!(
	pub enum Event<T> where <T as system::Trait>::AccountId {
		/// (delegator, target) when a delegator delegates to a target
		Delegated(AccountId, AccountId),
		/// (delegator, target) when an account undelegates from a target
		Undelegated(AccountId, AccountId),
	}
);

decl_storage! {
	trait Store for Module<T: Trait> as Delegation {
		pub DelegationDepth get(delegation_depth) config(): u32;
		/// The map of strict delegates for each account
		pub DelegatesOf get(delegate_of): map T::AccountId => Option<T::AccountId>;
		/// The map of accounts delegating to a specific account
		pub DelegatesTo get(delegates_to): map T::AccountId => Option<Vec<T::AccountId>>;
	}
}
