use tauri::Url;

pub fn trusted_navigation(url: &Url, development_url: Option<&Url>) -> bool {
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    let embedded = url.port().is_none()
        && ((url.scheme() == "tauri" && url.host_str() == Some("localhost"))
            || (matches!(url.scheme(), "http" | "https")
                && url.host_str() == Some("tauri.localhost")));
    embedded || development_url.is_some_and(|dev| url.origin() == dev.origin())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_remote_navigation_and_origin_lookalikes() {
        for allowed in [
            "tauri://localhost/index.html",
            "http://tauri.localhost/",
            "https://tauri.localhost/",
        ] {
            assert!(trusted_navigation(&Url::parse(allowed).unwrap(), None));
        }
        for blocked in [
            "https://github.com/hazavi/AudioFlow",
            "https://tauri.localhost.example.com/",
            "https://tauri.localhost@evil.example/",
            "http://tauri.localhost:8080/",
            "http://localhost:1420/",
            "file:///C:/Windows/test.html",
            "data:text/html,test",
            "javascript:alert(1)",
            "tauri://evil.example/",
        ] {
            assert!(
                !trusted_navigation(&Url::parse(blocked).unwrap(), None),
                "{blocked}"
            );
        }
    }

    #[test]
    fn development_navigation_requires_the_configured_origin() {
        let development = Url::parse("http://localhost:1420/").unwrap();
        assert!(trusted_navigation(&development, Some(&development)));
        for blocked in [
            "http://localhost:1421/",
            "https://localhost:1420/",
            "http://localhost.evil.example:1420/",
        ] {
            assert!(!trusted_navigation(
                &Url::parse(blocked).unwrap(),
                Some(&development)
            ));
        }
    }
}
