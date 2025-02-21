use support::{decl_module, decl_storage, ensure, StorageValue, StorageMap, dispatch::Result, Parameter};
use sr_primitives::traits::{SimpleArithmetic, Bounded, CheckedAdd, CheckedSub};
use codec::{Encode, Decode};
use runtime_io::blake2_128;
use system::ensure_signed;
use rstd::result;

pub trait Trait: system::Trait {
    type KittyIndex: Parameter + SimpleArithmetic + Bounded + Default + Copy;
}

#[derive(Encode, Decode)]
pub struct Kitty(pub [u8; 16]);

decl_storage! {
	trait Store for Module<T: Trait> as Kitties {
		/// Stores all the kitties, key is the kitty id / index
		pub Kitties get(kitty): map T::KittyIndex => Option<Kitty>;
		/// Stores the total number of kitties. i.e. the next kitty index
		pub KittiesCount get(kitties_count): T::KittyIndex;

		/// Get kitty ID by account ID and user kitty index
		pub OwnedKitties get(owned_kitties): map (T::AccountId, T::KittyIndex) => T::KittyIndex;
		/// Get number of kitties by account ID
		pub OwnedKittiesCount get(owned_kitties_count): map T::AccountId => T::KittyIndex;

		pub KittyOwner get(owner_of): map T::KittyIndex => Option<T::AccountId>;
		pub OwnedKittiesIndex: map T::KittyIndex => T::KittyIndex ;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		/// Create a new kitty
		pub fn create(origin) -> Result {
			let sender = ensure_signed(origin)?;

			// 作业：重构create方法，避免重复代码
			let kitty_id = Self::next_kitty_id()?;

			// Generate a random 128bit value
			let dna = Self::random_value(&sender);

			// Create and store kitty
			let kitty = Kitty(dna);
            Self::insert_kitty(sender.clone(), kitty_id, kitty);

			Ok(())
		}

		/// Breed kitties
		pub fn breed(origin, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) -> Result {
			let sender = ensure_signed(origin)?;
			Self::do_breed(sender, kitty_id_1, kitty_id_2)?;
			Ok(())
		}

		/// Transfer kitties
		pub fn transfer(origin, to: T::AccountId, kitty_id: T::KittyIndex) -> Result {
			let sender = ensure_signed(origin)?;
			let owner = Self::owner_of(kitty_id).ok_or("No owner for this kitty")?;
            ensure!(owner == sender, "You do not own this kitty");

			Self::do_transfer(sender, to, kitty_id);
			Ok(())
		}
	}
}

fn combine_dna(dna1: u8, dna2: u8, selector: u8) -> u8 {
    // 作业：实现combine_dna
    // 伪代码：
    // selector.map_bits(|bit, index| if (bit == 1) { dna1 & (1 << index) } else { dna2 & (1 << index) })
    // 注意 map_bits这个方法不存在。只要能达到同样效果，不局限算法
    // 测试数据：dna1 = 0b11110000, dna2 = 0b11001100, selector = 0b10101010, 返回值 0b11100100
    (dna1 & selector) | (dna2 & (!selector))
}

impl<T: Trait> Module<T> {
    fn random_value(sender: &T::AccountId) -> [u8; 16] {
        let payload = (<system::Module<T>>::random_seed(), sender, <system::Module<T>>::extrinsic_index(), <system::Module<T>>::block_number());
        payload.using_encoded(blake2_128)
    }

