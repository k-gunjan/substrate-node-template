#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_imports)]
#![allow(unused_variables)]
// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
//   #[allow(unused_imports)]
  use frame_support::pallet_prelude::*;
  use frame_system::pallet_prelude::*;
  // use frame_support::inherent::Vec;
  use scale_info::prelude::string::String;
	use frame_support::{
		inherent::Vec,
		sp_runtime::traits::Hash,
		traits::{tokens::ExistenceRequirement, Currency, Randomness},
		transactional,
	};
	use scale_info::TypeInfo;
	use sp_io::hashing::{
		blake2_128,
		sha2_256
	};
	// use url::Url;
	use hex_literal;


  #[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;


  // Struct for holding File information.
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct File<T: Config> {
		pub price: Option<BalanceOf<T>>,
		pub owner: AccountOf<T>,
		pub file_type : FileType,
		pub file_link: BoundedVec<u8, T::MaxLength>,
		pub allow_download :bool,
		pub file_size : u32,
	}

	 // Set FileType type in File struct.
	 #[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	 #[scale_info(skip_type_params(T))]
	 #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	 pub enum FileType {
		 Normal,
		 Privileged,
	 }
    

  #[pallet::pallet]
  #[pallet::generate_store(pub(super) trait Store)]
  pub struct Pallet<T>(_);

  /// Configure the pallet by specifying the parameters and types on which it depends.
  #[pallet::config]
  pub trait Config: frame_system::Config {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

    /// The Currency handler for the FileStorage pallet.
	type Currency: Currency<Self::AccountId>;

	/// The maximum number of files a single account can own.
	#[pallet::constant]
	type MaxFileOwned: Get<u32>;

	/// The minimum length a file_link may be.
	#[pallet::constant]
	type MinLength: Get<u32>;
	/// The maximum length a file_link may be.
	#[pallet::constant]
	type MaxLength: Get<u32>;

	/// max length of vector of owners of a file
	// type MaxLengthOwners: Get<Self::Hash>;

	/// The type of Randomness we want to specify for this pallet.
	type KittyRandomness: Randomness<Self::Hash, Self::BlockNumber>;
  }
  
  
  // Pallets use events to inform users when important changes are made.
  // Event documentation should end with an array that provides descriptive names for parameters.
  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    ///Event emitted when a file is uploaded 
    FileCreated { who: T::AccountId, cid: T::Hash },
	///Event file Downloaded
	FileDownloaded {cid: T::Hash, count: u64},
	///Event ownership changed
	FileOwnerChanged {cid: T::Hash, new_owner: T::AccountId},
  }
  
  
  #[pallet::error]
  pub enum Error<T> {
	///already uploaded
	AlreadyUploaded,
	///file does not exist
	FileDoesNotExist,
	///link of the file is too long
	LinkTooLong,
	///link of the file is too short
	LinkTooShort,
	///sender is not the owner of the file
	SenderIsNotOwner,
	///file download not allowed at the time of upload
	FileNotDownloadable,
  }

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
	/// Keeps track of what accounts own what File.
	pub(super) type FilesOwned<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::Hash,
		T::AccountId,
		// ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn files_download_cnt)]
	/// Keeps track of count of downloads file wise.
	pub(super) type FilesDownloadCnt<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::Hash,
		u64,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn total_download_cnt)]
	/// Keeps track of total number of downloads.
	pub(super) type TotalDownloadCount<T: Config> = StorageValue<
		_,
		u64,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn file_downloaders)]
	/// Keeps track of what accounts downloaded a file
	pub(super) type FileDownloaders<T: Config> = StorageMap<
		_,
		Twox64Concat,
		[u64;32],
		(T::AccountId, T::Hash, u64),
		// ValueQuery,
	>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // /helper fund
		// fn calculate_hash<T: Hash>(t: &T) -> u64 {
		// 	let mut s = DefaultHasher::new();
		// 	t.hash(&mut s);
		// 	s.finish()
		// }

		// #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        // pub fn my_transfer(origin: OriginFor<T>,  source: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        // let owner = ensure_signed(origin)?;
        // match PalletDataStore::<T>::get() {
        //   Some(destination) => {
        //       T::Currency::transfer(&source, &destination, amount, ExistenceRequirement::KeepAlive)?;
        //   },
        //   None => return Err(Error::<T>::NoneValue.into()),
        // };
        //  Ok(())
        //    }

        /// Upload File and sets its properties and updates storage.
		#[pallet::weight(100)]
		pub fn create_file(
			origin: OriginFor<T>,
			cid: T::Hash,
			cost: Option<BalanceOf<T>>,
			file_type: Option<FileType>,
			file_link: Vec<u8>,
			allow_download :bool,
            file_size: u32,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

            //Action: checking if the file already created
            ensure!(!Files::<T>::contains_key(&cid), Error::<T>::AlreadyUploaded);
      
	        let bounded_file_link: BoundedVec<_, _> =
	      			file_link.try_into().map_err(|()| Error::<T>::LinkTooLong)?;
	        ensure!(bounded_file_link.len() >= T::MinLength::get() as usize, Error::<T>::LinkTooShort);
      
            let new_cost: Option<BalanceOf<T>> = 
	                 if file_size < 250 {
	      			None
	      		   } else {
	      			cost.clone()
	      		   };
			// let dave: T::AccountId = hex_literal::hex!["5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy"].into();
			// T::Currency::transfer(&sender, &dave, cost.unwrap(), ExistenceRequirement::KeepAlive)?;
            //create File data
            let file = File::<T> {
            price: new_cost,
            file_type: file_type.unwrap_or_else(|| FileType::Normal),
            owner: sender.clone(),
	      	file_link: bounded_file_link,
	      	allow_download,
	      	file_size,
            };
			//insert file
	        <Files<T>>::insert(&cid, file);

			//update number of total files uploaded
	        let mut cnt = <FileCnt<T>>::get();
	        cnt+=1;
	        <FileCnt<T>>::set(cnt);
      
	        //update owner of the files
	        <FilesOwned<T>>::insert(&cid, &sender);
      
            // Deposite file created event
            Self::deposit_event(Event::FileCreated { who: sender, cid });
	      		Ok(())
	    }

    

        /// Download File .
		#[pallet::weight(100)]
		pub fn download_file(
			origin: OriginFor<T>,
			cid: T::Hash,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// check if file exists
            ensure!(Files::<T>::contains_key(&cid), Error::<T>::FileDoesNotExist);
            //check if file is downloadable
			let is_allowed = <Files<T>>::get(&cid).unwrap().allow_download;
	        ensure!(is_allowed, Error::<T>::FileNotDownloadable);
			//increment the download count of individual files
	        <FilesDownloadCnt<T>>::mutate(&cid, |x| {
				let cnt = *x;
				*x = cnt + 1; //Some(cnt + 1);
			});   

			//increment overall file download count
			<TotalDownloadCount<T>>::mutate(|x| *x+=1 );

			//trace downloader details
			//check if file downloaded
			// let mut has_str = sha2_256(sender.to_vec());
			// has_str.push_str(sender.decode_into_raw_public_keys());
			// has_str;
			// let ifd:bool = <FileDownloaders<T>>::contains_key(&cid);
			if 1 == 1 {
				//if downloaded
				// let downloader: T::AccountId = <FileDownloaders<T>>::get(&cid).unwrap()[0];
				// //check if sender downloaded
				// if sender == downloader {
				// 	//update the count of downlodes
				// 	<FileDownloaders<T>>::mutate(&cid, |x| {
				// 		let cnt = *x[1];

				// 	} )
				// }
			} else {
				//file never downloaded so add the details
				// <FileDownloaders<T>>::insert(&cid, (sender,1));
			}


            //get the count of download of the file
			let cnt: u64 = <FilesDownloadCnt<T>>::get(&cid); 
            // Deposite file created event
            Self::deposit_event(Event::FileDownloaded{ cid, count: cnt });
	      	Ok(())
	    }

		/// Transfer Ownership .
		#[pallet::weight(100)]
		pub fn change_owner_of_file(
			origin: OriginFor<T>,
			cid: T::Hash,
			new_owner: T::AccountId,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			//check if file exists
            ensure!(Files::<T>::contains_key(&cid), Error::<T>::FileDoesNotExist);
			//get the owner of the file
			let owner = <FilesOwned<T>>::get(&cid);
			//check if the file is owneed by the sender
			ensure!(owner == core::prelude::v1::Some(sender.clone()), Error::<T>::SenderIsNotOwner);
	        //increment the download count by one
	        <FilesOwned<T>>::mutate(&cid, |_| sender.clone());      
            // Deposite file owner changed event
            Self::deposit_event(Event::FileOwnerChanged{ cid, new_owner: sender });
	      		Ok(())
	    }

    }


}