use nkcore_debug::api_name_of;

fn main() {
    let closure = || {};
    let _name = api_name_of!((closure)());
}
