use std::collections::BTreeMap;

use super::Address;
use super::Identifier;
use super::ObjectDigest;
use super::ObjectId;
use super::StructTag;
use super::TransactionDigest;

pub type Version = u64;

/// Reference to an object
///
/// Contains sufficient information to uniquely identify a specific object.
///
/// # BCS
///
/// The BCS serialized form for this type is defined by the following ABNF:
///
/// ```text
/// object-ref = object-id u64 digest
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[cfg_attr(feature = "proptest", derive(test_strategy::Arbitrary))]
pub struct ObjectReference {
    /// The object id of this object.
    object_id: ObjectId,
    /// The version of this object.
    version: Version,
    /// The digest of this object.
    digest: ObjectDigest,
}

impl ObjectReference {
    /// Creates a new object reference from the object's id, version, and digest.
    pub fn new(object_id: ObjectId, version: Version, digest: ObjectDigest) -> Self {
        Self {
            object_id,
            version,
            digest,
        }
    }

    /// Returns a reference to the object id that this ObjectReference is referring to.
    pub fn object_id(&self) -> &ObjectId {
        &self.object_id
    }

    /// Returns the version of the object that this ObjectReference is referring to.
    pub fn version(&self) -> Version {
        self.version
    }

    /// Returns the digest of the object that this ObjectReference is referring to.
    pub fn digest(&self) -> &ObjectDigest {
        &self.digest
    }

    /// Returns a 3-tuple containing the object id, version, and digest.
    pub fn into_parts(self) -> (ObjectId, Version, ObjectDigest) {
        let Self {
            object_id,
            version,
            digest,
        } = self;

        (object_id, version, digest)
    }
}

/// Enum of different types of ownership for an object.
///
/// # BCS
///
/// The BCS serialized form for this type is defined by the following ABNF:
///
/// ```text
/// owner = owner-address / owner-object / owner-shared / owner-immutable
///
/// owner-address   = %x00 address
/// owner-object    = %x01 object-id
/// owner-shared    = %x02 u64
/// owner-immutable = %x03
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize),
    serde(rename_all = "lowercase")
)]
#[cfg_attr(feature = "proptest", derive(test_strategy::Arbitrary))]
pub enum Owner {
    /// Object is exclusively owned by a single address, and is mutable.
    Address(Address),
    /// Object is exclusively owned by a single object, and is mutable.
    Object(ObjectId),
    /// Object is shared, can be used by any address, and is mutable.
    Shared(
        /// The version at which the object became shared
        Version,
    ),
    /// Object is immutable, and hence ownership doesn't matter.
    Immutable,

    /// Object is exclusively owned by a single address and sequenced via consensus.
    ConsensusAddress {
        /// The version at which the object most recently became a consensus object.
        /// This serves the same function as `initial_shared_version`, except it may change
        /// if the object's Owner type changes.
        start_version: Version,

        /// The owner of the object.
        owner: Address,
    },
}

/// Object data, either a package or struct
///
/// # BCS
///
/// The BCS serialized form for this type is defined by the following ABNF:
///
/// ```text
/// object-data = object-data-struct / object-data-package
///
/// object-data-struct  = %x00 object-move-struct
/// object-data-package = %x01 object-move-package
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[allow(clippy::large_enum_variant)]
#[cfg_attr(feature = "proptest", derive(test_strategy::Arbitrary))]
//TODO think about hiding this type and not exposing it
pub enum ObjectData {
    /// An object whose governing logic lives in a published Move module
    Struct(MoveStruct),
    /// Map from each module name to raw serialized Move module bytes
    Package(MovePackage),
    // ... Sui "native" types go here
}

/// A move package
///
/// # BCS
///
/// The BCS serialized form for this type is defined by the following ABNF:
///
/// ```text
/// object-move-package = object-id u64 move-modules type-origin-table linkage-table
///
/// move-modules = map (identifier bytes)
/// type-origin-table = vector type-origin
/// linkage-table = map (object-id upgrade-info)
/// ```
#[derive(Eq, PartialEq, Debug, Clone, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[cfg_attr(feature = "proptest", derive(test_strategy::Arbitrary))]
pub struct MovePackage {
    /// Address or Id of this package
    pub id: ObjectId,

