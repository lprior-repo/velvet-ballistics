use vb_compile::{CompileError, compile_workflow};
use vb_core::CompiledNodeKind;

const SOURCE: &str = "version: velvet-ballistics/v1
name: collect_bounds
when:
  manual: {}
steps:
  - id: bounded
    collect:
      variable: batch
      source: \"0\"
      pages: 2
      items: 1
      steps:
        - id: copy
          set:
            output: copied
            value: \"$batch\"
  - id: done
    finish:
      result: \"$step.bounded\"
";

fn reject(source: &str, expected: &str) -> Result<(), String> {
    match compile_workflow(source.as_bytes()) {
        Ok(_) => Err("expected compile error but got Ok".to_string()),
        Err(errors) => {
            let found = errors.iter().any(|e| {
                matches!(e, CompileError::StepFieldShape { field: "collect", expected: actual, .. } if *actual == expected)
            });
            if found {
                Ok(())
            } else {
                Err(format!(
                    "expected field error on collect with '{}', got {:?}",
                    expected, errors
                ))
            }
        }
    }
}

#[test]
fn both_bounds_omitted() -> Result<(), String> {
    let src = SOURCE
        .replace("      pages: 2\n", "")
        .replace("      items: 1\n", "");
    reject(src.as_str(), "explicit positive pages or items limit")
}

#[test]
fn pages_zero() -> Result<(), String> {
    let src = SOURCE.replace("pages: 2", "pages: 0");
    reject(src.as_str(), "explicit positive pages or items limit")
}

#[test]
fn items_zero() -> Result<(), String> {
    let src = SOURCE.replace("items: 1", "items: 0");
    reject(src.as_str(), "explicit positive pages or items limit")
}

#[test]
fn pages_items_overflow() -> Result<(), String> {
    let src = SOURCE
        .replace("pages: 2", "pages: 4294967295")
        .replace("items: 1", "items: 2");
    reject(src.as_str(), "pages * items must fit u32")
}

#[test]
fn collect_bounds_compile() -> Result<(), String> {
    let workflow = compile_workflow(SOURCE.as_bytes())
        .map_err(|e| format!("expected Ok but got error: {:?}", e))?;
    let parts = workflow.to_parts();
    let mut found_limit = None;
    let mut found_page_size = None;
    for node in parts.nodes.iter() {
        if let CompiledNodeKind::CollectStart {
            limit, page_size, ..
        } = &node.kind
        {
            found_limit = Some(*limit);
            found_page_size = Some(*page_size);
        }
    }
    let limit = found_limit.ok_or("missing CollectStart node")?;
    let page_size = found_page_size.ok_or("missing CollectStart node")?;
    if limit != 2 {
        return Err(format!("expected limit 2, got {}", limit));
    }
    if page_size != 1 {
        return Err(format!("expected page_size 1, got {}", page_size));
    }
    Ok(())
}
