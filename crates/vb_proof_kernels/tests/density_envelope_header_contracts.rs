#![forbid(unsafe_code)]

use vb_proof_kernels::envelope_header::{
    EnvelopeHeader, HEADER_LEN, ValidationError, ValidationResult, compute_header_crc,
    validate_header_before_alloc, validate_header_crc,
};

macro_rules! ktest {
    ($(#[$attr:meta])* $name:ident, $body:block) => {
        $(#[$attr])*
        fn $name() $body
    };
}

ktest!(
    #[test]
    envelope_header_len_constant_is_sixty,
    {
        assert_eq!(HEADER_LEN, 60);
    }
);

ktest!(
    #[test]
    envelope_new_uses_magic_value,
    {
        assert_eq!(EnvelopeHeader::new().magic, EnvelopeHeader::MAGIC_VALUE);
    }
);

ktest!(
    #[test]
    envelope_new_uses_version_one,
    {
        assert_eq!(EnvelopeHeader::new().version, 1);
    }
);

ktest!(
    #[test]
    envelope_new_has_zero_payload_len,
    {
        assert_eq!(EnvelopeHeader::new().payload_len(), 0);
    }
);

ktest!(
    #[test]
    envelope_default_matches_new,
    {
        assert_eq!(EnvelopeHeader::default(), EnvelopeHeader::new());
    }
);

ktest!(
    #[test]
    envelope_validate_magic_accepts_new,
    {
        assert!(EnvelopeHeader::new().validate_magic());
    }
);

ktest!(
    #[test]
    envelope_validate_magic_rejects_changed_magic,
    {
        let mut header = EnvelopeHeader::new();
        header.magic = 0;
        assert!(!header.validate_magic());
    }
);

ktest!(
    #[test]
    envelope_validate_header_len_returns_true,
    {
        assert!(EnvelopeHeader::new().validate_header_len());
    }
);

ktest!(
    #[test]
    envelope_payload_len_combines_high_and_low,
    {
        let mut header = EnvelopeHeader::new();
        header.payload_len_hi = 1;
        header.payload_len_u32 = 2;
        assert_eq!(header.payload_len(), 4_294_967_298);
    }
);

ktest!(
    #[test]
    envelope_validate_payload_len_accepts_equal_max,
    {
        let header = EnvelopeHeader::new();
        assert!(header.validate_payload_len(0));
    }
);

ktest!(
    #[test]
    envelope_validate_payload_len_rejects_over_max,
    {
        let mut header = EnvelopeHeader::new();
        header.payload_len_u32 = 2;
        assert!(!header.validate_payload_len(1));
    }
);

ktest!(
    #[test]
    envelope_validate_before_alloc_accepts_new_header,
    {
        assert_eq!(
            EnvelopeHeader::new().validate_before_alloc(0),
            ValidationResult::Ok
        );
    }
);

ktest!(
    #[test]
    envelope_validate_before_alloc_rejects_bad_magic,
    {
        let mut header = EnvelopeHeader::new();
        header.magic = 0;
        assert_eq!(
            header.validate_before_alloc(0),
            ValidationResult::Err(ValidationError::InvalidMagic)
        );
    }
);

ktest!(
    #[test]
    envelope_validate_before_alloc_rejects_large_payload,
    {
        let mut header = EnvelopeHeader::new();
        header.payload_len_u32 = 2;
        assert_eq!(
            header.validate_before_alloc(1),
            ValidationResult::Err(ValidationError::PayloadTooLarge)
        );
    }
);

ktest!(
    #[test]
    envelope_free_function_delegates_validation,
    {
        assert_eq!(
            validate_header_before_alloc(&EnvelopeHeader::new(), 0),
            ValidationResult::Ok
        );
    }
);

ktest!(
    #[test]
    envelope_crc_stub_returns_zero,
    {
        assert_eq!(compute_header_crc(&EnvelopeHeader::new()), 0);
    }
);

ktest!(
    #[test]
    envelope_crc_stub_validates_header,
    {
        assert!(validate_header_crc(&EnvelopeHeader::new()));
    }
);
