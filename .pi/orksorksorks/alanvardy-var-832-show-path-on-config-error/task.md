# Task

When the orksorksorks CLI cannot find or read its configuration file, the
error the user sees is a bare std io error string that does not say which
path the program actually tried (or how it resolved that path). This makes
misconfiguration (wrong $XDG_CONFIG_HOME, wrong --config location) hard to
diagnose.

Build: make the config-not-found failure surface the resolved config file
path (and ideally how it was resolved) in the error output, for both text
and JSON output modes. The resolved path is already computed upstream;
it just never reaches the error.