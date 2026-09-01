use super::admit_provider_server_argv;
use std::ffi::OsString;

#[test]
fn provider_process_surface_admits_only_catalog_declared_serve() {
    assert!(admit_provider_server_argv([OsString::from("serve")]).is_ok());
    assert!(admit_provider_server_argv([]).is_err());
    assert!(
        admit_provider_server_argv([OsString::from("serve"), OsString::from("unexpected")])
            .is_err()
    );
}