    /// Most move packages are uniquely identified by their ID (i.e. there is only one version per
    /// ID), but the version is still stored because one package may be an upgrade of another (at a
    /// different ID), in which case its version will be one greater than the version of the
    /// upgraded package.
    ///
    /// Framework packages are an exception to this rule -- all versions of the framework packages
    /// exist at the same ID, at increasing versions.
    ///
    /// In all cases, packages are referred to by move calls using just their ID, and they are
    /// always loaded at their latest version.
    pub version: Version,

    /// Set of modules defined by this package
    #[cfg_attr(
        feature = "serde",
        serde(with = "::serde_with::As::<BTreeMap<::serde_with::Same, ::serde_with::Bytes>>")
    )]
    #[cfg_attr(
        feature = "proptest",
        strategy(
            proptest::collection::btree_map(proptest::arbitrary::any::<Identifier>(), proptest::collection::vec(proptest::arbitrary::any::<u8>(), 0..=1024), 0..=5)
        )
    )]
    pub modules: BTreeMap<Identifier, Vec<u8>>,

    /// Maps struct/module to a package version where it was first defined, stored as a vector for
    /// simple serialization and deserialization.
    #[cfg_attr(feature = "proptest", any(proptest::collection::size_range(0..=1).lift()))]
    pub type_origin_table: Vec<TypeOrigin>,

    /// For each dependency, maps original package ID to the info about the (upgraded) dependency
    /// version that this package is using
    #[cfg_attr(
        feature = "proptest",
        strategy(
            proptest::collection::btree_map(proptest::arbitrary::any::<ObjectId>(), proptest::arbitrary::any::<UpgradeInfo>(), 0..=5)
        )
    )]
    pub linkage_table: BTreeMap<ObjectId, UpgradeInfo>,
}

/// Identifies a struct and the module it was defined in
///
/// # BCS
///
/// The BCS serialized form for this type is defined by the following ABNF:
///
/// ```text
/// type-origin = identifier identifier object-id
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[cfg_attr(feature = "proptest", derive(test_strategy::Arbitrary))]
pub struct TypeOrigin {
    pub module_name: Identifier,
    pub struct_name: Identifier,
    pub package: ObjectId,
}

/// Upgraded package info for the linkage table
///
/// # BCS
///
/// The BCS serialized form for this type is defined by the following ABNF:
///
/// ```text
/// upgrade-info = object-id u64
/// ```
#[derive(Eq, PartialEq, Debug, Clone, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[cfg_attr(feature = "proptest", derive(test_strategy::Arbitrary))]
pub struct UpgradeInfo {
    /// Id of the upgraded packages
    pub upgraded_id: ObjectId,
    /// Version of the upgraded package
    pub upgraded_version: Version,
}

/// A move struct
///
/// # BCS
///
/// The BCS serialized form for this type is defined by the following ABNF:
///
/// ```text
/// object-move-struct = compressed-struct-tag bool u64 object-contents
///
/// compressed-struct-tag = other-struct-type / gas-coin-type / staked-sui-type / coin-type
/// other-struct-type     = %x00 struct-tag
/// gas-coin-type         = %x01
/// staked-sui-type       = %x02
/// coin-type             = %x03 type-tag
///
/// ; first 32 bytes of the contents are the object's object-id
/// object-contents = uleb128 (object-id *OCTET) ; length followed by contents
/// ```
#[derive(Eq, PartialEq, Debug, Clone, Hash)]
//TODO hand-roll a Deserialize impl to enforce that an objectid is present
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[cfg_attr(feature = "proptest", derive(test_strategy::Arbitrary))]
pub struct MoveStruct {
    /// The type of this object
    #[cfg_attr(
        feature = "serde",
        serde(with = "::serde_with::As::<serialization::BinaryMoveStructType>")
    )]
    pub(crate) type_: StructTag,

    /// DEPRECATED this field is no longer used to determine whether a tx can transfer this
    /// object. Instead, it is always calculated from the objects type when loaded in execution
    has_public_transfer: bool,

    /// Number that increases each time a tx takes this object as a mutable input
    /// This is a lamport timestamp, not a sequentially increasing version
    version: Version,

    /// BCS bytes of a Move struct value
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::_serde::ReadableBase64Encoded")
    )]
    #[cfg_attr(feature = "proptest", any(proptest::collection::size_range(32..=1024).lift()))]
    pub(crate) contents: Vec<u8>,
}

