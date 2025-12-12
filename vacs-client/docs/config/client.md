# Client Configuration

This reference explains how to control various aspects of the client's behavior, such as window settings, release channels, and ignore lists using the `ClientConfig` settings, read from the (optional) `client.toml` config file.

For general information on the configuration file format, file locations, and recommended editors, please refer to the [main configuration reference](README.md).

## Overview

The `client` configuration allows you to control the client's behavior and local logic. It consists of the following sections:

- **[Ignore list](#ignore-list)** - Manage ignored users
- **[Extra stations config](#extra-stations-config)** - Load an additional stations config file
- **[Selected stations profile](#selected-stations-profile)** - Currently active stations profile

## Configuration structure

```toml
[client]
ignored = []
selected_stations_profile = "Default"
# extra_stations_config = "/path/to/extra_stations.toml"
```

---

## Ignore list

The `ignored` list allows you to completely ignore specific users client-side (identified by their VATSIM CID).

**Type:** Array of strings (CIDs)  
**Default:** `[]` (empty list)  
**Optional:** Yes

Any incoming calls initiated by a CID in this list will be silently ignored by the client. Their call attempts will also not show up in your call history, however to an ignored user, it will still look like you are online and simply not answering their calls.

**This is not a block feature:** You can still initiate calls to users in your ignore list. The setting only suppresses _incoming_ interactions.

> [!NOTE]  
> This is a global setting and independent from your currently selected [stations profile](stations.md#profiles).

You can change this list manually in the configuration file before startup, or by going to the `Telephone` page in the client and modifying the list of ignored users in the `Ign.` tab. Alternatively, you can select a call from the `Call List` and ignore the caller using the `Ignore CID` button.

Note that all changes made to the config file only apply after `vacs` has been restarted and might be overwritten if you change any other settings via the UI.

**Example:**

```toml
[client]
# Ignore calls from these CIDs
ignored = ["10000003", "1234567"]
```

---

## Extra stations config

The `extra_stations_config` setting allows you to load an additional stations configuration file. This is useful for including the stations profiles provided by your NAV team in your FIR's sector file.

**Type:** String (Path)  
**Default:** None  
**Optional:** Yes

The path can be absolute or relative to the configuration directory. You can pick an extra stations config file to apply via the `Mission` page in the client.

> [!WARNING]  
> Under Windows, all backslashes in the path must be escaped with another backslash (e.g., `C:\\Users\\user\\Documents\\EuroScope\\SectorFile\\stations.toml`).

**Example:**

```toml
[client]
extra_stations_config = "/home/user/Documents/EuroScope/SectorFile/stations.toml"
```

> [!NOTE]  
> The extra stations config is merged with the main stations config. If there are any conflicts, the extra stations config takes precedence.

---

## Selected stations profile

The `selected_stations_profile` setting determines which [stations profile](stations.md#profiles) is currently active.

**Type:** String  
**Default:** `"Default"`  
**Optional:** Yes

This value is updated automatically when you switch profiles in the client UI.
