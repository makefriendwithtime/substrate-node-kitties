use super::*;
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};

#[test]
fn create_test() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(KittiesModule::create(Origin::signed(1)));
		assert_eq!(KittiesModule::next_kitty_id(), Some(1));
	});
}

#[test]
fn create_kitties_overflow() {
	new_test_ext().execute_with(|| {
		NextKittyId::<Test>::put(u32::max_value());
		assert_noop!(KittiesModule::create(Origin::signed(1)), Error::<Test>::KittiesOverflow);
	});
}

#[test]
fn create_stake_not_enough() {
	new_test_ext().execute_with(|| {
		assert_noop!(KittiesModule::create(Origin::signed(666)), Error::<Test>::StakeNotEnough);
	});
}

#[test]
fn transfer_test() {
	new_test_ext().execute_with(|| {
		let _ = KittiesModule::create(Origin::signed(1));
		assert_ok!(KittiesModule::transfer(Origin::signed(1), 0, 2));
	});
}

#[test]
fn transfer_not_owner() {
	new_test_ext().execute_with(|| {
		let _ = KittiesModule::create(Origin::signed(1));
		assert_noop!(KittiesModule::transfer(Origin::signed(2), 0, 1), Error::<Test>::NotOwner);
	});
}

#[test]
fn breed_test() {
	new_test_ext().execute_with(|| {
		let _ = KittiesModule::create(Origin::signed(1));
		let _ = KittiesModule::create(Origin::signed(1));

		assert_ok!(KittiesModule::breed(Origin::signed(1), 0, 1));
		assert_eq!(KittiesModule::next_kitty_id(), Some(3));
	});
}

#[test]
fn breed_same_id() {
	new_test_ext().execute_with(|| {
		let _ = KittiesModule::create(Origin::signed(1));
		assert_noop!(KittiesModule::breed(Origin::signed(1), 0, 0), Error::<Test>::SameId);
	});
}

#[test]
fn breed_invalid_id() {
	new_test_ext().execute_with(|| {
		let _ = KittiesModule::create(Origin::signed(1));
		assert_noop!(KittiesModule::breed(Origin::signed(1), 0, 1), Error::<Test>::InvalidId);
	});
}

#[test]
fn breed_kitties_overflow() {
	new_test_ext().execute_with(|| {
		let _ = KittiesModule::create(Origin::signed(1));
		let _ = KittiesModule::create(Origin::signed(1));

		NextKittyId::<Test>::put(u32::max_value());

		assert_noop!(KittiesModule::breed(Origin::signed(1), 0, 1), Error::<Test>::KittiesOverflow);
	});
}
