#![cfg_attr(not(feature = "std"), no_std)]
#[allow(unused_imports)]
// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;



#[frame_support::pallet]
pub mod pallet {
  use frame_support::pallet_prelude::*;
  use frame_system::pallet_prelude::*;
  // use frame_support::inherent::Vec;
  use scale_info::prelude::string::String;
	use frame_support::{
		sp_runtime::traits::Hash,
		traits::{tokens::ExistenceRequirement, Currency, Randomness},
		transactional,
	};
	use scale_info::TypeInfo;
	use sp_io::hashing::blake2_128;


  #[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;


  // Struct for holding File information.
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct File<T: Config> {
		pub dna: [u8; 16], // Using 16 bytes to represent a kitty DNA
		pub price: Option<BalanceOf<T>>,
		pub gender: Gender,
		pub owner: AccountOf<T>,
	}

  // Set Gender type in Kitty struct.
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Gender {
		Male,
		Female,
	}

    
  #[derive(Encode, Decode, Clone, Default, Eq, PartialEq, Debug, MaxEncodedLen, TypeInfo)]
  pub struct File<AccountId, Hash> {
      cid : Hash,
      uploader: AccountId,
      file_link: String,
      allow_download: bool,
      file_type: String,
      cost: u32,
      file_size: u32,
  }
  

  #[pallet::pallet]
  #[pallet::generate_store(pub(super) trait Store)]
  pub struct Pallet<T>(_);

  /// Configure the pallet by specifying the parameters and types on which it depends.
  #[pallet::config]
  pub trait Config: frame_system::Config {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

    /// The Currency handler for the Kitties pallet.
		type Currency: Currency<Self::AccountId>;

		/// The maximum amount of Kitties a single account can own.
		#[pallet::constant]
		type MaxFileOwned: Get<u32>;

		/// The type of Randomness we want to specify for this pallet.
		type KittyRandomness: Randomness<Self::Hash, Self::BlockNumber>;
  }
  
  
  // Pallets use events to inform users when important changes are made.
  // Event documentation should end with an array that provides descriptive names for parameters.
  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    /// Event emitted when a claim has been created.
    ClaimCreated { who: T::AccountId, claim: T::Hash },
    /// Event emitted when a claim is revoked by the owner.
    ClaimRevoked { who: T::AccountId, claim: T::Hash },
    ///Event emitted when a file is uploaded 
    FileCreated { who: T::AccountId, cid: T::Hash },
  }
  
  
  #[pallet::error]
  pub enum Error<T> {
    /// The claim already exists.
    AlreadyClaimed,
    /// The claim does not exist, so it cannot be revoked.
    NoSuchClaim,
    /// The claim is owned by another account, so caller can't revoke it.
    NotClaimOwner,
    /// Handles arithemtic overflow when incrementing the Kitty counter.
		KittyCntOverflow,
		/// An account cannot own more Kitties than `MaxKittyCount`.
		ExceedMaxFileOwned,
		/// Buyer cannot be the owner.
		BuyerIsKittyOwner,
		/// Cannot transfer a kitty to its owner.
		TransferToSelf,
		/// Handles checking whether the Kitty exists.
		KittyNotExist,
		/// Handles checking that the Kitty is owned by the account transferring, buying or setting a price for it.
		NotKittyOwner,
		/// Ensures the Kitty is for sale.
		KittyNotForSale,
		/// Ensures that the buying price is greater than the asking price.
		KittyBidPriceTooLow,
		/// Ensures that an account has enough funds to purchase a Kitty.
		NotEnoughBalance,
  }

  
  // #[pallet::storage]
  // pub(super) type  Files<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, File< T::AccountId, T::Hash> >; 
  // pub(super) type Claims<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, (T::AccountId, T::BlockNumber)>;
  // pub(super) type  Files<T> = StorageMap<_, Blake2_128Concat, T, File >; 
  //cid ,(uploader, file_link, allow_download, file_type, cost, file_size)
  // pub(super) type FileOwner<T: Config> = StorageMap<_, Blake2_128Concat, u32 , T::AccountId>;
  
  // Dispatchable functions allow users to interact with the pallet and invoke state changes.
  // These functions materialize as "extrinsics", which are often compared to transactions.
  // Dispatchable functions must be annotated with a weight and must return a DispatchResult.

  #[pallet::storage]
	#[pallet::getter(fn file_cnt)]
	/// Keeps track of the number of Files in existence.
	pub(super) type FileCnt<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn files)]
	/// Stores a Files's unique traits, owner and price.
	pub(super) type Files<T: Config> = StorageMap<_, Twox64Concat, T::Hash, File<T>>;

	#[pallet::storage]
	#[pallet::getter(fn files_owned)]
	/// Keeps track of what accounts own what Kitty.
	pub(super) type FilesOwned<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<T::Hash, T::MaxFileOwned>,
		ValueQuery,
	>;

  #[pallet::call]
  impl<T: Config> Pallet<T> {


    /// Upload File and sets its properties and updates storage.
		#[pallet::weight(100)]
		pub fn create_file(
			origin: OriginFor<T>,
			cid: T::Hash,
			new_price: Option<BalanceOf<T>>,
      dna: Option<[u8;16]>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// ACTION #1a: Checking Kitty owner
			// ensure!(Self::is_kitty_owner(&kitty_id, &sender)?, <Error<T>>::NotKittyOwner);
      //Action: checking if file already created
      ensure!(!Files::<T>::contains_key(&cid), Error::<T>::AlreadyClaimed);

      //create File data
      let file = File::<T> {
        dna: dna.unwrap_or_else([1,1,1,1,2,2,2,2,3,3,3,3,4,4,4,4]),
        price: new_price.clone(),
        gender: gender.unwrap_or_else(Gender.Male),
        owner: AccountOf<T>,
      }

			// let mut kitty = Self::kitties(&kitty_id).ok_or(<Error<T>>::KittyNotExist)?;

			// ACTION #2: Set the Kitty price and update new Kitty infomation to storage.
			// kitty.price = new_price.clone();
			<Files<T>>::insert(&cid, file);

			// ACTION #3: Deposit a "PriceSet" event.
			// Self::deposit_event(Event::PriceSet(sender, kitty_id, new_price));

      // Deposite file created event
      Self::deposit_event(Event::FileCreated { who: sender, cid });
			Ok(())
		}
  }
}