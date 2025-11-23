# Stations configuration reference

This reference explains how to configure station filtering, prioritization, and display using the `StationsConfig` settings.

As with all other configuration for `vacs`, the stations config is stored as a [TOML](https://toml.io/en/) file.  
Various tools exist helping you create and edit TOML files, such as [Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml) for [Visual Studio Code](https://code.visualstudio.com/).
If your tool of choice supports [JSON Schema](https://json-schema.org/), you can find the schema for the `vacs` configuration in this directory ([config.schema.json](config.schema.json)) or as a [GitHub URL](https://raw.githubusercontent.com/MorpheusXAUT/vacs/refs/heads/main/vacs-client/docs/config/config.schema.json).

Whilst in theory, you can add it to the basic `config.toml` or any of the other config files read by `vacs` (they're all merged upon startup), it is recommended to create a separate `stations.toml` file for easier separation and maintenance.

You can place the `stations.toml` either in the default config directory or the installation location of `vacs` (where you launch the client from).

The config directory is dependent on the operating system:

-   Linux: `$XDG_CONFIG_HOME/app.vacs.vacs-client/` or `$HOME/.config/app.vacs.vacs-client/`
-   macOS: `$HOME/Library/Application Support/app.vacs.vacs-client/`
-   Windows: `%APPDATA%\app.vacs.vacs-client\`

## Overview

The `stations` configuration allows you to customize how stations are displayed and filtered in the client. It consists of several sections:

-   **[Profiles](#profiles)** - Define (multiple) filtering configurations that you can switch between in the UI

> [!IMPORTANT]  
> These settings are purely client-side and do not prevent a different user from calling you, even if your filters do not match their callsign and you thus cannot see them.  
> If you are receiving a call from a station you cannot currently see, they will still have their respective callsign shown in the call display, however, you **will not** be able to call them back.

## Configuration structure

```toml
# Default configuration
[stations]

# Profiles for filtering and prioritizing stations
[stations.profiles.Default]
include = []
exclude = []
priority = ["*_FMP", "*_CTR", "*_APP", "*_TWR", "*_GND"]
```

---

## Profiles

Profiles allow you to define multiple filtering configurations and switch between them in the UI. Each profile controls which stations are shown and how they're ordered using three main settings:

-   **`include`** – Allowlist patterns for stations to show
-   **`exclude`** – Blocklist patterns for stations to hide
-   **`priority`** – Ordered patterns that determine display order

### Profile structure

```toml
# Define multiple profiles under [stations.profiles.]
[stations.profiles.Default]
include = []
exclude = []
priority = ["*_FMP", "*_CTR", "*_APP", "*_TWR", "*_GND"]

[stations.profiles.CentersOnly]
include = ["*_CTR"]
exclude = []
priority = ["LOVV_CTR", "EDMM_CTR"]

[stations.profiles.LOVVOnly]
include = ["LO*"]
exclude = ["LON*"]
priority = ["*_FMP", "*_CTR", "*_APP", "*_TWR", "*_GND"]
```

### Profile names

Profile names (the part after `stations.profiles.`) can contain:

-   Letters (a-z, A-Z)
-   Numbers (0-9)
-   Underscores (`_`)
-   Hyphens (`-`)

These names will be displayed in the UI for profile selection.

### Profile settings

Each profile supports the following settings:

#### `include`: selecting which stations to show

**Type:** Array of strings ([glob patterns](#glob-pattern-matching))  
**Default:** `[]` (empty array)  
**Optional:** Yes

Controls which stations are eligible to be displayed.

-   **If empty** (default): All stations are eligible, subject to `exclude` rules
-   **If not empty**: Only stations matching at least one pattern are eligible, all other connected clients are hidden.

**Examples:**

```toml
[stations.profiles.local_area]
# Show only Austrian and Munich stations
include = ["LO*", "EDDM_*", "EDMM_*"]

[stations.profiles.app_ctr]
# Show only approach and center controllers
include = ["*_APP", "*_CTR"]

[stations.profiles.vienna]
# Show everything from Vienna airport
include = ["LOWW_*"]
```

---

#### `exclude`: hiding specific stations

**Type:** Array of strings ([glob patterns](#glob-pattern-matching))  
**Default:** `[]` (empty array)  
**Optional:** Yes

Excludes specific stations from being displayed. Exclude rules always take precedence over `include` rules, allowing you to e.g., include a whole FIR, but exclude all of their ground stations.

**Examples:**

```toml
[stations.profiles.hide_all_ground]
# Hide all ground, tower, and delivery positions
exclude = ["*_TWR", "*_GND", "*_DEL"]

[stations.profiles.hide_airports]
# Hide specific airports
exclude = ["LOWL_*", "LOWG_*"]

[stations.profiles.hide_fmp]
# Hide flow management positions
exclude = ["*_FPM"]
```

---

#### `priority`: ordering stations

**Type:** Ordered array of strings ([glob patterns](#glob-pattern-matching))  
**Default:** `["*_FMP", "*_CTR", "*_APP", "*_TWR", "*_GND"]`  
**Optional:** Yes

Determines the display order of stations. The first matching pattern assigns the station's priority bucket – earlier patterns = higher priority.

Stations are grouped by their priority bucket and then sorted within each bucket. Stations that don't match any priority pattern appear last. After grouping, stations are sorted in alphabetical order (ascending) within their respective buckets.

**Default behavior:**

The default priority list orders stations by controller type:

1. Flow Management Positions (`*_FMP`)
2. Center controllers (`*_CTR`)
3. Approach controllers (`*_APP`)
4. Tower controllers (`*_TWR`)
5. Ground controllers (`*_GND`)

Stations not matched by the `priority` setting are grouped by their station type (alphabetical order, ascending), followed by the remaining stations _without_ a valid type (should only appear on `dev` server).

> [!TIP]  
> If you're trying to completely disable the default behavior, set `priority` to an empty array (`[]`).  
> If you omit the value from your config file, the default will be used.

**Examples:**

```toml
[stations.profiles.local_area]
# Prioritize your local area
priority = [
  "LOVV_*",           # Austrian center first
  "LOWW_*_APP",       # Vienna approach
  "LOWW_*_TWR",       # Vienna tower
  "LOWW_*",           # Other Vienna positions
  "*_CTR",            # Other centers
  "*_APP"             # Other approaches
]

[stations.profiles.centers_first]
# Simple setup: centers, then everything else (grouped by type, alphabetically, ascending)
priority = ["*_CTR"]
```

---

### Glob pattern matching

All patterns use glob-like syntax, which provides flexible matching with wildcards:

#### Wildcards

-   **`*`** – Matches zero or more characters
-   **`?`** – Matches exactly one character

#### Matching rules

-   Matching is **case-insensitive** (`loww` matches `LOWW`)
-   Patterns must match the **entire callsign** (anchored at start and end)
    -   If you want to match a substring in the middle, surround it with wildcards (e.g., `*WW*`)
-   The pattern is converted to a regular expression where:
    -   `*` becomes `.*` (any characters)
    -   `?` becomes `.` (single character)
    -   Other regex special characters are escaped

#### Pattern examples

| Pattern      | Matches                                | Doesn't Match             |
| ------------ | -------------------------------------- | ------------------------- |
| `LOWW_*`     | `LOWW_APP`, `LOWW_TWR`, `LOWW_1_TWR`   | `LOWWAPP`, `LOWI_APP`     |
| `*_APP`      | `LOWW_APP`, `EDDM_APP`, `LOVV_S_APP`   | `LOWW_TWR`, `APP`         |
| `LO*`        | `LOWW_APP`, `LOVV_CTR`, `LO123`        | `EDDM_APP`, `XLO`         |
| `LOWW*_APP`  | `LOWW_APP`, `LOWW_M_APP`, `LOWW_1_APP` | `LOWWAPP`, `LOWI_APP`     |
| `LOWW_?_TWR` | `LOWW_1_TWR`, `LOWW_2_TWR`             | `LOWW_TWR`, `LOWW_12_TWR` |
| `*`          | Everything                             | Nothing                   |
| `LOWW_APP`   | `LOWW_APP` (exact match)               | `LOWW_1_APP`              |

#### Common patterns

```toml
# All stations from a country prefix
include = ["LO*"]        # Austria (LOWW, LOWI, LOVV, etc.)
include = ["ED*"]        # Germany
include = ["LH*"]        # Hungary

# All positions at an airport
include = ["LOWW_*"]     # Vienna
include = ["EDDM_*"]     # Munich

# Specific position types everywhere
include = ["*_CTR"]      # All centers
include = ["*_APP"]      # All approaches
include = ["*_TWR"]      # All towers
include = ["*_GND"]      # All ground
include = ["*_DEL"]      # All delivery

# Numbered positions
include = ["LOWW_?_APP"] # LOWW_1_APP, LOWW_2_APP (single digit)
include = ["LOWW_*_APP"] # LOWW_1_APP, LOWW_12_APP (any number)

# Combined patterns
include = ["LOWW_*_TWR"] # All Vienna towers (but not LOWW_TWR)
include = ["ED*_CTR"]    # All German centers
```

---

### How filtering works

Stations are processed in this order:

1. **Include check**: If `include` is not empty, station must match at least one pattern
2. **Exclude check**: Station must not match any `exclude` pattern
3. **Priority assignment**: First matching `priority` pattern determines display order
4. **Display**: Stations are shown grouped and sorted by priority

#### Example walkthrough

Given this configuration:

```toml
[stations.profiles.example]
include = ["LO*", "EDMM_*", "EDDM_*"]
exclude = ["*_GND", "*_DEL"]
priority = ["LOVV*", "*_CTR", "LO*_APP", "*_APP", "*_TWR"]
```

Station processing:

| Callsign       | Include Match? | Exclude Match? | Priority       | Result                  |
| -------------- | -------------- | -------------- | -------------- | ----------------------- |
| `LOVV_CTR`     | ✓ (`LO*`)      | ✗              | 1 (`LOVV*`)    | **Shown, rank 1**       |
| `LOWW_APP`     | ✓ (`LO*`)      | ✗              | 3 (`*_APP`)    | **Shown, rank 3**       |
| `LOWW_GND`     | ✓ (`LO*`)      | ✓ (`*_GND`)    | –              | Hidden                  |
| `EDMM_ALB_CTR` | ✓ (`*_CTR`)    | ✗              | 2 (`*_CTR`)    | **Shown, rank 2**       |
| `EDDM_TWR`     | ✓ (`EDDM_*`)   | ✗              | 4 (`*_TWR`)    | **Shown, rank 4**       |
| `EDDF_APP`     | ✗              | ✗              | –              | Hidden (not in include) |
| `LON_S_FMP`    | ✓ (`LO*`)      | ✗              | 6 (no pattern) | **Shown, rank 5**       |

### Complete Examples

#### Example 1: Multiple workflow profiles

Create different profiles for different scenarios:

```toml
[stations.profiles.FIR_Wien]
# Only show stations from FIR Wien
include = ["LO*"]
exclude = ["LON*"]
priority = ["*_FMP", "*_CTR", "LOWW*_APP", "*_APP", "LOWW*_TWR", "*_TWR", "*_GND"]

[stations.profiles.CTR_only]
# Show only center controllers
include = ["*_CTR"]
exclude = ["*_FSS"]
priority = ["LOVV*", "EDMM*"]

[stations.profiles.No_Training]
# Hide common training positions
include = []
exclude = ["*_M_*", "*_X_*", "*_OBS"]
priority = ["*_FMP", "*_CTR", "*_APP", "*_TWR", "*_GND"]
```

#### Example 2: FIR-specific profiles

Create profiles for different FIRs you control in:

```toml
[stations.profiles.LOVV]
include = ["LO*"]
exclude = ["LON*"]
priority = ["LOVV*", "LOWW*"]

[stations.profiles.EDMM]
include = ["EDMM*", "EDDM*"]
exclude = []
priority = ["EDMM*", "EDDM*"]
```

#### Example 3: Role-based profiles

Create profiles based on your controlling position:

```toml
[stations.profiles.TWR]
# When controlling tower, show relevant positions
include = ["LO*"]
exclude = ["LON*"]
priority = ["LOWW*_APP", "LOWW*_TWR", "LOWW*_GND", "LOWW*_DEL"]

[stations.profiles.CTR]
# When controlling center, focus on adjacent centers and approach
include = ["*_CTR", "*_APP"]
exclude = []
priority = ["LOVV*_CTR", "EDMM*_CTR", "*_CTR", "LOWW*_APP", "EDDM*_APP", "*_APP"]
```

---

### Tips

-   Create multiple profiles for different workflows (e.g., "default", "CTR", "APP")
-   Use descriptive profile names that indicate their purpose
-   Start with simple patterns and add complexity as needed
-   Use `exclude` to refine broad `include` patterns
-   Put your most important stations at the top of `priority`
-   Leave `include` empty to see everything (filtered only by `exclude`)
-   Remember that `exclude` always wins over `include`
-   You can switch between profiles in the UI without restarting the application