impl MoveStruct {
    /// Construct a move struct
    pub fn new(
        type_: StructTag,
        has_public_transfer: bool,
        version: Version,
        contents: Vec<u8>,
    ) -> Option<Self> {
        id_opt(&contents).map(|_| Self {
            type_,
            has_public_transfer,
            version,
            contents,
        })
    }

    /// Return the type of the struct
    pub fn object_type(&self) -> &StructTag {
        &self.type_
    }

    /// Return if this object can be publicly transfered
    ///
    /// DEPRECATED
    ///
    /// This field is no longer used to determine whether a tx can transfer this object. Instead,
    /// it is always calculated from the objects type when loaded in execution.
    #[doc(hidden)]
    pub fn has_public_transfer(&self) -> bool {
        self.has_public_transfer
    }

    /// Return the version of this object
    pub fn version(&self) -> Version {
        self.version
    }

    /// Return the raw contents of this struct
    pub fn contents(&self) -> &[u8] {
        &self.contents
    }

    /// Return the ObjectId of this object
    pub fn object_id(&self) -> ObjectId {
        id_opt(self.contents()).unwrap()
    }
}

/// Type of a Sui object
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum ObjectType {
    /// Move package containing one or more bytecode modules
    Package,
    /// A Move struct of the given type
    Struct(StructTag),
}

/// An object on the sui blockchain
///
/// # BCS
///
/// The BCS serialized form for this type is defined by the following ABNF:
///
/// ```text
/// object = object-data owner digest u64
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[cfg_attr(feature = "proptest", derive(test_strategy::Arbitrary))]
pub struct Object {
    /// The meat of the object
    pub(crate) data: ObjectData,

    /// The owner that unlocks this object
    owner: Owner,

    /// The digest of the transaction that created or last mutated this object
    previous_transaction: TransactionDigest,

    /// The amount of SUI we would rebate if this object gets deleted.
    /// This number is re-calculated each time the object is mutated based on
    /// the present storage gas price.
    storage_rebate: u64,
}

impl Object {
    /// Build an object
    pub fn new(
        data: ObjectData,
        owner: Owner,
        previous_transaction: TransactionDigest,
        storage_rebate: u64,
    ) -> Self {
        Self {
            data,
            owner,
            previous_transaction,
            storage_rebate,
        }
    }

    /// Return this object's id
    pub fn object_id(&self) -> ObjectId {
        match &self.data {
            ObjectData::Struct(struct_) => id_opt(&struct_.contents).unwrap(),
            ObjectData::Package(package) => package.id,
        }
    }

    /// Return this object's version
    pub fn version(&self) -> Version {
        match &self.data {
            ObjectData::Struct(struct_) => struct_.version,
            ObjectData::Package(package) => package.version,
        }
    }

    /// Return this object's type
    pub fn object_type(&self) -> ObjectType {
        match &self.data {
            ObjectData::Struct(struct_) => ObjectType::Struct(struct_.type_.clone()),
            ObjectData::Package(_) => ObjectType::Package,
        }
    }

    /// Try to interpret this object as a move struct
    pub fn as_struct(&self) -> Option<&MoveStruct> {
        match &self.data {
            ObjectData::Struct(struct_) => Some(struct_),
            _ => None,
        }
    }

    /// Return this object's owner
    pub fn owner(&self) -> &Owner {
        &self.owner
    }

