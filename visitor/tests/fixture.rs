use std::{fs, io::ErrorKind, path::PathBuf};
use swc_core::{
    common::{chain, Mark},
    ecma::{
        parser::{EsConfig, Syntax},
        transforms::{base::resolver, testing::test_fixture},
        visit::as_folder,
    },
};
use swc_vue_jsx_visitor::{Options, VueJsxTransformVisitor};

#[testing::fixture("tests/fixture/**/input.jsx")]
fn test(input: PathBuf) {
    let config = match fs::read_to_string(input.with_file_name("config.json")) {
        Ok(json) => serde_json::from_str(&json).unwrap(),
        Err(err) if err.kind() == ErrorKind::NotFound => Options {
            optimize: true,
            ..Default::default()
        },
        Err(err) => panic!("failed to read `config.json`: {err}"),
    };
    let output = input.with_file_name("output.js");

    test_fixture(
        Syntax::Es(EsConfig {
            jsx: true,
            ..Default::default()
        }),
        &|tester| {
            let unresolved_mark = Mark::new();
            chain!(
                resolver(unresolved_mark, Mark::new(), false),
                as_folder(VueJsxTransformVisitor::new(
                    config.clone(),
                    unresolved_mark,
                    Some(tester.comments.clone())
                ))
            )
        },
        &input,
        &output,
        Default::default(),
    )
}
