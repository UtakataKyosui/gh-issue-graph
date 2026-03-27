use anyhow::Result;
use serde_json::Value;

/// jaq を使って JSON 値に jq フィルタを適用する
pub fn apply_jq(value: &Value, filter_expr: &str) -> Result<Value> {
    use jaq_core::{load::{Arena, File, Loader}, Compiler, Ctx, RcIter};
    use jaq_json::Val;

    let program = File { code: filter_expr, path: () };

    let loader = Loader::new(jaq_std::defs().chain(jaq_json::defs()));
    let arena = Arena::default();

    let modules = loader.load(&arena, program)
        .map_err(|e| anyhow::anyhow!("jq load error: {:?}", e))?;

    let filter = Compiler::default()
        .with_funs(jaq_std::funs().chain(jaq_json::funs()))
        .compile(modules)
        .map_err(|e| anyhow::anyhow!("jq compile error: {:?}", e))?;

    let inputs = RcIter::new(core::iter::empty());
    let input_val = Val::from(value.clone());
    let ctx = Ctx::new(vec![], &inputs);
    let outputs: Vec<Value> = filter
        .run((ctx, input_val))
        .filter_map(|r| r.ok().map(|v| Value::from(v)))
        .collect();

    Ok(if outputs.len() == 1 {
        outputs.into_iter().next().unwrap()
    } else {
        Value::Array(outputs)
    })
}
