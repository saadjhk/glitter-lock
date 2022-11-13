use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction::{transfer}, program_pack::{Pack}
};
use crate::{instruction::GlitterLockInstruction, state::GlitterLock};
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum LockError {
    #[error("Early Unlock")]
    EarlyUnlock,
}
pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = GlitterLockInstruction::unpack(instruction_data)?;

        match instruction {
            GlitterLockInstruction::Lock { amount } => {
                msg!("Instruction: Lock");
                Self::process_lock(accounts, amount, program_id)
            }
            GlitterLockInstruction::Release => {
                msg!("Instruction: Release");
                Self::process_unlock(accounts, program_id)
            }
        }
    }

    fn process_lock(accounts: &[AccountInfo], amount: u64, program_id: &Pubkey) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let locker = next_account_info(accounts_iter)?;
        let locker_pda = next_account_info(accounts_iter)?;

        if locker_pda.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }

        if !locker.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let lamports = locker.lamports();

        if lamports.lt(&amount) {
            return Err(ProgramError::InsufficientFunds);
        }

        let mut lock = GlitterLock::unpack_unchecked(&locker_pda.data.borrow())?;

        if lock.is_initialized {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        let clock = Clock::default();
        lock.amount = amount;
        lock.is_initialized = true;
        lock.locker_public_key = locker_pda.key.clone();
        lock.lock_time = clock.unix_timestamp;
        transfer(locker.key, &locker_pda.key, amount);

        Ok(())
    }

    fn process_unlock(
        accounts: &[AccountInfo], program_id: &Pubkey
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let locker = next_account_info(accounts_iter)?;
        let locker_pda = next_account_info(accounts_iter)?;

        if locker_pda.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }

        if !locker.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let mut lock = GlitterLock::unpack_unchecked(&locker_pda.data.borrow())?;

        let one_min = 60;
        let current_time = Clock::default().unix_timestamp;
        if lock.lock_time + one_min < current_time {
            return Err(ProgramError::Custom(LockError::EarlyUnlock as u32))
        }

        lock.is_initialized = false;
        transfer(locker_pda.key, &lock.locker_public_key, lock.amount);

        Ok(())
    }
}
