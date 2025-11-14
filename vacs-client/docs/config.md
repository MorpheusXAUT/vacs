# vacs-client configuration reference

The vacs client reads configuration from:

1. Built-in defaults,
2. `config.toml` in the config directory,
3. `config.toml` in the current working directory,
4. `audio.toml` in the config directory,
5. `client.toml` in the config directory,
6. `audio.toml` in the current working directory,
7. Environment variables with the `VACS_CLIENT_` prefix.

The config directory is dependent on the operating system:
- Linux: `$XDG_CONFIG_HOME/app.vacs.vacs-client/` or `$HOME/.config/app.vacs.vacs-client/`
- macOS: `$HOME/Library/Application Support/app.vacs.vacs-client/`
- Windows: `%APPDATA%\app.vacs.vacs-client\`

Later sources override earlier ones. Whilst all config files _can_ contain any kind of configuration value,
vacs only persists a certain subset of configuration depending on the file read/written.

All configuration files use the [TOML](https://toml.io/en/) format.

---

## Top-level structure

```toml
[backend]
# BackendConfig

[audio]
# AudioConfig

[webrtc]
# WebRTCConfig

[client]
# ClientConfig
```

##