use nkcore_debug::api_name_of;

fn generic<T>() {}

fn main() {
    let _name = api_name_of!(generic::<Item = u32>());
}
