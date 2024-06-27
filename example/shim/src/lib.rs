
/*
#[no_mangle]
#[link_section = ".init_array"]
pub unsafe extern "C" fn example_chook_hook() -> () {
*/

#[link_section = ".init_array"]
fn example_chook_hook() {
    eprintln!("init function running!");
    chook_shim::run(|arg: types::Arg| -> types::Ret {
        eprintln!("{}", arg.print_this_string);
        eprintln!("{}", arg.print_this_int);

        types::Ret {
            trimmed_string: String::from(arg.trim_this_string.trim()),
            doubled_int: arg.double_this_int * 2,
        }
    })
}
