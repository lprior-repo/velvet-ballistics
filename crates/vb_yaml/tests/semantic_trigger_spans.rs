use vb_yaml::{SourceSpan, build_semantic_source_map, build_source_map};

#[test]
fn trigger_containers_select_exact_key_spans_after_repeated_unicode_text()
-> Result<(), Box<dyn std::error::Error>> {
    let prefix = "label: 'manual schedule event webhook λ'\nwhen:\n  ";
    for trigger in ["manual", "schedule", "event", "webhook"] {
        let yaml = format!("{prefix}{trigger}: {{}}\nother: {{}}\n");
        let map = build_semantic_source_map(&yaml)?;
        let end = prefix
            .len()
            .checked_add(trigger.len())
            .ok_or("byte offset")?;
        let end_col = 3usize.checked_add(trigger.len()).ok_or("column")?;
        let expected = SourceSpan::new(prefix.len(), end, 3, 3, 3, end_col);
        let actual = map.span_for_path(&format!("$.when.{trigger}"));
        assert_eq!(actual, Some(expected), "{trigger}");
        assert_eq!(
            yaml.get(expected.start_offset..expected.end_offset),
            Some(trigger)
        );
        assert_eq!(map.span_for_path("$.when.absent"), None);

        let container_start = format!("{prefix}{trigger}: {{}}\nother: ").len();
        let container_end = container_start.checked_add(1).ok_or("container offset")?;
        assert_eq!(
            map.span_for_path("$.other"),
            Some(SourceSpan::new(container_start, container_end, 4, 8, 4, 9))
        );
        assert_eq!(yaml.get(container_start..container_end), Some("{"));
    }
    Ok(())
}

#[test]
fn ordinary_source_map_uses_utf8_bytes_and_one_based_columns()
-> Result<(), Box<dyn std::error::Error>> {
    for newline in ["\n", "\r\n", "\r"] {
        let prefix = format!("title: 'λ'{newline}word: ");
        let yaml = format!("{prefix}κ{newline}");
        let map = build_source_map(&yaml)?;
        let end = prefix.len().checked_add("κ".len()).ok_or("byte offset")?;
        let expected = SourceSpan::new(prefix.len(), end, 2, 7, 2, 8);
        let matches: Vec<_> = map
            .iter()
            .filter_map(|(_, span)| {
                (yaml.get(span.start_offset..span.end_offset) == Some("κ")).then_some(span)
            })
            .collect();
        assert_eq!(matches, vec![expected], "{newline:?}");
        assert!(
            map.iter()
                .all(|(_, span)| yaml.get(span.start_offset..span.end_offset).is_some())
        );
    }
    Ok(())
}