    /// Return this object's data
    pub fn data(&self) -> &ObjectData {
        &self.data
    }

    /// Return the digest of the transaction that last modified this object
    pub fn previous_transaction(&self) -> TransactionDigest {
        self.previous_transaction
    }

    /// Return the storage rebate locked in this object
    ///
    /// Storage rebates are credited to the gas coin used in a transaction that deletes this
    /// object.
    pub fn storage_rebate(&self) -> u64 {
        self.storage_rebate
    }
}

fn id_opt(contents: &[u8]) -> Option<ObjectId> {
    if ObjectId::LENGTH > contents.len() {
        return None;
    }

    Some(ObjectId::from(
        Address::from_bytes(&contents[..ObjectId::LENGTH]).unwrap(),
    ))
}

/// An object part of the initial chain state
///
/// `GenesisObject`'s are included as a part of genesis, the initial checkpoint/transaction, that
/// initializes the state of the blockchain.
///
/// # BCS
///
/// The BCS serialized form for this type is defined by the following ABNF:
///
/// ```text
/// genesis-object = object-data owner
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "proptest", derive(test_strategy::Arbitrary))]
pub struct GenesisObject {
    data: ObjectData,
    owner: Owner,
}

impl GenesisObject {
    pub fn new(data: ObjectData, owner: Owner) -> Self {
        Self { data, owner }
    }

    pub fn object_id(&self) -> ObjectId {
        match &self.data {
            ObjectData::Struct(struct_) => id_opt(&struct_.contents).unwrap(),
            ObjectData::Package(package) => package.id,
        }
    }

    pub fn version(&self) -> Version {
        match &self.data {
            ObjectData::Struct(struct_) => struct_.version,
            ObjectData::Package(package) => package.version,
        }
    }

    pub fn object_type(&self) -> ObjectType {
        match &self.data {
            ObjectData::Struct(struct_) => ObjectType::Struct(struct_.type_.clone()),
            ObjectData::Package(_) => ObjectType::Package,
        }
    }

    pub fn owner(&self) -> &Owner {
        &self.owner
    }

    pub fn data(&self) -> &ObjectData {
        &self.data
    }
}

//TODO improve ser/de to do borrowing to avoid clones where possible
#[cfg(feature = "serde")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "serde")))]
mod serialization {
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serialize;
    use serde::Serializer;
    use serde_with::DeserializeAs;
    use serde_with::SerializeAs;

    use super::*;
    use crate::TypeTag;

    /// Wrapper around StructTag with a space-efficient representation for common types like coins
    /// The StructTag for a gas coin is 84 bytes, so using 1 byte instead is a win.
    /// The inner representation is private to prevent incorrectly constructing an `Other` instead of
    /// one of the specialized variants, e.g. `Other(GasCoin::type_())` instead of `GasCoin`
    #[derive(serde_derive::Deserialize)]
    enum MoveStructType {
        /// A type that is not `0x2::coin::Coin<T>`
        Other(StructTag),
        /// A SUI coin (i.e., `0x2::coin::Coin<0x2::sui::SUI>`)
        GasCoin(TypeTag),

        /// A record of a staked SUI coin (i.e., `0x3::staking_pool::StakedSui`)
        StakedSui,
        /// A non-SUI coin type (i.e., `0x2::coin::Coin<T> where T != 0x2::sui::SUI`)
        Coin(TypeTag),
        // NOTE: if adding a new type here, and there are existing on-chain objects of that
        // type with Other(_), that is ok, but you must hand-roll PartialEq/Eq/Ord/maybe Hash
        // to make sure the new type and Other(_) are interpreted consistently.
    }

