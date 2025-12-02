fn main() {
    // napi-build setup (only when napi feature is enabled)
    #[cfg(feature = "napi")]
    {
        extern crate napi_build;
        napi_build::setup();
    }
}

