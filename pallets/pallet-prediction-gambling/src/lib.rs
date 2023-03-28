#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet_timestamp as timestamp;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;


#[frame_support::pallet]
pub mod pallet {
	use frame_support::{log, pallet_prelude::*, PalletId, sp_runtime::traits::AccountIdConversion, traits::{Currency, LockableCurrency}};
	// use frame_support::sp_runtime::Saturating;
	use frame_support::traits::ExistenceRequirement;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type Currency: LockableCurrency<Self::AccountId, Moment=Self::BlockNumber>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type EpochBlockLength: Get<u32>;

	}

	#[pallet::storage]
	#[pallet::getter(fn pool_stakes)]
	pub(super) type PoolStakes<T> = StorageValue<_, BalanceOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn last_start_time)]
	pub(super) type EpochBlockNumberStartTime<T> = StorageValue<_, <T as frame_system::Config>::BlockNumber>;

	#[pallet::storage]
	#[pallet::getter(fn current_bets)]
	pub(super) type Bets<T: Config> = StorageMap<
		_,
		// Blake2_128Concat, T::Currency,
		Blake2_128Concat, T::AccountId,
		(bool, BalanceOf<T>, <T as frame_system::Config>::BlockNumber), // Storing for a wallet if they're bull / bear & their bet amount.
		ValueQuery
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		PoolInitialized { block_number: T::BlockNumber },
		PoolBlockNumberUpdated { block_number: T::BlockNumber },
		UserBet { amount: BalanceOf<T>, who: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		/// Wallet has already set a bet within the current epoch.
		AlreadyBet,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Updates pallet storage current block number, and runs the epoch end handler if necessary.
		/// ToDo: Update with an automated backend handler
		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1, 1).ref_time())]
		pub fn update_block_number(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			let epoch_start_time = <EpochBlockNumberStartTime<T>>::get();
			if current_block_number >
				epoch_start_time.unwrap() + (T::EpochBlockLength::get().into()) {
				log::info!("Prediction Gambling: Detected epoch end! Starting end round handler...");
				_ = Self::handle_epoch_end();
			}
			<EpochBlockNumberStartTime<T>>::put(current_block_number);
			Self::deposit_event(Event::PoolBlockNumberUpdated { block_number: current_block_number });
			Ok(())
		}


		/// Extrinsic for a user to stake their bet into the current pool epoch.
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1, 1).ref_time())]
		pub fn bet(origin: OriginFor<T>, amount: BalanceOf<T>, bull: bool) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// ToDo: Move into own function to clean up code!
			let epoch_start_time = <EpochBlockNumberStartTime<T>>::get();
			if epoch_start_time.is_none() {
				let current_block_number = <frame_system::Pallet<T>>::block_number();
				<EpochBlockNumberStartTime<T>>::put(&current_block_number);
				Self::deposit_event(Event::PoolInitialized { block_number: current_block_number });
			}
			let new_pool_stakes =
				T::Currency::free_balance(&Self::account_id()) + amount;

			// Making sure user has only 1 bet per epoch & setting it.
			match <Bets<T>>::try_get(&who) {
				Ok(_) => {return Err(Error::<T>::AlreadyBet.into())} // User has already bet
				Err(_) => {
					let current_epoch = <EpochBlockNumberStartTime<T>>::get();
					let user_bet = (bull, amount.clone(), current_epoch.unwrap());
					<Bets<T>>::set(&who, user_bet);
				}
			}

			// Transferring amount to pallet wallet
			T::Currency::transfer(
				&who,
				&Self::account_id(),
				amount,
				ExistenceRequirement::AllowDeath,
			)?;
			<PoolStakes<T>>::put(new_pool_stakes);
			Self::deposit_event(Event::UserBet { amount, who });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn account_id() -> <T as frame_system::Config>::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		pub fn handle_epoch_end() -> DispatchResult {
			log::info!("Prediction Gambling: Finished epoch end handler");
			Ok(())
		}
	}
}
