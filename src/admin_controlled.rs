use crate::sdk;

pub type PausedMask = u8;

pub(crate) trait AdminControlled<S: sdk::Env + sdk::IO> {
    /// Returns true if the current account is owner
    fn is_owner(&self, s: &S) -> bool {
        s.current_account_id() == s.predecessor_account_id()
    }

    /// Return the current mask representing all paused events.
    fn get_paused(&self, s: &S) -> PausedMask;

    /// Update mask with all paused events.
    /// Implementor is responsible for guaranteeing that this function can only be
    /// called by owner of the contract.
    fn set_paused(&mut self, paused: PausedMask, s: &mut S);

    /// Return if the contract is paused for the current flag and user
    fn is_paused(&self, flag: PausedMask, s: &S) -> bool {
        (self.get_paused(s) & flag) != 0 && !self.is_owner(s)
    }

    /// Asserts the passed paused flag is not set. Panics with "ERR_PAUSED" if the flag is set.
    fn assert_not_paused(&self, flag: PausedMask, s: &S) {
        assert!(!self.is_paused(flag, s), "ERR_PAUSED");
    }
}