    fn next_kitty_id() -> result::Result<T::KittyIndex, &'static str> {
        let kitty_id = Self::kitties_count();
        if kitty_id == T::KittyIndex::max_value() {
            return Err("Kitties count overflow");
        }
        Ok(kitty_id)
    }

    fn insert_kitty(owner: T::AccountId, kitty_id: T::KittyIndex, kitty: Kitty) {
        // Create and store kitty
        <Kitties<T>>::insert(kitty_id, kitty);
        <KittiesCount<T>>::put(kitty_id + 1.into());

        // Store the ownership information
        let user_kitties_id = Self::owned_kitties_count(owner.clone());
        <OwnedKitties<T>>::insert((owner.clone(), user_kitties_id), kitty_id);
        <OwnedKittiesCount<T>>::insert(owner.clone(), user_kitties_id + 1.into());

        <OwnedKittiesIndex<T>>::insert(kitty_id, user_kitties_id);
        <KittyOwner<T>>::insert(kitty_id, owner.clone());
    }

    fn do_breed(sender: T::AccountId, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) -> Result {
        let kitty1 = Self::kitty(kitty_id_1);
        let kitty2 = Self::kitty(kitty_id_2);

        ensure!(kitty1.is_some(), "Invalid kitty_id_1");
        ensure!(kitty2.is_some(), "Invalid kitty_id_2");
        ensure!(kitty_id_1 != kitty_id_2, "Needs different parent");

        let kitty_id = Self::next_kitty_id()?;

        let kitty1_dna = kitty1.unwrap().0;
        let kitty2_dna = kitty2.unwrap().0;

        // Generate a random 128bit value
        let selector = Self::random_value(&sender);
        let mut new_dna = [0u8; 16];

        // Combine parents and selector to create new kitty
        for i in 0..kitty1_dna.len() {
            new_dna[i] = combine_dna(kitty1_dna[i], kitty2_dna[i], selector[i]);
        }

        Self::insert_kitty(sender, kitty_id, Kitty(new_dna));

        Ok(())
    }

    fn do_transfer(from: T::AccountId, to: T::AccountId, kitty_id: T::KittyIndex) -> Result {
        let owned_kitty_count_from = Self::owned_kitties_count(&from);
        let owned_kitty_count_to = Self::owned_kitties_count(&to);

        // 判断从 from 账户的 owned_kitty_count 中减去 kitty 时是否会出现溢出
        let new_owned_kitty_count_from = owned_kitty_count_from.checked_sub(&1.into())
            .ok_or("Transfer causes underflow of 'from' kitty")?;
        // 判断将 kitty 添加到 to 账户的 owned_kitty_count 时是否会出现溢出
        let new_owned_kitty_count_to = owned_kitty_count_to.checked_add(&1.into())
            .ok_or("Transfer causes overflow of 'to' kitty")?;

        // 要转移猫的索引
        let kitty_index = <OwnedKittiesIndex<T>>::get(kitty_id);
        // 判断要转移猫的索引是否为最未位索引
        if kitty_index != new_owned_kitty_count_from {
            // 得到老用户最未位索引对应的猫
            let last_kitty_id = <OwnedKitties<T>>::get((from.clone(), new_owned_kitty_count_from));
            // 将老用户最未位索引对应的猫放入要转移猫对应的索引位置
            <OwnedKitties<T>>::insert((from.clone(), kitty_index), last_kitty_id);
            // 调整原最未位索引对应的猫的新索引位置
            <OwnedKittiesIndex<T>>::insert(last_kitty_id, kitty_index);
        }

        // 变更猫所属权
        <KittyOwner<T>>::insert(&kitty_id, &to);
        // 变更猫索引位置
        <OwnedKittiesIndex<T>>::insert(kitty_id, owned_kitty_count_to);

        // 移除 from 账户最未位索引对应的猫
        <OwnedKitties<T>>::remove((from.clone(), new_owned_kitty_count_from));
        // to 账户最未位放入猫
        <OwnedKitties<T>>::insert((to.clone(), owned_kitty_count_to), kitty_id);

        // from 账户猫数量减1
        <OwnedKittiesCount<T>>::insert(&from, new_owned_kitty_count_from);
        // to 账户猫数量加1
        <OwnedKittiesCount<T>>::insert(&to, new_owned_kitty_count_to);

        Ok(())
    }
}
