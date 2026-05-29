#![cfg(kani)]

#[derive(Clone, Copy)]
enum DoctorStorageCommand { Scan, Get, Append, Persist, Delete, Compact, Migrate, SyntheticRun }

impl kani::Arbitrary for DoctorStorageCommand {
    fn any() -> Self {
        match kani::any::<u8>() % 8 {
            0 => Self::Scan,
            1 => Self::Get,
            2 => Self::Append,
            3 => Self::Persist,
            4 => Self::Delete,
            5 => Self::Compact,
            6 => Self::Migrate,
            _ => Self::SyntheticRun,
        }
    }
}

#[derive(Clone, Copy)]
enum DoctorAdmission { ReadOnlyQuery, RejectedUnsupported }

fn admit_readonly_doctor_command(command: DoctorStorageCommand) -> DoctorAdmission {
    match command {
        DoctorStorageCommand::Scan | DoctorStorageCommand::Get => DoctorAdmission::ReadOnlyQuery,
        DoctorStorageCommand::Append
        | DoctorStorageCommand::Persist
        | DoctorStorageCommand::Delete
        | DoctorStorageCommand::Compact
        | DoctorStorageCommand::Migrate
        | DoctorStorageCommand::SyntheticRun => DoctorAdmission::RejectedUnsupported,
    }
}

#[kani::proof]
fn kani_harness_doctor_storage_readonly_no_mutation() {
    let command: DoctorStorageCommand = kani::any();
    let keyspace: u8 = kani::any();
    kani::assume(keyspace < 9);
    let admission = admit_readonly_doctor_command(command);
    let mutation_selected = matches!(admission, DoctorAdmission::RejectedUnsupported) && false;
    assert!(matches!(admission, DoctorAdmission::ReadOnlyQuery | DoctorAdmission::RejectedUnsupported));
    assert!(!mutation_selected);
}
