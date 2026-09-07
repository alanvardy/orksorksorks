use serde::Serialize;
use std::fmt;

/// The central error type for orksorksorks.
///
/// `source` is a lowercase tag (e.g. `"io"`, `"toml::ser"`) and `message`
/// is the human-readable description.  `Display` owns all coloring —
/// callers must not pre-apply ANSI codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Error {
    pub message: String,
    pub source: String,
}

impl Error {
    /// Primary constructor.  Prefer `&format!(...)` for the message to
    /// avoid an unnecessary allocation.
    pub fn new(source: &str, message: &str) -> Self {
        Self {
            source: source.to_string(),
            message: message.to_string(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Error from {}:\n{}",
            crate::format::yellow_string(&self.source),
            crate::format::red_string(&self.message),
        )
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self {
            source: "io".to_string(),
            message: e.to_string(),
        }
    }
}

impl From<toml::ser::Error> for Error {
    fn from(e: toml::ser::Error) -> Self {
        Self {
            source: "toml::ser".to_string(),
            message: e.to_string(),
        }
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Self {
            source: "toml::de".to_string(),
            message: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn display_includes_source_and_message() {
        let err = Error::new("test_tag", "something broke");
        let displayed = format!("{err}");
        // Under cfg!(test) ANSI is stripped; check plain substrings
        assert!(displayed.contains("Error from test_tag"), "{displayed}");
        assert!(displayed.contains("something broke"), "{displayed}");
    }

    #[test]
    fn from_io_error_tags_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let err = Error::from(io_err);
        assert_eq!(err.source, "io");
        assert!(err.message.contains("no such file"));
    }

    #[test]
    fn from_toml_ser_error_tags_toml_ser() {
        // toml::ser::Error is hard to construct; use a struct that fails
        struct AlwaysFail;
        impl serde::Serialize for AlwaysFail {
            fn serialize<S: serde::Serializer>(&self, _s: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("boom"))
            }
        }
        let toml_err = toml::to_string(&AlwaysFail).unwrap_err();
        let err = Error::from(toml_err);
        assert_eq!(err.source, "toml::ser");
    }

    #[test]
    fn from_toml_de_error_tags_toml_de() {
        #[derive(serde::Deserialize)]
        struct NeedsAField {
            #[allow(dead_code)]
            required: String,
        }
        let toml_err = toml::from_str::<NeedsAField>("missing = \"nope\"")
            .err()
            .unwrap();
        let err = Error::from(toml_err);
        assert_eq!(err.source, "toml::de");
    }

    #[test]
    fn serialize_round_trip() {
        let err = Error::new("io", "permission denied");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains(r#""message":"permission denied""#));
        assert!(json.contains(r#""source":"io""#));
    }

    #[test]
    fn partial_eq() {
        let a = Error::new("x", "y");
        let b = Error::new("x", "y");
        let c = Error::new("x", "z");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn error_trait_impl() {
        let err = Error::new("t", "m");
        // Just prove the trait is usable
        let _: &dyn std::error::Error = &err;
    }
}