    /// See `MoveStructType`
    #[derive(serde_derive::Serialize)]
    enum MoveStructTypeRef<'a> {
        /// A type that is not `0x2::coin::Coin<T>`
        Other(&'a StructTag),
        /// A SUI coin (i.e., `0x2::coin::Coin<0x2::sui::SUI>`)
        GasCoin(&'a TypeTag),
        /// A record of a staked SUI coin (i.e., `0x3::staking_pool::StakedSui`)
        StakedSui,
        /// A non-SUI coin type (i.e., `0x2::coin::Coin<T> where T != 0x2::sui::SUI`)
        Coin(&'a TypeTag),
        // NOTE: if adding a new type here, and there are existing on-chain objects of that
        // type with Other(_), that is ok, but you must hand-roll PartialEq/Eq/Ord/maybe Hash
        // to make sure the new type and Other(_) are interpreted consistently.
    }

    impl MoveStructType {
        fn into_struct_tag(self) -> StructTag {
            match self {
                MoveStructType::Other(tag) => tag,
                MoveStructType::GasCoin(type_tag) => StructTag::gas_coin(),
                MoveStructType::StakedSui => StructTag::staked_sui(),
                MoveStructType::Coin(type_tag) => StructTag::coin(type_tag),
            }
        }
    }

    impl<'a> MoveStructTypeRef<'a> {
        fn from_struct_tag(s: &'a StructTag) -> Self {
            let StructTag {
                address,
                module,
                name,
                type_params,
            } = s;

            if let Some(coin_type) = s.is_coin() {
                if let TypeTag::Struct(s_inner) = coin_type {
                    let StructTag {
                        address,
                        module,
                        name,
                        type_params,
                    } = s_inner.as_ref();

                    if address == &Address::TWO
                        && module == "bfc"
                        && name == "BFC"
                        && type_params.is_empty()
                    {
                        return Self::GasCoin(coin_type);
                    }
                }

                Self::Coin(coin_type)
            } else if address == &Address::THREE
                && module == "staking_pool"
                && name == "StakedBfc"
                && type_params.is_empty()
            {
                Self::StakedSui
            } else {
                Self::Other(s)
            }
        }
    }

    pub(super) struct BinaryMoveStructType;

    impl SerializeAs<StructTag> for BinaryMoveStructType {
        fn serialize_as<S>(source: &StructTag, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let move_object_type = MoveStructTypeRef::from_struct_tag(source);
            move_object_type.serialize(serializer)
        }
    }

    impl<'de> DeserializeAs<'de, StructTag> for BinaryMoveStructType {
        fn deserialize_as<D>(deserializer: D) -> Result<StructTag, D::Error>
        where
            D: Deserializer<'de>,
        {
            let struct_type = MoveStructType::deserialize(deserializer)?;
            Ok(struct_type.into_struct_tag())
        }
    }

    #[derive(serde_derive::Serialize, serde_derive::Deserialize)]
    enum BinaryGenesisObject {
        RawObject { data: ObjectData, owner: Owner },
    }

    impl Serialize for GenesisObject {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let binary = BinaryGenesisObject::RawObject {
                data: self.data.clone(),
                owner: self.owner,
            };
            binary.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for GenesisObject {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let BinaryGenesisObject::RawObject { data, owner } =
                Deserialize::deserialize(deserializer)?;

            Ok(GenesisObject { data, owner })
        }
    }

    #[cfg(test)]
    mod test {
        use crate::object::Object;

        #[cfg(target_arch = "wasm32")]
        use wasm_bindgen_test::wasm_bindgen_test as test;

        #[test]
        fn object_fixture() {
            const SUI_COIN: &[u8] = &[
                0, 1, 1, 32, 79, 43, 0, 0, 0, 0, 0, 40, 35, 95, 175, 213, 151, 87, 206, 190, 35,
                131, 79, 35, 254, 22, 15, 181, 40, 108, 28, 77, 68, 229, 107, 254, 191, 160, 196,
                186, 42, 2, 122, 53, 52, 133, 199, 58, 0, 0, 0, 0, 0, 79, 255, 208, 0, 85, 34, 190,
                75, 192, 41, 114, 76, 127, 15, 110, 215, 9, 58, 107, 243, 160, 155, 144, 230, 47,
                97, 220, 21, 24, 30, 26, 62, 32, 17, 197, 192, 38, 64, 173, 142, 143, 49, 111, 15,
                211, 92, 84, 48, 160, 243, 102, 229, 253, 251, 137, 210, 101, 119, 173, 228, 51,
                141, 20, 15, 85, 96, 19, 15, 0, 0, 0, 0, 0,
            ];

            const SUI_STAKE: &[u8] = &[
                0, 2, 1, 154, 1, 52, 5, 0, 0, 0, 0, 80, 3, 112, 71, 231, 166, 234, 205, 164, 99,
                237, 29, 56, 97, 170, 21, 96, 105, 158, 227, 122, 22, 251, 60, 162, 12, 97, 151,
                218, 71, 253, 231, 239, 116, 138, 12, 233, 128, 195, 128, 77, 33, 38, 122, 77, 53,
                154, 197, 198, 75, 212, 12, 182, 163, 224, 42, 82, 123, 69, 248, 40, 207, 143, 211,
                13, 106, 1, 0, 0, 0, 0, 0, 0, 59, 81, 183, 246, 112, 0, 0, 0, 0, 79, 255, 208, 0,
                85, 34, 190, 75, 192, 41, 114, 76, 127, 15, 110, 215, 9, 58, 107, 243, 160, 155,
                144, 230, 47, 97, 220, 21, 24, 30, 26, 62, 32, 247, 239, 248, 71, 247, 102, 190,
                149, 232, 153, 138, 67, 169, 209, 203, 29, 255, 215, 223, 57, 159, 44, 40, 218,
                166, 13, 80, 71, 14, 188, 232, 68, 0, 0, 0, 0, 0, 0, 0, 0,
            ];

            const NFT: &[u8] = &[
                0, 0, 97, 201, 195, 159, 216, 97, 133, 173, 96, 215, 56, 212, 229, 43, 208, 139,
                218, 7, 29, 54, 106, 205, 224, 126, 7, 195, 145, 106, 45, 117, 168, 22, 12, 100,
                105, 115, 116, 114, 105, 98, 117, 116, 105, 111, 110, 11, 68, 69, 69, 80, 87, 114,
                97, 112, 112, 101, 114, 0, 0, 124, 24, 223, 4, 0, 0, 0, 0, 40, 31, 8, 18, 84, 38,
                164, 252, 84, 115, 250, 246, 137, 132, 128, 186, 156, 36, 62, 18, 140, 21, 4, 90,
                209, 105, 85, 84, 92, 214, 97, 81, 207, 64, 194, 198, 208, 21, 0, 0, 0, 0, 79, 255,
                208, 0, 85, 34, 190, 75, 192, 41, 114, 76, 127, 15, 110, 215, 9, 58, 107, 243, 160,
                155, 144, 230, 47, 97, 220, 21, 24, 30, 26, 62, 32, 170, 4, 94, 114, 207, 155, 31,
                80, 62, 254, 220, 206, 240, 218, 83, 54, 204, 197, 255, 239, 41, 66, 199, 150, 56,
                189, 86, 217, 166, 216, 128, 241, 64, 205, 21, 0, 0, 0, 0, 0,
            ];

            const FUD_COIN: &[u8] = &[
                0, 3, 7, 118, 203, 129, 155, 1, 171, 237, 80, 43, 238, 138, 112, 43, 76, 45, 84,
                117, 50, 193, 47, 37, 0, 28, 157, 234, 121, 90, 94, 99, 28, 38, 241, 3, 102, 117,
                100, 3, 70, 85, 68, 0, 1, 193, 89, 252, 3, 0, 0, 0, 0, 40, 33, 214, 90, 11, 56,
                243, 115, 10, 250, 121, 250, 28, 34, 237, 104, 130, 148, 40, 130, 29, 248, 137,
                244, 27, 138, 94, 150, 28, 182, 104, 162, 185, 0, 152, 247, 62, 93, 1, 0, 0, 0, 42,
                95, 32, 226, 13, 31, 128, 91, 188, 127, 235, 12, 75, 73, 116, 112, 3, 227, 244,
                126, 59, 81, 214, 118, 144, 243, 195, 17, 82, 216, 119, 170, 32, 239, 247, 71, 249,
                241, 98, 133, 53, 46, 37, 100, 242, 94, 231, 241, 184, 8, 69, 192, 69, 67, 1, 116,
                251, 229, 226, 99, 119, 79, 255, 71, 43, 64, 242, 19, 0, 0, 0, 0, 0,
            ];

            const BULLSHARK_PACKAGE: &[u8] = &[
                1, 135, 35, 29, 28, 138, 126, 114, 145, 204, 122, 145, 8, 244, 199, 188, 26, 10,
                28, 14, 182, 55, 91, 91, 97, 10, 245, 202, 35, 223, 14, 140, 86, 1, 0, 0, 0, 0, 0,
                0, 0, 1, 9, 98, 117, 108, 108, 115, 104, 97, 114, 107, 162, 6, 161, 28, 235, 11, 6,
                0, 0, 0, 10, 1, 0, 12, 2, 12, 36, 3, 48, 61, 4, 109, 12, 5, 121, 137, 1, 7, 130, 2,
                239, 1, 8, 241, 3, 96, 6, 209, 4, 82, 10, 163, 5, 5, 12, 168, 5, 75, 0, 7, 1, 16,
                2, 9, 2, 21, 2, 22, 2, 23, 0, 0, 2, 0, 1, 3, 7, 1, 0, 0, 2, 1, 12, 1, 0, 1, 2, 2,
                12, 1, 0, 1, 2, 4, 12, 1, 0, 1, 4, 5, 2, 0, 5, 6, 7, 0, 0, 12, 0, 1, 0, 0, 13, 2,
                1, 0, 0, 8, 3, 1, 0, 1, 20, 7, 8, 1, 0, 2, 8, 18, 19, 1, 0, 2, 10, 10, 11, 1, 2, 2,
                14, 17, 1, 1, 0, 3, 17, 7, 1, 1, 12, 3, 18, 16, 1, 1, 12, 4, 19, 13, 14, 0, 5, 15,
                5, 6, 0, 3, 6, 5, 9, 7, 12, 8, 15, 6, 9, 4, 9, 2, 8, 0, 7, 8, 5, 0, 4, 7, 11, 4, 1,
                8, 0, 3, 5, 7, 8, 5, 2, 7, 11, 4, 1, 8, 0, 11, 2, 1, 8, 0, 2, 11, 3, 1, 8, 0, 11,
                4, 1, 8, 0, 1, 10, 2, 1, 8, 6, 1, 9, 0, 1, 11, 1, 1, 9, 0, 1, 8, 0, 7, 9, 0, 2, 10,
                2, 10, 2, 10, 2, 11, 1, 1, 8, 6, 7, 8, 5, 2, 11, 4, 1, 9, 0, 11, 3, 1, 9, 0, 1, 11,
                3, 1, 8, 0, 1, 6, 8, 5, 1, 5, 1, 11, 4, 1, 8, 0, 2, 9, 0, 5, 4, 7, 11, 4, 1, 9, 0,
                3, 5, 7, 8, 5, 2, 7, 11, 4, 1, 9, 0, 11, 2, 1, 9, 0, 1, 3, 9, 66, 85, 76, 76, 83,
                72, 65, 82, 75, 4, 67, 111, 105, 110, 12, 67, 111, 105, 110, 77, 101, 116, 97, 100,
                97, 116, 97, 6, 79, 112, 116, 105, 111, 110, 11, 84, 114, 101, 97, 115, 117, 114,
                121, 67, 97, 112, 9, 84, 120, 67, 111, 110, 116, 101, 120, 116, 3, 85, 114, 108, 9,
                98, 117, 108, 108, 115, 104, 97, 114, 107, 4, 98, 117, 114, 110, 4, 99, 111, 105,
                110, 15, 99, 114, 101, 97, 116, 101, 95, 99, 117, 114, 114, 101, 110, 99, 121, 11,
                100, 117, 109, 109, 121, 95, 102, 105, 101, 108, 100, 4, 105, 110, 105, 116, 4,
                109, 105, 110, 116, 17, 109, 105, 110, 116, 95, 97, 110, 100, 95, 116, 114, 97,
                110, 115, 102, 101, 114, 21, 110, 101, 119, 95, 117, 110, 115, 97, 102, 101, 95,
                102, 114, 111, 109, 95, 98, 121, 116, 101, 115, 6, 111, 112, 116, 105, 111, 110,
                20, 112, 117, 98, 108, 105, 99, 95, 102, 114, 101, 101, 122, 101, 95, 111, 98, 106,
                101, 99, 116, 15, 112, 117, 98, 108, 105, 99, 95, 116, 114, 97, 110, 115, 102, 101,
                114, 6, 115, 101, 110, 100, 101, 114, 4, 115, 111, 109, 101, 8, 116, 114, 97, 110,
                115, 102, 101, 114, 10, 116, 120, 95, 99, 111, 110, 116, 101, 120, 116, 3, 117,
                114, 108, 135, 35, 29, 28, 138, 126, 114, 145, 204, 122, 145, 8, 244, 199, 188, 26,
                10, 28, 14, 182, 55, 91, 91, 97, 10, 245, 202, 35, 223, 14, 140, 86, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 2, 10, 2, 10, 9, 66, 85, 76, 76, 83, 72, 65, 82, 75, 10, 2, 20, 19, 66, 117,
                108, 108, 32, 83, 104, 97, 114, 107, 32, 83, 117, 105, 70, 114, 101, 110, 115, 10,
                2, 1, 0, 10, 2, 39, 38, 104, 116, 116, 112, 115, 58, 47, 47, 105, 46, 105, 98, 98,
                46, 99, 111, 47, 104, 87, 89, 50, 87, 53, 120, 47, 98, 117, 108, 108, 115, 104, 97,
                114, 107, 46, 112, 110, 103, 0, 2, 1, 11, 1, 0, 0, 0, 0, 4, 20, 11, 0, 49, 6, 7, 0,
                7, 1, 7, 2, 7, 3, 17, 10, 56, 0, 10, 1, 56, 1, 12, 2, 12, 3, 11, 2, 56, 2, 11, 3,
                11, 1, 46, 17, 9, 56, 3, 2, 1, 1, 4, 0, 1, 6, 11, 0, 11, 1, 11, 2, 11, 3, 56, 4, 2,
                2, 1, 4, 0, 1, 5, 11, 0, 11, 1, 56, 5, 1, 2, 0, 1, 9, 98, 117, 108, 108, 115, 104,
                97, 114, 107, 9, 66, 85, 76, 76, 83, 72, 65, 82, 75, 135, 35, 29, 28, 138, 126,
                114, 145, 204, 122, 145, 8, 244, 199, 188, 26, 10, 28, 14, 182, 55, 91, 91, 97, 10,
                245, 202, 35, 223, 14, 140, 86, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 2, 4, 0, 0, 0, 0, 0, 0, 0, 3, 32, 87, 145, 191, 231, 147, 185,
                46, 159, 240, 181, 95, 126, 236, 65, 154, 55, 16, 196, 229, 218, 47, 59, 99, 197,
                13, 89, 18, 159, 205, 129, 112, 131, 112, 192, 126, 0, 0, 0, 0, 0,
            ];

            for fixture in [SUI_COIN, SUI_STAKE, NFT, FUD_COIN, BULLSHARK_PACKAGE] {
                let object: Object = bcs::from_bytes(fixture).unwrap();
                assert_eq!(bcs::to_bytes(&object).unwrap(), fixture);

                let json = serde_json::to_string_pretty(&object).unwrap();
                println!("{json}");
                assert_eq!(object, serde_json::from_str(&json).unwrap());
            }
        }
    }
}
