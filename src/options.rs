use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(unused)]
pub struct Options {
    pub transform_on: bool,

    pub optimize: bool,

    pub custom_element_patterns: Vec<String>,

    #[serde(default = "default_true")]
    pub merge_props: bool,

    #[serde(default = "default_true")]
    pub enable_object_slots: bool,

    pub pragma: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            transform_on: false,
            optimize: false,
            custom_element_patterns: Default::default(),
            merge_props: true,
            enable_object_slots: true,
            pragma: None,
        }
    }
}

fn default_true() -> bool {
    true
}
