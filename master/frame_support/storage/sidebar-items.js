window.SIDEBAR_ITEMS = {"enum":[["TransactionOutcome","Describes on what should happen with a storage transaction."]],"fn":[["storage_prefix","Returns the storage prefix for a specific pallet name and storage name."]],"mod":[["bounded_btree_map","Traits, types and structs to support a bounded BTreeMap."],["bounded_btree_set","Traits, types and structs to support a bounded `BTreeSet`."],["bounded_vec","Traits, types and structs to support putting a bounded vector into storage, as a raw value, map or a double map."],["child","Operation on runtime child storages."],["hashed","Operation on runtime storage using hashed keys."],["migration","Some utilities for helping access storage with arbitrary key types."],["storage_noop_guard","Contains the [`crate::StorageNoopGuard`] for conveniently asserting that no storage mutation has been made by a whole code block."],["transactional","Provides functionality around the transaction storage."],["types","Storage types to build abstraction on storage, they implements storage traits such as StorageMap and others."],["unhashed","Operation on unhashed runtime storage."],["weak_bounded_vec","Traits, types and structs to support putting a bounded vector into storage, as a raw value, map or a double map."]],"struct":[["ChildTriePrefixIterator","Iterate over a prefix of a child trie and decode raw_key and raw_value into `T`."],["Key","A type used exclusively by storage maps as their key type."],["KeyLenOf","Utility type for converting a storage map into a `Get<u32>` impl which returns the maximum key size."],["KeyPrefixIterator","Iterate over a prefix and decode raw_key into `T`."],["PrefixIterator","Iterate or drain over a prefix and decode raw_key and raw_value into `T`."]],"trait":[["IterableStorageDoubleMap","A strongly-typed double map in storage whose secondary keys and values can be iterated over."],["IterableStorageMap","A strongly-typed map in storage whose keys and values can be iterated over."],["IterableStorageNMap","A strongly-typed map with arbitrary number of keys in storage whose keys and values can be iterated over."],["PrefixIteratorOnRemoval","Trait for specialising on removal logic of [`PrefixIterator`]."],["StorageAppend","Marker trait that will be implemented for types that support the `storage::append` api."],["StorageDecodeLength","Marker trait that will be implemented for types that support to decode their length in an efficient way. It is expected that the length is at the beginning of the encoded object and that the length is a `Compact<u32>`."],["StorageDoubleMap","An implementation of a map with a two keys."],["StorageMap","A strongly-typed map in storage."],["StorageNMap","An implementation of a map with an arbitrary number of keys."],["StoragePrefixedMap","Trait for maps that store all its value after a unique prefix."],["StorageStreamIter","An iterator that streams values directly from storage."],["StorageTryAppend","Marker trait that is implemented for types that support the `storage::append` api with a limit on the number of element."],["StorageValue","A trait for working with macro-generated storage values under the substrate storage API."],["TryAppendDoubleMap","Storage double map that is capable of `StorageTryAppend`."],["TryAppendMap","Storage map that is capable of `StorageTryAppend`."],["TryAppendValue","Storage value that is capable of `StorageTryAppend`."]]};