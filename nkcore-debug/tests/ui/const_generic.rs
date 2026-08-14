use nkcore_debug::api_name_of;

fn const_generic<const N: usize>() {}

fn main() {
    let _name = api_name_of!(const_generic::<1>());
}
