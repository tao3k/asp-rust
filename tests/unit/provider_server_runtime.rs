use super::admit_provider_server_argv;
use std::ffi::OsString;

#[test]
fn provider_process_surface_admits_only_catalog_declared_serve() {
    assert!(admit_provider_server_argv([OsString::from("serve")]).is_ok());
    for legacy in ["search", "query", "check", "projection", "agent"] {
        let error = admit_provider_server_argv([OsString::from(legacy)])
            .expect_err("legacy provider-local command must be rejected");
        assert!(error.contains("only admitted process argument is `serve`"));
    }
    assert!(admit_provider_server_argv([]).is_err());
    assert!(
        admit_provider_server_argv([OsString::from("serve"), OsString::from("--legacy")]).is_err()
    );
}
