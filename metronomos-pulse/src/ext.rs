#[cfg(feature = "ext-reqwest")]
mod ext_reqwest {
    use reqwest::Client;

    use crate::value::CustomPulseValue;

    impl CustomPulseValue for Client {
        const NAME: &'static str = "reqwest::Client";
    }
}
