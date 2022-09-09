#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*, 
		traits::{ConstU32,Randomness, Currency, ReservableCurrency},
	};
	use frame_system::pallet_prelude::*;
	use sp_io::hashing::blake2_128;
	use sp_runtime::traits::{AtLeast32BitUnsigned, Bounded};

	type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	
	#[derive(Encode,Decode,Clone,PartialEq,Eq,Debug,TypeInfo,MaxEncodedLen)]
	pub struct Kitty(pub [u8; 16]);

	#[pallet::config]
	pub trait Config:frame_system::Config{
		type Event: From<Event<Self>>+IsType<<Self as frame_system::Config>::Event>; 
		type Randomness: Randomness<Self::Hash,Self::BlockNumber>;
		type KittyIndex: Parameter
					+ AtLeast32BitUnsigned
					+ Copy
					+ Default
					+ Bounded
					+ MaxEncodedLen;
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		type KittyStake: Get<BalanceOf<Self>>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super)trait Store)] 
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn next_kitty_id)]
	pub type NextKittyId<T:Config>=StorageValue<_,T::KittyIndex>;

	#[pallet::storage]
	#[pallet::getter(fn kitties)]
	pub type Kitties<T:Config>=StorageMap<_,Blake2_128Concat,T::KittyIndex,Kitty>;

	#[pallet::storage]
	#[pallet::getter(fn kitty_owner)]
	pub type KittyOwner<T:Config>=StorageMap<_,Blake2_128Concat,T::KittyIndex,T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn owner_kitties)]
	pub type OwnerKitties<T:Config>=StorageMap<_,Blake2_128Concat,T::AccountId,BoundedVec<T::KittyIndex,ConstU32<256>>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super)fn deposit_event)] 
	pub enum Event<T:Config>{
		KittyCreated(T::AccountId,T::KittyIndex,Kitty),
		KittyBred(T::AccountId,T::KittyIndex,Kitty),
		KittyTransferred(T::AccountId,T::AccountId,T::KittyIndex),
	}

	#[pallet::error]
	pub enum Error<T> {
		InvalidId,
		NotOwner,
		SameId,
		KittiesOverflow,
		ExceedMaxOwnerKitties,
		StakeNotEnough,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn create(origin: OriginFor<T>) -> DispatchResult{
			let who=ensure_signed(origin)?;
			let kitty_id = match Self::next_kitty_id() {
				Some(id) => {
					ensure!(id != T::KittyIndex::max_value(), Error::<T>::KittiesOverflow);
					id
				}
				None => 0u32.into(),
			};

			T::Currency::reserve(&who, T::KittyStake::get()).map_err(|_| Error::<T>::StakeNotEnough)?;

			let dna= Self::random_value(&who); 
			let kitty = Kitty(dna);

			Kitties::<T>::insert(kitty_id,&kitty); 
			KittyOwner::<T>::insert(kitty_id,&who); 

			NextKittyId::<T>::put(kitty_id + 1u32.into());

			OwnerKitties::<T>::try_mutate(&who, |kitty_index_vec| {
				kitty_index_vec.try_push(kitty_id.clone())
			}).map_err(|_| Error::<T>::ExceedMaxOwnerKitties)?;

			Self::deposit_event(Event::KittyCreated(who,kitty_id, kitty));
			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn breed(origin: OriginFor<T>, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) -> DispatchResult{
			let who = ensure_signed(origin)?;

			ensure!(kitty_id_1 != kitty_id_2, Error::<T>::SameId);

			T::Currency::reserve(&who, T::KittyStake::get()).map_err(|_| Error::<T>::StakeNotEnough)?;

			let kitty_1 =Self::get_kitty(kitty_id_1).map_err(|_| Error::<T>::InvalidId)?; 
			let kitty_2 = Self::get_kitty(kitty_id_2).map_err(|_| Error::<T>::InvalidId)?;

			let kitty_id = match Self::next_kitty_id() {
				Some(id) => {
					ensure!(id != T::KittyIndex::max_value(), Error::<T>::KittiesOverflow);
					id
				}
				None => 0u32.into(),
			};
			
			let selector=Self::random_value(&who);
			let mut data =[0u8; 16];
			for i in 0..kitty_1.0.len() {
				data[i] =(kitty_1.0[i] & selector[i])| (kitty_2.0[i] & !selector[i]);
			}
			let new_kitty = Kitty(data);

			<Kitties<T>>::insert(kitty_id, &new_kitty);
			KittyOwner::<T>::insert(kitty_id,&who); 
			NextKittyId::<T>::put(kitty_id + 1u32.into());

			OwnerKitties::<T>::try_mutate(&who, |kitty_index_vec| {
				kitty_index_vec.try_push(kitty_id.clone())
			}).map_err(|_| Error::<T>::ExceedMaxOwnerKitties)?;

			Self::deposit_event(Event::KittyBred(who, kitty_id, new_kitty));
			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn transfer(origin: OriginFor<T>,kitty_id: T::KittyIndex,new_owner:T::AccountId)-> DispatchResult{
			let who=ensure_signed(origin)?;

			Self::get_kitty(kitty_id).map_err(|_| Error::<T>::InvalidId)?;
			ensure!(Self::kitty_owner(kitty_id)==Some(who.clone()),Error::<T>::NotOwner);

			OwnerKitties::<T>::try_mutate(&who, |kitty_index_vec| {
				if let Some(index) = kitty_index_vec.iter().position(|kitty_index| kitty_index == &kitty_id) {
					kitty_index_vec.swap_remove(index);
					return Ok(());
				}
				Err(())
			}).map_err(|_| <Error<T>>::NotOwner)?;

			T::Currency::reserve(&new_owner, T::KittyStake::get()).map_err(|_| Error::<T>::StakeNotEnough)?;

			<KittyOwner<T>>::insert(kitty_id, &new_owner);

			T::Currency::unreserve(&who, T::KittyStake::get());

			OwnerKitties::<T>::try_mutate(&new_owner, |kitty_index_vec| {
				kitty_index_vec.try_push(kitty_id.clone())
			}).map_err(|_| Error::<T>::ExceedMaxOwnerKitties)?;

			Self::deposit_event(Event::KittyTransferred(who, new_owner, kitty_id));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T>{
		// get random
		fn random_value(sender: &T::AccountId)->[u8;16]{
			let payload=(
				T::Randomness::random_seed(),
				&sender,
				<frame_system::Pallet::<T>>::extrinsic_index(),
			);

			payload.using_encoded(blake2_128)
		}

		// get kitty
		fn get_kitty(kitty_id: T::KittyIndex)-> Result<Kitty,()>{
			match Self::kitties(kitty_id){
				Some(kitty)=> Ok(kitty), 
				None => Err(()),
			}
		}
	}
}