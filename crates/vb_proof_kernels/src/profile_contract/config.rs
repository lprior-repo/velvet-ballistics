//! ProfileConfig — a single profile definition.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Represents one `[profile.<name>]` section from root Cargo.toml.

use crate::profile_contract::types::{ProfileName, ProfileKey, SettingValue, StrVal};

/// A complete profile definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileConfig {
    /// The validated profile name.
    pub name: ProfileName,
    /// The parent profile for inheritance (from `inherits` key). None if no inheritance.
    pub inherits: Option<ProfileName>,
    /// Explicit key-value pairs from the TOML table.
    pub settings: Vec<(ProfileKey, SettingValue)>,
}

impl ProfileConfig {
    /// Construct a ProfileConfig.
    ///
    /// `inherits` is resolved from settings (if the Inherits key has value "release").
    /// Keys and values are validated at construction.
    pub fn new(
        name: ProfileName,
        settings: Vec<(ProfileKey, SettingValue)>,
    ) -> Self {
        // Extract inherits target by scanning settings.
        // This is done explicitly without iterator chaining to keep
        // Kani unrolling predictable.
        let mut inherits = None;
        for (k, v) in &settings {
            if *k == ProfileKey::Inherits {
                if let SettingValue::String(s) = v {
                    match s {
                        StrVal::Release => inherits = Some(ProfileName::Release),
                        _ => { /* unknown inherits target — deferred */ }
                    }
                }
            }
        }

        Self { name, inherits, settings }
    }

    /// Look up a setting value by key. Returns None if not present.
    pub fn get(&self, key: ProfileKey) -> Option<&SettingValue> {
        for (k, v) in &self.settings {
            if *k == key {
                return Some(v);
            }
        }
        None
    }

    /// Returns true if the profile has explicit `inherits` pointing at `parent`.
    pub fn inherits_from(&self, parent: ProfileName) -> bool {
        self.inherits == Some(parent)
    }
}

#[cfg(kani)]
use crate::profile_contract::types::DebugMode;

#[cfg(kani)]
impl kani::Arbitrary for ProfileConfig {
    fn any() -> Self {
        let name: ProfileName = kani::any();
        let has_inherits: bool = kani::any();
        let num_settings: u8 = kani::any();
        let num_settings = num_settings.min(12); // bounded to 12 keys max

        let mut settings = Vec::with_capacity(num_settings as usize);
        let mut has_inherits_key = false;

        for _ in 0..num_settings {
            let key: ProfileKey = kani::any();
            let value: SettingValue = kani::any();
            if key == ProfileKey::Inherits { has_inherits_key = true; }
            settings.push((key, value));
        }

        // Ensure inherits consistency: if has_inherits is true, there must be an Inherits key
        if has_inherits && !has_inherits_key {
            // Force an inherits = "release" setting
            settings.push((ProfileKey::Inherits, SettingValue::String(StrVal::Release)));
        }

        Self::new(name, settings)
    }
}

#[cfg(kani)]
impl kani::Arbitrary for ProfileName {
    fn any() -> Self {
        let idx: u8 = kani::any();
        match idx % 6 {
            0 => Self::Release,
            1 => Self::Bench,
            2 => Self::Hardened,
            3 => Self::Fuzz,
            4 => Self::Test,
            _ => Self::Dev,
        }
    }
}

#[cfg(kani)]
impl kani::Arbitrary for ProfileKey {
    fn any() -> Self {
        let idx: u8 = kani::any();
        match idx % 9 {
            0 => Self::OptLevel,
            1 => Self::Lto,
            2 => Self::CodegenUnits,
            3 => Self::Strip,
            4 => Self::Debug,
            5 => Self::DebugAssertions,
            6 => Self::OverflowChecks,
            7 => Self::Panic,
            _ => Self::Inherits,
        }
    }
}

#[cfg(kani)]
impl kani::Arbitrary for SettingValue {
    fn any() -> Self {
        let variant: u8 = kani::any();
        match variant % 5 {
            0 => SettingValue::Bool(kani::any()),
            1 => {
                let s: StrVal = kani::any();
                SettingValue::String(s)
            }
            2 => SettingValue::U8(kani::any()),
            3 => SettingValue::U16(kani::any()),
            _ => {
                let dm: DebugMode = kani::any();
                SettingValue::DebugMode(dm)
            }
        }
    }
}

#[cfg(kani)]
impl kani::Arbitrary for StrVal {
    fn any() -> Self {
        let idx: u8 = kani::any();
        match idx % 12 {
            0 => Self::Thin,
            1 => Self::Fat,
            2 => Self::Off,
            3 => Self::True,
            4 => Self::False,
            5 => Self::None_,
            6 => Self::Symbols,
            7 => Self::Debuginfo,
            8 => Self::Release,
            9 => Self::Unwind,
            10 => Self::Abort,
            _ => Self::Other,
        }
    }
}

#[cfg(kani)]
impl kani::Arbitrary for DebugMode {
    fn any() -> Self {
        let idx: u8 = kani::any();
        match idx % 3 {
            0 => Self::False,
            1 => Self::True,
            _ => Self::LineTablesOnly,
        }
    }
}
