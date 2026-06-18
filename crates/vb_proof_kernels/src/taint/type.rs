//! Non-Verus Taint type: three-element lattice Clean < DerivedFromSecret < Secret.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Taint {
    Clean,
    DerivedFromSecret,
    Secret,
}

impl Taint {
    /// Returns the lattice rank: Clean=0, DerivedFromSecret=1, Secret=2.
    #[must_use]
    pub fn rank(&self) -> u8 {
        match self {
            Taint::Clean => 0,
            Taint::DerivedFromSecret => 1,
            Taint::Secret => 2,
        }
    }
}
