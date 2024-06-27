use serde_derive::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Arg {
    pub print_this_string: String,
    pub trim_this_string: String,
    pub print_this_int: i64,
    pub double_this_int: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Ret {
    pub trimmed_string: String,
    pub doubled_int: i64,
}
